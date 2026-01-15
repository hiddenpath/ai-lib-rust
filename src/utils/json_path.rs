//! JSONPath evaluator and path mapper for frame selection and field extraction
//!
//! Inspired by ai-lib's PathMapper implementation, with support for:
//! - Nested path access (e.g., "a.b.c")
//! - Array indexing (e.g., "choices[0].delta.content")
//! - Condition evaluation (exists, ==, !=, in, &&, ||, >, <, >=, <=)
//! - Regular expression matching

use serde_json::{json, Value};
use std::collections::HashMap;

/// Path mapper error
#[derive(Debug, thiserror::Error)]
pub enum PathMapperError {
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Cannot set value at path: {0}")]
    CannotSetValue(String),
}

/// Path mapper for extracting and setting values in JSON using dot-notation paths
pub struct PathMapper;

impl PathMapper {
    /// Get value from JSON using dot-notation path (supports array indexing)
    ///
    /// Examples:
    /// - "choices[0].delta.content"
    /// - "input.temperature"
    /// - "delta.text"
    pub fn get_path<'a>(obj: &'a Value, path: &str) -> Option<&'a Value> {
        if path.is_empty() {
            return None;
        }

        // Remove leading "$." if present (JSONPath style)
        let normalized = path.trim().trim_start_matches("$.").to_string();
        let parts: Vec<&str> = normalized.split('.').collect();
        let mut current = obj;

        for part in parts {
            if part.is_empty() {
                return None;
            }

            // Check if part contains array index, e.g., "choices[0]"
            if let Some(bracket_pos) = part.find('[') {
                // Extract key and index
                let key = &part[..bracket_pos];
                let idx_str = part[bracket_pos + 1..].trim_end_matches(']');

                // First access the object key
                if !key.is_empty() {
                    match current {
                        Value::Object(map) => {
                            current = map.get(key)?;
                        }
                        _ => return None,
                    }
                }

                // Then access the array index
                if let Ok(idx) = idx_str.parse::<usize>() {
                    match current {
                        Value::Array(arr) => {
                            current = arr.get(idx)?;
                        }
                        _ => return None,
                    }
                } else if idx_str == "*" {
                    // Wildcard: get first element
                    match current {
                        Value::Array(arr) => {
                            current = arr.first()?;
                        }
                        _ => return None,
                    }
                } else {
                    return None;
                }
            } else {
                // Simple key access OR dot-index access (e.g. "choices.0.delta")
                match current {
                    Value::Object(map) => {
                        current = map.get(part)?;
                    }
                    Value::Array(arr) => {
                        // Support "0" / "1" style index segments (common in some JSONPath variants)
                        if let Ok(idx) = part.parse::<usize>() {
                            current = arr.get(idx)?;
                        } else if part == "*" {
                            current = arr.first()?;
                        } else {
                            return None;
                        }
                    }
                    _ => return None,
                }
            }
        }

        Some(current)
    }

    /// Get string value from path (converts number to string if needed)
    pub fn get_string(obj: &Value, path: &str) -> Option<String> {
        Self::get_path(obj, path).and_then(|v| {
            if v.is_string() {
                v.as_str().map(|s| s.to_string())
            } else {
                serde_json::to_string(v).ok()
            }
        })
    }

    /// Set value at nested path in JSON object
    ///
    /// Examples:
    /// - "input.temperature" -> sets obj["input"]["temperature"]
    /// - "generationConfig.maxOutputTokens" -> sets obj["generationConfig"]["maxOutputTokens"]
    pub fn set_path(obj: &mut Value, path: &str, value: Value) -> Result<(), PathMapperError> {
        if path.is_empty() {
            return Err(PathMapperError::InvalidPath("Empty path".to_string()));
        }

        // Remove leading "$." if present
        let normalized = path.trim().trim_start_matches("$.").to_string();
        let parts: Vec<&str> = normalized.split('.').collect();

        if parts.is_empty() {
            return Err(PathMapperError::InvalidPath("Empty path parts".to_string()));
        }

        // Ensure root object is Object
        if !obj.is_object() {
            *obj = json!({});
        }

        let mut current = obj
            .as_object_mut()
            .ok_or_else(|| PathMapperError::CannotSetValue("Root is not an object".to_string()))?;

        // Process all but the last path segment
        for (idx, part) in parts.iter().enumerate().take(parts.len() - 1) {
            if part.is_empty() {
                return Err(PathMapperError::InvalidPath(format!(
                    "Empty path part at index {}",
                    idx
                )));
            }

            // If path doesn't exist or is not an object, create new object
            if !current.contains_key(*part) || !current[*part].is_object() {
                current.insert(part.to_string(), json!({}));
            }

            // Move to next level
            current = current[*part].as_object_mut().ok_or_else(|| {
                PathMapperError::CannotSetValue(format!("Cannot access object at path: {}", part))
            })?;
        }

        // Set the last path segment's value
        let last_part = parts
            .last()
            .ok_or_else(|| PathMapperError::InvalidPath("No last part".to_string()))?;

        if last_part.is_empty() {
            return Err(PathMapperError::InvalidPath(
                "Last path part is empty".to_string(),
            ));
        }

        current.insert(last_part.to_string(), value);
        Ok(())
    }

    /// Batch set multiple paths
    pub fn set_paths(
        obj: &mut Value,
        paths: &HashMap<String, Value>,
    ) -> Result<(), PathMapperError> {
        for (path, value) in paths {
            Self::set_path(obj, path, value.clone())?;
        }
        Ok(())
    }
}

/// JSONPath evaluator for condition matching
/// Supports: exists, ==, !=, in, &&, ||, >, <, >=, <=, regex
#[derive(Clone)]
pub struct JsonPathEvaluator {
    expression: String,
}

impl JsonPathEvaluator {
    pub fn new(expression: &str) -> Result<Self, String> {
        if expression.is_empty() {
            return Err("Empty expression".to_string());
        }
        Ok(Self {
            expression: expression.to_string(),
        })
    }

    /// Check if expression matches the JSON value
    ///
    /// Supports:
    /// - exists($.path) - check if path exists
    /// - $.path == "value" - equality check
    /// - $.path != "value" - inequality check
    /// - $.path in ['value1', 'value2'] - list membership
    /// - $.path != null / $.path == null - null check
    /// - $.path > 10 / $.path < 10 - numeric comparison
    /// - $.path >= 10 / $.path <= 10 - numeric comparison
    /// - $.path =~ /pattern/ - regex matching
    /// - && and || for logical combination
    pub fn matches(&self, value: &Value) -> bool {
        Self::evaluate_match(&self.expression, value)
    }

    /// Extract string value from JSON using path
    pub fn extract_string(&self, value: &Value) -> Option<String> {
        // If expression is a simple path, extract it
        if self.expression.starts_with("$.") {
            return PathMapper::get_string(value, &self.expression);
        }
        None
    }

    /// Evaluate match expression with support for numeric comparisons and regex
    fn evaluate_match(expr: &str, root: &Value) -> bool {
        // Split by OR
        let or_parts: Vec<&str> = expr.split("||").collect();
        for or_part in or_parts {
            let mut ok = true;
            // Split by AND
            let and_parts: Vec<&str> = or_part.split("&&").collect();
            for part in and_parts {
                let cond = part.trim();
                if cond.is_empty() {
                    continue;
                }

                // exists() check
                if cond.starts_with("exists(") && cond.ends_with(')') {
                    let path = cond.trim_start_matches("exists(").trim_end_matches(')');
                    if PathMapper::get_path(root, path).is_none() {
                        ok = false;
                        break;
                    }
                    continue;
                }

                // Regex matching: $.path =~ /pattern/
                if let Some(idx) = cond.find("=~") {
                    let (path, rest) = cond.split_at(idx);
                    let path = path.trim();
                    let pattern_str = rest.trim_start_matches("=~").trim();

                    // Extract pattern from /pattern/ or "pattern"
                    let pattern = pattern_str
                        .trim_start_matches('/')
                        .trim_end_matches('/')
                        .trim_matches('"')
                        .trim_matches('\'');

                    if let Some(actual) = PathMapper::get_string(root, path) {
                        // Simple regex matching (for full implementation, use regex crate)
                        // For now, support basic wildcard patterns
                        if !Self::simple_regex_match(&actual, pattern) {
                            ok = false;
                            break;
                        }
                    } else {
                        ok = false;
                        break;
                    }
                    continue;
                }

                // Numeric comparisons: >, <, >=, <=
                for op in &[">=", "<=", ">", "<"] {
                    if let Some(idx) = cond.find(op) {
                        let (path, rest) = cond.split_at(idx);
                        let path = path.trim();
                        let target_str = rest
                            .trim_start_matches(op)
                            .trim()
                            .trim_matches('"')
                            .trim_matches('\'');

                        if let Ok(target_num) = target_str.parse::<f64>() {
                            if let Some(actual_val) = PathMapper::get_path(root, path) {
                                let actual_num = actual_val.as_f64().or_else(|| {
                                    actual_val.as_str().and_then(|s| s.parse::<f64>().ok())
                                });

                                if let Some(actual) = actual_num {
                                    let matches = match *op {
                                        ">" => actual > target_num,
                                        "<" => actual < target_num,
                                        ">=" => actual >= target_num,
                                        "<=" => actual <= target_num,
                                        _ => false,
                                    };
                                    if !matches {
                                        ok = false;
                                        break;
                                    }
                                    continue;
                                }
                            }
                        }
                        ok = false;
                        break;
                    }
                }

                // "in" list check
                if let Some(idx) = cond.find(" in ") {
                    let (path, rest) = cond.split_at(idx);
                    let path = path.trim();
                    let list_str = rest.trim_start_matches(" in ").trim();
                    let list_str = list_str.trim_start_matches('[').trim_end_matches(']');
                    let values: Vec<String> = list_str
                        .split(',')
                        .filter_map(|v| v.trim().trim_matches('\'').trim_matches('"').parse().ok())
                        .collect();
                    let actual = PathMapper::get_string(root, path);
                    if !actual.map(|a| values.contains(&a)).unwrap_or(false) {
                        ok = false;
                        break;
                    }
                    continue;
                }

                // "!= null" check
                if let Some(idx) = cond.find("!= null") {
                    let path = cond[..idx].trim();
                    let val = PathMapper::get_path(root, path);
                    if val.is_none() || val == Some(&Value::Null) {
                        ok = false;
                        break;
                    }
                    continue;
                }

                // "== null" check
                if let Some(idx) = cond.find("== null") {
                    let path = cond[..idx].trim();
                    let val = PathMapper::get_path(root, path);
                    if val.is_some() && val != Some(&Value::Null) {
                        ok = false;
                        break;
                    }
                    continue;
                }

                // "==" equality check
                if let Some(idx) = cond.find("==") {
                    let (path, value_part) = cond.split_at(idx);
                    let path = path.trim();
                    let target = value_part
                        .trim_start_matches("==")
                        .trim()
                        .trim_matches('\'')
                        .trim_matches('"');
                    let actual = PathMapper::get_string(root, path);
                    if actual.as_deref() != Some(target) {
                        ok = false;
                        break;
                    }
                    continue;
                }

                // "!=" inequality check
                if let Some(idx) = cond.find("!=") {
                    let (path, value_part) = cond.split_at(idx);
                    let path = path.trim();
                    let target = value_part
                        .trim_start_matches("!=")
                        .trim()
                        .trim_matches('\'')
                        .trim_matches('"');
                    let actual = PathMapper::get_string(root, path);
                    if actual.as_deref() == Some(target) {
                        ok = false;
                        break;
                    }
                    continue;
                }
            }
            if ok {
                return true;
            }
        }
        false
    }

    /// Simple regex matching (supports basic wildcards)
    /// For full regex support, use the `regex` crate
    fn simple_regex_match(text: &str, pattern: &str) -> bool {
        // Simple wildcard matching: * matches any sequence, ? matches any character
        if pattern.contains('*') || pattern.contains('?') {
            // Basic wildcard implementation
            let mut text_chars = text.chars();
            let mut pattern_chars = pattern.chars();

            while let Some(p) = pattern_chars.next() {
                match p {
                    '*' => {
                        // Match zero or more characters
                        if let Some(next_p) = pattern_chars.next() {
                            // Find next character in pattern
                            while let Some(t) = text_chars.next() {
                                if t == next_p {
                                    break;
                                }
                            }
                        } else {
                            // * at end matches rest
                            return true;
                        }
                    }
                    '?' => {
                        // Match any single character
                        if text_chars.next().is_none() {
                            return false;
                        }
                    }
                    c => {
                        if text_chars.next() != Some(c) {
                            return false;
                        }
                    }
                }
            }
            text_chars.next().is_none()
        } else {
            // Simple substring match
            text.contains(pattern)
        }
    }
}
