//! Content filtering implementations

use super::config::{FilterAction, FilterRule};
use super::result::{Violation, ViolationType};

/// Trait for content filters
pub trait ContentFilter: Send + Sync {
    /// Check content for violations
    fn check(&self, content: &str) -> Vec<Violation>;
    
    /// Sanitize content by replacing violations
    fn sanitize(&self, content: &str, replacement: &str) -> String;
}

/// Keyword-based content filter
#[derive(Debug, Clone, Default)]
pub struct KeywordFilter {
    rules: Vec<CompiledKeywordRule>,
}

#[derive(Debug, Clone)]
struct CompiledKeywordRule {
    keyword: String,
    keyword_lower: String,
    case_sensitive: bool,
    action: FilterAction,
    category: Option<String>,
    description: Option<String>,
}

impl KeywordFilter {
    /// Create a new empty keyword filter
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Create from filter rules
    pub fn from_rules(rules: &[FilterRule]) -> Self {
        let compiled_rules: Vec<CompiledKeywordRule> = rules
            .iter()
            .filter(|r| !r.is_regex)
            .map(|r| CompiledKeywordRule {
                keyword: r.pattern.clone(),
                keyword_lower: r.pattern.to_lowercase(),
                case_sensitive: r.case_sensitive,
                action: r.action,
                category: r.category.clone(),
                description: r.description.clone(),
            })
            .collect();

        Self { rules: compiled_rules }
    }

    /// Add a keyword rule
    pub fn add_keyword(&mut self, keyword: impl Into<String>, action: FilterAction) {
        let keyword = keyword.into();
        self.rules.push(CompiledKeywordRule {
            keyword_lower: keyword.to_lowercase(),
            keyword,
            case_sensitive: false,
            action,
            category: None,
            description: None,
        });
    }
}

impl ContentFilter for KeywordFilter {
    fn check(&self, content: &str) -> Vec<Violation> {
        let content_lower = content.to_lowercase();
        let mut violations = Vec::new();

        for rule in &self.rules {
            let matched = if rule.case_sensitive {
                content.contains(&rule.keyword)
            } else {
                content_lower.contains(&rule.keyword_lower)
            };

            if matched {
                violations.push(Violation {
                    violation_type: ViolationType::Keyword,
                    pattern: rule.keyword.clone(),
                    action: rule.action,
                    category: rule.category.clone(),
                    description: rule.description.clone(),
                    matched_text: Some(rule.keyword.clone()),
                });
            }
        }

        violations
    }

    fn sanitize(&self, content: &str, replacement: &str) -> String {
        let mut result = content.to_string();

        for rule in &self.rules {
            if matches!(rule.action, FilterAction::Sanitize | FilterAction::Block) {
                if rule.case_sensitive {
                    result = result.replace(&rule.keyword, replacement);
                } else {
                    // Case-insensitive replacement
                    let lower = result.to_lowercase();
                    let keyword_lower = &rule.keyword_lower;
                    
                    let mut new_result = String::new();
                    let mut last_end = 0;
                    
                    for (start, _) in lower.match_indices(keyword_lower) {
                        new_result.push_str(&result[last_end..start]);
                        new_result.push_str(replacement);
                        last_end = start + keyword_lower.len();
                    }
                    new_result.push_str(&result[last_end..]);
                    result = new_result;
                }
            }
        }

        result
    }
}

/// Regex pattern-based content filter
#[derive(Debug, Clone, Default)]
pub struct PatternFilter {
    rules: Vec<CompiledPatternRule>,
}

#[derive(Debug, Clone)]
struct CompiledPatternRule {
    pattern_str: String,
    case_sensitive: bool,
    action: FilterAction,
    category: Option<String>,
    description: Option<String>,
}

impl PatternFilter {
    /// Create a new empty pattern filter
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Create from filter rules
    pub fn from_rules(rules: &[FilterRule]) -> Self {
        let compiled_rules: Vec<CompiledPatternRule> = rules
            .iter()
            .filter(|r| r.is_regex)
            .map(|r| CompiledPatternRule {
                pattern_str: r.pattern.clone(),
                case_sensitive: r.case_sensitive,
                action: r.action,
                category: r.category.clone(),
                description: r.description.clone(),
            })
            .collect();

        Self { rules: compiled_rules }
    }

    /// Add a pattern rule
    pub fn add_pattern(&mut self, pattern: impl Into<String>, action: FilterAction) {
        self.rules.push(CompiledPatternRule {
            pattern_str: pattern.into(),
            case_sensitive: true,
            action,
            category: None,
            description: None,
        });
    }

    /// Compile a pattern string to regex
    fn compile_pattern(pattern: &str, case_sensitive: bool) -> Option<regex::Regex> {
        let pattern_str = if case_sensitive {
            pattern.to_string()
        } else {
            format!("(?i){}", pattern)
        };
        
        regex::Regex::new(&pattern_str).ok()
    }
}

impl ContentFilter for PatternFilter {
    fn check(&self, content: &str) -> Vec<Violation> {
        let mut violations = Vec::new();

        for rule in &self.rules {
            if let Some(re) = Self::compile_pattern(&rule.pattern_str, rule.case_sensitive) {
                if let Some(m) = re.find(content) {
                    violations.push(Violation {
                        violation_type: ViolationType::Pattern,
                        pattern: rule.pattern_str.clone(),
                        action: rule.action,
                        category: rule.category.clone(),
                        description: rule.description.clone(),
                        matched_text: Some(m.as_str().to_string()),
                    });
                }
            }
        }

        violations
    }

    fn sanitize(&self, content: &str, replacement: &str) -> String {
        let mut result = content.to_string();

        for rule in &self.rules {
            if matches!(rule.action, FilterAction::Sanitize | FilterAction::Block) {
                if let Some(re) = Self::compile_pattern(&rule.pattern_str, rule.case_sensitive) {
                    result = re.replace_all(&result, replacement).to_string();
                }
            }
        }

        result
    }
}
