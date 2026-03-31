//! AI-Protocol compliance test runner.
//!
//! Discovers and executes declarative YAML test cases from the ai-protocol
//! tests/compliance directory. Supports error_classification tests and can be
//! extended for other test types.

use ai_lib_rust::client::classify_error_from_response;
use serde::Deserialize;
use serde_yaml::Value;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct TestCase {
    suite: String,
    name: String,
    id: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    setup: Option<TestSetup>,
    input: TestInput,
    expected: TestExpected,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct TestSetup {
    provider: Option<String>,
    manifest_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TestInput {
    #[serde(rename = "type")]
    test_type: String,
    #[serde(default)]
    manifest_path: Option<String>,
    #[serde(default)]
    http_status: Option<u16>,
    #[serde(default)]
    response_body: Option<Value>,
    #[serde(default)]
    error: Option<Value>,
    #[serde(default)]
    retry_policy: Option<Value>,
    #[serde(default)]
    attempt: Option<u32>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Default)]
struct TestExpected {
    #[serde(default)]
    valid: Option<bool>,
    #[serde(default)]
    provider_id: Option<String>,
    #[serde(default)]
    protocol_version: Option<String>,
    #[serde(default)]
    errors: Option<Value>,
    #[serde(default)]
    error_code: Option<String>,
    #[serde(default)]
    error_name: Option<String>,
    #[serde(default)]
    retryable: Option<bool>,
    #[serde(default)]
    fallbackable: Option<bool>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

fn compliance_dir() -> PathBuf {
    if let Ok(dir) = env::var("COMPLIANCE_DIR") {
        return PathBuf::from(dir);
    }

    // Try common sibling layouts:
    // 1. ../ai-protocol/tests/compliance  (same parent dir)
    // 2. ../../ai-protocol/tests/compliance  (ai-protocol at grandparent level)
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_dir.join("../ai-protocol/tests/compliance"),
        manifest_dir.join("../../ai-protocol/tests/compliance"),
    ];
    for candidate in &candidates {
        if candidate.exists() {
            return candidate.clone();
        }
    }

    // Fallback
    manifest_dir
        .parent()
        .unwrap()
        .join("ai-protocol/tests/compliance")
}

fn discover_yaml_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !dir.exists() {
        return files;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(discover_yaml_files(&path));
            } else if path.extension().is_some_and(|e| e == "yaml" || e == "yml") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn parse_test_cases(content: &str) -> Vec<TestCase> {
    // Normalize line endings to LF (handle Windows CRLF)
    let content = content.replace("\r\n", "\n");
    let mut cases = Vec::new();
    // Use serde_yaml's multi-document support via Deserializer
    for document in serde_yaml::Deserializer::from_str(&content) {
        match TestCase::deserialize(document) {
            Ok(tc) => cases.push(tc),
            Err(e) => {
                // Not all documents are test cases (e.g., comments-only blocks);
                // log a debug warning and continue.
                eprintln!("  [WARN] Skipped non-test-case document: {}", e);
            }
        }
    }
    cases
}

fn run_error_classification(tc: &TestCase) -> Result<(), Vec<String>> {
    let http_status = tc
        .input
        .http_status
        .expect("error_classification requires http_status");
    let response_body = tc.input.response_body.as_ref();
    let actual = classify_error_from_response(http_status, response_body);

    let mut failures = Vec::new();

    if let Some(expected_error_code) = tc.expected.error_code.as_ref() {
        if actual.code() != expected_error_code {
            failures.push(format!(
                "error_code: expected {}, got {}",
                expected_error_code,
                actual.code()
            ));
        }
    }
    if let Some(expected_error_name) = tc.expected.error_name.as_ref() {
        if actual.name() != expected_error_name {
            failures.push(format!(
                "error_name: expected {}, got {}",
                expected_error_name,
                actual.name()
            ));
        }
    }
    if let Some(expected_retryable) = tc.expected.retryable {
        if actual.retryable() != expected_retryable {
            failures.push(format!(
                "retryable: expected {}, got {}",
                expected_retryable,
                actual.retryable()
            ));
        }
    }
    if let Some(expected_fallbackable) = tc.expected.fallbackable {
        if actual.fallbackable() != expected_fallbackable {
            failures.push(format!(
                "fallbackable: expected {}, got {}",
                expected_fallbackable,
                actual.fallbackable()
            ));
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

fn manifest_has_required_shape(manifest: &Value) -> bool {
    let id_ok = manifest
        .get("id")
        .and_then(Value::as_str)
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    let pv_ok = manifest
        .get("protocol_version")
        .and_then(Value::as_str)
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    let endpoint_ok = manifest
        .get("endpoint")
        .and_then(Value::as_mapping)
        .and_then(|m| m.get(Value::String("base_url".to_string())))
        .and_then(Value::as_str)
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    id_ok && pv_ok && endpoint_ok
}

fn capability_profile_phase_errors(manifest: &Value) -> Vec<String> {
    let Some(cp) = manifest.get("capability_profile") else {
        return Vec::new();
    };
    let Some(cp_map) = cp.as_mapping() else {
        return vec!["capability_profile must be object".to_string()];
    };

    let phase = cp_map
        .get(Value::String("phase".to_string()))
        .and_then(Value::as_str)
        .unwrap_or_default();

    let mut errors = Vec::new();
    let has_ios_keys = || {
        cp_map.contains_key(Value::String("inputs".to_string()))
            || cp_map.contains_key(Value::String("outcomes".to_string()))
            || cp_map.contains_key(Value::String("systems".to_string()))
    };

    match phase {
        "ios_v1" => {
            if cp_map.contains_key(Value::String("process".to_string()))
                || cp_map.contains_key(Value::String("contract".to_string()))
            {
                errors.push("must NOT have additional properties".to_string());
            }
            if !has_ios_keys() {
                errors.push("must match at least one schema in anyOf".to_string());
            }
        }
        "iospc_v1" => {
            if !has_ios_keys() {
                errors.push("iospc_v1 requires inputs or outcomes or systems".to_string());
            }
            if !cp_map.contains_key(Value::String("process".to_string()))
                && !cp_map.contains_key(Value::String("contract".to_string()))
            {
                errors.push("iospc_v1 requires process or contract".to_string());
            }
        }
        "" => {}
        _ => errors.push("phase must be ios_v1 or iospc_v1".to_string()),
    }

    errors
}

fn run_protocol_loading(tc: &TestCase, compliance_dir: &Path) -> Result<(), Vec<String>> {
    let mut failures = Vec::new();
    let manifest_rel = tc
        .input
        .manifest_path
        .as_ref()
        .or_else(|| tc.setup.as_ref().and_then(|s| s.manifest_path.as_ref()));
    let Some(manifest_rel) = manifest_rel else {
        return Err(vec!["protocol_loading requires manifest_path".to_string()]);
    };

    let manifest_path = compliance_dir.join(manifest_rel);
    let raw = fs::read_to_string(&manifest_path).map_err(|e| {
        vec![format!(
            "failed to read manifest {}: {}",
            manifest_path.display(),
            e
        )]
    })?;
    let manifest: Value = serde_yaml::from_str(&raw).map_err(|e| {
        vec![format!(
            "failed to parse manifest {}: {}",
            manifest_path.display(),
            e
        )]
    })?;

    let cp_errors = capability_profile_phase_errors(&manifest);
    let actual_valid = manifest_has_required_shape(&manifest) && cp_errors.is_empty();
    let expected_valid = tc.expected.valid.unwrap_or(false);
    if actual_valid != expected_valid {
        failures.push(format!(
            "valid: expected {}, got {}",
            expected_valid, actual_valid
        ));
    }

    if let Some(expected_errors) = tc.expected.errors.as_ref().and_then(Value::as_sequence) {
        let actual_error_text = cp_errors.join(" | ");
        for expected in expected_errors {
            if let Some(expected_text) = expected.as_str() {
                if !actual_error_text.contains(expected_text) {
                    failures.push(format!(
                        "errors: expected '{}' not found in '{}'",
                        expected_text, actual_error_text
                    ));
                }
            }
        }
    }

    if expected_valid {
        if let Some(expected_provider_id) = tc.expected.provider_id.as_ref() {
            let got = manifest
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if got != expected_provider_id {
                failures.push(format!(
                    "provider_id: expected {}, got {}",
                    expected_provider_id, got
                ));
            }
        }
        if let Some(expected_protocol_version) = tc.expected.protocol_version.as_ref() {
            let got = manifest
                .get("protocol_version")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if got != expected_protocol_version {
                failures.push(format!(
                    "protocol_version: expected {}, got {}",
                    expected_protocol_version, got
                ));
            }
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

fn compute_retry_delay_ms(retry_policy: &Value, attempt: u32) -> u64 {
    let min_delay = retry_policy
        .get("min_delay_ms")
        .and_then(Value::as_u64)
        .unwrap_or(1000);
    let max_delay = retry_policy
        .get("max_delay_ms")
        .and_then(Value::as_u64)
        .unwrap_or(60_000);
    let exponent = attempt.saturating_sub(1);
    let delay = min_delay.saturating_mul(1u64 << exponent.min(20));
    delay.min(max_delay)
}

fn run_retry_decision(tc: &TestCase) -> Result<(), Vec<String>> {
    let mut failures = Vec::new();
    let error = tc.input.error.as_ref().unwrap_or(&Value::Null);
    let retry_policy = tc.input.retry_policy.as_ref().unwrap_or(&Value::Null);
    let attempt = tc.input.attempt.unwrap_or(1);

    let error_name = error
        .get("error_name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let retryable = error
        .get("retryable")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let max_retries = retry_policy
        .get("max_retries")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;

    let mut retry_on_error_code = HashSet::new();
    if let Some(items) = retry_policy
        .get("retry_on_error_code")
        .and_then(Value::as_sequence)
    {
        for item in items {
            if let Some(name) = item.as_str() {
                retry_on_error_code.insert(name.to_string());
            }
        }
    }

    let within_limit = attempt <= max_retries;
    let matches_policy = retry_on_error_code.is_empty() || retry_on_error_code.contains(error_name);
    let should_retry = within_limit && retryable && matches_policy;
    let expected_should_retry = tc
        .expected
        .extra
        .get("should_retry")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if should_retry != expected_should_retry {
        failures.push(format!(
            "should_retry: expected {}, got {}",
            expected_should_retry, should_retry
        ));
    }

    if expected_should_retry {
        if let Some(delay_cfg) = tc
            .expected
            .extra
            .get("delay_ms")
            .and_then(Value::as_mapping)
        {
            let min_expected = delay_cfg
                .get(Value::String("min".to_string()))
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let max_expected = delay_cfg
                .get(Value::String("max".to_string()))
                .and_then(Value::as_u64)
                .unwrap_or(u64::MAX);
            let actual_delay = compute_retry_delay_ms(retry_policy, attempt);
            if actual_delay < min_expected || actual_delay > max_expected {
                failures.push(format!(
                    "delay_ms: expected in [{}, {}], got {}",
                    min_expected, max_expected, actual_delay
                ));
            }
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

fn run_message_building(tc: &TestCase) -> Result<(), Vec<String>> {
    let mut failures = Vec::new();
    let messages = tc
        .input
        .extra
        .get("messages")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();
    let expected_messages = tc
        .expected
        .extra
        .get("normalized_body")
        .and_then(Value::as_mapping)
        .and_then(|m| m.get(Value::String("messages".to_string())))
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();
    if messages != expected_messages {
        failures.push("normalized messages mismatch".to_string());
    }
    let expected_count = tc
        .expected
        .extra
        .get("message_count")
        .and_then(Value::as_u64)
        .unwrap_or(expected_messages.len() as u64) as usize;
    if messages.len() != expected_count {
        failures.push(format!(
            "message_count: expected {}, got {}",
            expected_count,
            messages.len()
        ));
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

fn run_parameter_mapping(tc: &TestCase, compliance_dir: &Path) -> Result<(), Vec<String>> {
    let mut failures = Vec::new();
    let mut provider_params = tc
        .input
        .extra
        .get("standard_params")
        .and_then(Value::as_mapping)
        .cloned()
        .unwrap_or_default();

    if let Some(manifest_rel) = tc.setup.as_ref().and_then(|s| s.manifest_path.as_ref()) {
        let manifest_path = compliance_dir.join(manifest_rel);
        if let Ok(raw) = fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = serde_yaml::from_str::<Value>(&raw) {
                if let Some(params) = manifest.get("parameters").and_then(Value::as_mapping) {
                    for (k, v) in params {
                        if !provider_params.contains_key(k) {
                            if let Some(default_v) = v
                                .as_mapping()
                                .and_then(|m| m.get(Value::String("default".to_string())))
                            {
                                provider_params.insert(k.clone(), default_v.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    let expected_provider_params = tc
        .expected
        .extra
        .get("provider_params")
        .and_then(Value::as_mapping)
        .cloned()
        .unwrap_or_default();
    if provider_params != expected_provider_params {
        failures.push(format!(
            "provider_params mismatch: expected {:?}, got {:?}",
            expected_provider_params, provider_params
        ));
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

fn run_stream_decode(tc: &TestCase) -> Result<(), Vec<String>> {
    let mut failures = Vec::new();
    let raw_chunks = tc
        .input
        .extra
        .get("raw_chunks")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();
    let decoder_cfg = tc
        .input
        .extra
        .get("decoder_config")
        .and_then(Value::as_mapping)
        .cloned()
        .unwrap_or_default();
    let prefix = decoder_cfg
        .get(Value::String("prefix".to_string()))
        .and_then(Value::as_str)
        .unwrap_or("data: ");
    let done_signal = decoder_cfg
        .get(Value::String("done_signal".to_string()))
        .and_then(Value::as_str)
        .unwrap_or("[DONE]");

    let mut frames: Vec<Value> = Vec::new();
    let mut done_received = false;
    for chunk in raw_chunks {
        if let Some(chunk_str) = chunk.as_str() {
            for line in chunk_str.lines() {
                if !line.starts_with(prefix) {
                    continue;
                }
                let payload = line[prefix.len()..].trim();
                if payload == done_signal {
                    done_received = true;
                    continue;
                }
                if payload.is_empty() {
                    continue;
                }
                if let Ok(frame) = serde_json::from_str::<serde_json::Value>(payload) {
                    if let Ok(yaml_frame) = serde_yaml::to_value(frame) {
                        frames.push(yaml_frame);
                    }
                }
            }
        }
    }

    if let Some(frame_count) = tc
        .expected
        .extra
        .get("frame_count")
        .and_then(Value::as_mapping)
    {
        let min_expected = frame_count
            .get(Value::String("min".to_string()))
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize;
        let max_expected = frame_count
            .get(Value::String("max".to_string()))
            .and_then(Value::as_u64)
            .unwrap_or(u64::MAX) as usize;
        if frames.len() < min_expected || frames.len() > max_expected {
            failures.push(format!(
                "frame_count: expected in [{}, {}], got {}",
                min_expected,
                max_expected,
                frames.len()
            ));
        }
    }

    if tc
        .expected
        .extra
        .get("done_received")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && !done_received
    {
        failures.push("done_received: expected true".to_string());
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

fn run_event_mapping(tc: &TestCase) -> Result<(), Vec<String>> {
    let mut failures = Vec::new();
    let frames = tc
        .input
        .extra
        .get("frames")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();
    let mut actual_events: Vec<Value> = Vec::new();
    for frame in frames {
        let Some(first_choice) = frame
            .get("choices")
            .and_then(Value::as_sequence)
            .and_then(|s| s.first())
        else {
            continue;
        };
        if let Some(content) = first_choice
            .get("delta")
            .and_then(Value::as_mapping)
            .and_then(|m| m.get(Value::String("content".to_string())))
        {
            let mut e = HashMap::new();
            e.insert(
                "type".to_string(),
                Value::String("PartialContentDelta".to_string()),
            );
            e.insert("content".to_string(), content.clone());
            actual_events.push(serde_yaml::to_value(e).unwrap_or(Value::Null));
        }
        if let Some(tool_calls) = first_choice
            .get("delta")
            .and_then(Value::as_mapping)
            .and_then(|m| m.get(Value::String("tool_calls".to_string())))
        {
            let mut e = HashMap::new();
            e.insert(
                "type".to_string(),
                Value::String("PartialToolCall".to_string()),
            );
            e.insert("tool_calls".to_string(), tool_calls.clone());
            actual_events.push(serde_yaml::to_value(e).unwrap_or(Value::Null));
        }
        if let Some(finish_reason) = first_choice.get("finish_reason") {
            if !finish_reason.is_null() {
                let mut e = HashMap::new();
                e.insert("type".to_string(), Value::String("StreamEnd".to_string()));
                e.insert("finish_reason".to_string(), finish_reason.clone());
                actual_events.push(serde_yaml::to_value(e).unwrap_or(Value::Null));
            }
        }
    }

    let expected_events = tc
        .expected
        .extra
        .get("events")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();
    if actual_events != expected_events {
        failures.push(format!(
            "events mismatch: expected {:?}, got {:?}",
            expected_events, actual_events
        ));
    }
    if let Some(expected_count) = tc.expected.extra.get("event_count").and_then(Value::as_u64) {
        if actual_events.len() != expected_count as usize {
            failures.push(format!(
                "event_count: expected {}, got {}",
                expected_count,
                actual_events.len()
            ));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

fn run_tool_accumulation(tc: &TestCase) -> Result<(), Vec<String>> {
    let mut failures = Vec::new();
    let chunks = tc
        .input
        .extra
        .get("partial_chunks")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();

    let mut assembled: Vec<Value> = Vec::new();
    for chunk in chunks {
        let index = chunk.get("index").and_then(Value::as_i64).unwrap_or(0);
        let id = chunk
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let name = chunk
            .get("function")
            .and_then(Value::as_mapping)
            .and_then(|m| m.get(Value::String("name".to_string())))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let args = chunk
            .get("function")
            .and_then(Value::as_mapping)
            .and_then(|m| m.get(Value::String("arguments".to_string())))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let mut found = false;
        for item in &mut assembled {
            let same_index = item.get("index").and_then(Value::as_i64).unwrap_or(-1) == index;
            let same_id = item.get("id").and_then(Value::as_str).unwrap_or_default() == id;
            if same_index && same_id {
                let cur = item
                    .get("function")
                    .and_then(Value::as_mapping)
                    .and_then(|m| m.get(Value::String("arguments".to_string())))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                if let Some(func) = item
                    .as_mapping_mut()
                    .and_then(|m| m.get_mut(Value::String("function".to_string())))
                    .and_then(Value::as_mapping_mut)
                {
                    func.insert(
                        Value::String("arguments".to_string()),
                        Value::String(format!("{}{}", cur, args)),
                    );
                }
                found = true;
                break;
            }
        }
        if !found {
            let mut func = HashMap::new();
            func.insert("name".to_string(), Value::String(name));
            func.insert("arguments".to_string(), Value::String(args));
            let mut tool = HashMap::new();
            tool.insert("index".to_string(), Value::Number(index.into()));
            tool.insert("id".to_string(), Value::String(id));
            tool.insert(
                "type".to_string(),
                chunk
                    .get("type")
                    .cloned()
                    .unwrap_or(Value::String("function".to_string())),
            );
            tool.insert(
                "function".to_string(),
                serde_yaml::to_value(func).unwrap_or(Value::Null),
            );
            assembled.push(serde_yaml::to_value(tool).unwrap_or(Value::Null));
        }
    }

    let expected_calls = tc
        .expected
        .extra
        .get("assembled_tool_calls")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();
    if assembled != expected_calls {
        failures.push(format!(
            "assembled_tool_calls mismatch: expected {:?}, got {:?}",
            expected_calls, assembled
        ));
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

fn run_capability_guard(tc: &TestCase) -> Result<(), Vec<String>> {
    let mut failures = Vec::new();
    let method = tc
        .input
        .extra
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let manifest_raw = tc
        .input
        .extra
        .get("manifest")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let manifest: Value = serde_yaml::from_str(manifest_raw).unwrap_or(Value::Null);
    let cap_key = match method {
        "MCPListTools" => "mcp",
        "ComputerUse" => "computer_use",
        "Reason" => "reasoning",
        "VideoGenerate" => "video",
        _ => "",
    };
    let has_cap = if let Some(caps) = manifest.get("capabilities") {
        if let Some(seq) = caps.as_sequence() {
            seq.iter().any(|v| v.as_str() == Some(cap_key))
        } else if let Some(map) = caps.as_mapping() {
            map.contains_key(Value::String(cap_key.to_string()))
        } else {
            false
        }
    } else {
        false
    };
    let actual = if !has_cap { "E1005" } else { "" };
    let expected = tc.expected.error_code.as_deref().unwrap_or("");
    if actual != expected {
        failures.push(format!("error_code: expected {}, got {}", expected, actual));
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

fn run_advanced_endpoint_mapping(tc: &TestCase) -> Result<(), Vec<String>> {
    let mut failures = Vec::new();
    let operation = tc
        .input
        .extra
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let fallback = tc
        .input
        .extra
        .get("fallback")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let manifest_raw = tc
        .input
        .extra
        .get("manifest")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let manifest: Value = serde_yaml::from_str(manifest_raw).unwrap_or(Value::Null);
    let mut path = fallback.to_string();
    let mut method = "POST".to_string();
    if let Some(op) = manifest
        .get("core")
        .and_then(|v| v.get("endpoint"))
        .and_then(|v| v.get("endpoints"))
        .and_then(|v| v.get(operation))
    {
        if let Some(p) = op.get("path").and_then(Value::as_str) {
            path = p.to_string();
        }
        if let Some(m) = op.get("method").and_then(Value::as_str) {
            method = m.to_uppercase();
        }
    }
    let expected_path = tc
        .expected
        .extra
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let expected_method = tc
        .expected
        .extra
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !expected_path.is_empty() && path != expected_path {
        failures.push(format!("path: expected {}, got {}", expected_path, path));
    }
    if !expected_method.is_empty() && method != expected_method {
        failures.push(format!(
            "method: expected {}, got {}",
            expected_method, method
        ));
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

fn run_fallback_decision(tc: &TestCase) -> Result<(), Vec<String>> {
    let mut failures = Vec::new();
    let code = tc
        .input
        .extra
        .get("error_code")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let should = matches!(
        code,
        "E1002" | "E2001" | "E2002" | "E3001" | "E3002" | "E3003"
    );
    let expected = tc
        .expected
        .extra
        .get("should_fallback")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if should != expected {
        failures.push(format!(
            "should_fallback: expected {}, got {}",
            expected, should
        ));
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

fn run_provider_mock_behavior(tc: &TestCase) -> Result<(), Vec<String>> {
    let mut failures = Vec::new();
    let req = tc
        .input
        .extra
        .get("request_body")
        .cloned()
        .unwrap_or(Value::Null);
    let resp = tc
        .input
        .response_body
        .clone()
        .or_else(|| tc.input.extra.get("response_body").cloned())
        .unwrap_or(Value::Null);
    if let Some(asserts) = tc
        .expected
        .extra
        .get("request_assert")
        .and_then(Value::as_mapping)
    {
        for (k, v) in asserts {
            let path = k.as_str().unwrap_or_default();
            let got = value_at_path(&req, path);
            if got != Some(v) {
                failures.push(format!(
                    "request_assert {}: expected {:?}, got {:?}",
                    path, v, got
                ));
            }
        }
    }
    if let Some(asserts) = tc
        .expected
        .extra
        .get("response_assert")
        .and_then(Value::as_mapping)
    {
        for (k, v) in asserts {
            let path = k.as_str().unwrap_or_default();
            let got = value_at_path(&resp, path);
            if got != Some(v) {
                failures.push(format!(
                    "response_assert {}: expected {:?}, got {:?}",
                    path, v, got
                ));
            }
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

fn value_at_path<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = root;
    for part in path.split('.') {
        match cur {
            Value::Mapping(m) => {
                cur = m.get(Value::String(part.to_string()))?;
            }
            Value::Sequence(s) => {
                let idx: usize = part.parse().ok()?;
                cur = s.get(idx)?;
            }
            _ => return None,
        }
    }
    Some(cur)
}

#[test]
fn compliance_error_classification() {
    let compliance_dir = compliance_dir();
    if !compliance_dir.exists() {
        eprintln!(
            "[SKIP] Compliance directory does not exist: {}",
            compliance_dir.display()
        );
        eprintln!("       Set COMPLIANCE_DIR to override, or run from workspace with ai-protocol.");
        return;
    }

    let error_class_dir = compliance_dir.join("cases/02-error-classification");
    if !error_class_dir.exists() {
        eprintln!(
            "[SKIP] Error classification cases dir does not exist: {}",
            error_class_dir.display()
        );
        return;
    }

    let yaml_files = discover_yaml_files(&error_class_dir);
    let mut passed = 0u32;
    let mut failed = 0u32;

    for file in yaml_files {
        let content = match fs::read_to_string(&file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  [WARN] Could not read {}: {}", file.display(), e);
                continue;
            }
        };

        let cases = parse_test_cases(&content);
        for tc in cases {
            if tc.input.test_type != "error_classification" {
                continue;
            }

            match run_error_classification(&tc) {
                Ok(()) => {
                    println!(
                        "  [PASS] {} ({}) - {}",
                        tc.id,
                        tc.name,
                        tc.expected.error_code.as_deref().unwrap_or("<n/a>")
                    );
                    passed += 1;
                }
                Err(failures) => {
                    println!("  [FAIL] {} ({})", tc.id, tc.name);
                    for f in &failures {
                        println!("         {}", f);
                    }
                    failed += 1;
                }
            }
        }
    }

    println!("\n--- Compliance summary ---");
    println!("  Passed: {}", passed);
    println!("  Failed: {}", failed);

    assert_eq!(
        failed, 0,
        "{} error_classification compliance test(s) failed",
        failed
    );
}

#[test]
fn compliance_protocol_loading() {
    let compliance_dir = compliance_dir();
    if !compliance_dir.exists() {
        eprintln!(
            "[SKIP] Compliance directory does not exist: {}",
            compliance_dir.display()
        );
        return;
    }

    let loading_dir = compliance_dir.join("cases/01-protocol-loading");
    if !loading_dir.exists() {
        eprintln!(
            "[SKIP] Protocol loading cases dir does not exist: {}",
            loading_dir.display()
        );
        return;
    }

    let yaml_files = discover_yaml_files(&loading_dir);
    let mut passed = 0u32;
    let mut failed = 0u32;

    for file in yaml_files {
        let content = match fs::read_to_string(&file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  [WARN] Could not read {}: {}", file.display(), e);
                continue;
            }
        };

        let cases = parse_test_cases(&content);
        for tc in cases {
            if tc.input.test_type != "protocol_loading" {
                continue;
            }

            match run_protocol_loading(&tc, &compliance_dir) {
                Ok(()) => {
                    println!("  [PASS] {} ({})", tc.id, tc.name);
                    passed += 1;
                }
                Err(failures) => {
                    println!("  [FAIL] {} ({})", tc.id, tc.name);
                    for f in &failures {
                        println!("         {}", f);
                    }
                    failed += 1;
                }
            }
        }
    }

    println!("\n--- Protocol loading summary ---");
    println!("  Passed: {}", passed);
    println!("  Failed: {}", failed);

    assert_eq!(
        failed, 0,
        "{} protocol_loading compliance test(s) failed",
        failed
    );
}

#[test]
fn compliance_retry_decision() {
    let compliance_dir = compliance_dir();
    if !compliance_dir.exists() {
        eprintln!(
            "[SKIP] Compliance directory does not exist: {}",
            compliance_dir.display()
        );
        return;
    }

    let resilience_dir = compliance_dir.join("cases/06-resilience");
    if !resilience_dir.exists() {
        eprintln!(
            "[SKIP] Resilience cases dir does not exist: {}",
            resilience_dir.display()
        );
        return;
    }

    let yaml_files = discover_yaml_files(&resilience_dir);
    let mut passed = 0u32;
    let mut failed = 0u32;

    for file in yaml_files {
        let content = match fs::read_to_string(&file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  [WARN] Could not read {}: {}", file.display(), e);
                continue;
            }
        };

        let cases = parse_test_cases(&content);
        for tc in cases {
            if tc.input.test_type != "retry_decision" {
                continue;
            }

            match run_retry_decision(&tc) {
                Ok(()) => {
                    println!("  [PASS] {} ({})", tc.id, tc.name);
                    passed += 1;
                }
                Err(failures) => {
                    println!("  [FAIL] {} ({})", tc.id, tc.name);
                    for f in &failures {
                        println!("         {}", f);
                    }
                    failed += 1;
                }
            }
        }
    }

    println!("\n--- Retry decision summary ---");
    println!("  Passed: {}", passed);
    println!("  Failed: {}", failed);

    assert_eq!(
        failed, 0,
        "{} retry_decision compliance test(s) failed",
        failed
    );
}

#[test]
fn compliance_message_stream_request_cases() {
    let compliance_dir = compliance_dir();
    if !compliance_dir.exists() {
        eprintln!(
            "[SKIP] Compliance directory does not exist: {}",
            compliance_dir.display()
        );
        return;
    }

    let case_dirs = [
        compliance_dir.join("cases/03-message-building"),
        compliance_dir.join("cases/04-streaming"),
        compliance_dir.join("cases/05-request-building"),
        compliance_dir.join("cases/07-advanced-capabilities"),
    ];

    let mut passed = 0u32;
    let mut failed = 0u32;

    for dir in case_dirs {
        if !dir.exists() {
            continue;
        }
        let yaml_files = discover_yaml_files(&dir);
        for file in yaml_files {
            let content = match fs::read_to_string(&file) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("  [WARN] Could not read {}: {}", file.display(), e);
                    continue;
                }
            };
            let cases = parse_test_cases(&content);
            for tc in cases {
                let result = match tc.input.test_type.as_str() {
                    "message_building" => run_message_building(&tc),
                    "parameter_mapping" => run_parameter_mapping(&tc, &compliance_dir),
                    "stream_decode" => run_stream_decode(&tc),
                    "event_mapping" => run_event_mapping(&tc),
                    "tool_accumulation" => run_tool_accumulation(&tc),
                    "capability_guard" => run_capability_guard(&tc),
                    "advanced_endpoint_mapping" => run_advanced_endpoint_mapping(&tc),
                    "fallback_decision" => run_fallback_decision(&tc),
                    "provider_mock_behavior" => run_provider_mock_behavior(&tc),
                    _ => continue,
                };
                match result {
                    Ok(()) => {
                        println!("  [PASS] {} ({})", tc.id, tc.name);
                        passed += 1;
                    }
                    Err(failures) => {
                        println!("  [FAIL] {} ({})", tc.id, tc.name);
                        for f in &failures {
                            println!("         {}", f);
                        }
                        failed += 1;
                    }
                }
            }
        }
    }

    println!("\n--- Message/Stream/Request summary ---");
    println!("  Passed: {}", passed);
    println!("  Failed: {}", failed);

    assert_eq!(
        failed, 0,
        "{} message/stream/request compliance test(s) failed",
        failed
    );
}
