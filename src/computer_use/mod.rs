//! Computer Use 抽象层 — 提供跨厂商的 GUI 自动化操作标准化和安全控制
//!
//! Computer Use abstraction layer for AI-Protocol. Provides:
//! - Normalized action types across providers (screen_based, tool_based)
//! - Safety policy enforcement (confirmation, sandbox, logging, domain allowlist)
//! - Provider-specific configuration extraction
//! - Action validation before execution

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::protocol::v2::manifest::ComputerUseConfig;

// ─── Normalized Action Types ────────────────────────────────────────────────

/// A normalized computer use action — provider-agnostic.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action_type")]
pub enum ComputerAction {
    /// Take a screenshot of the current display.
    Screenshot {
        #[serde(default = "default_screenshot_format")]
        format: String,
    },
    /// Click at screen coordinates.
    MouseClick {
        x: f64,
        y: f64,
        #[serde(default = "default_mouse_button")]
        button: MouseButton,
    },
    /// Double-click at screen coordinates.
    MouseDoubleClick { x: f64, y: f64 },
    /// Drag from start to end coordinates.
    MouseDrag {
        start_x: f64,
        start_y: f64,
        end_x: f64,
        end_y: f64,
    },
    /// Scroll at position.
    Scroll {
        x: f64,
        y: f64,
        /// Positive = down, negative = up.
        delta_y: i32,
        #[serde(default)]
        delta_x: i32,
    },
    /// Move cursor to position (no click).
    MouseMove { x: f64, y: f64 },
    /// Type text (with optional delay between keystrokes).
    KeyboardType { text: String },
    /// Press a single key or key combination.
    KeyboardShortcut { keys: Vec<String> },
    /// Navigate to a URL (browser mode).
    BrowserNavigate { url: String },
    /// Click a DOM element by selector (tool_based mode).
    BrowserClickElement { selector: String },
    /// Fill a form field (tool_based mode).
    BrowserFillForm {
        selector: String,
        value: String,
    },
    /// Zoom into a screen region for detailed inspection (Anthropic-specific).
    ZoomRegion {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    /// Read a file (Google tool_based mode).
    FileRead { path: String },
    /// Write a file (Google tool_based mode).
    FileWrite { path: String, content: String },
}

fn default_screenshot_format() -> String {
    "png".to_string()
}

fn default_mouse_button() -> MouseButton {
    MouseButton::Left
}

/// Mouse button enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Implementation approach of the provider's computer use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImplementationStyle {
    /// Screenshot-loop with pixel coordinates (OpenAI, Anthropic).
    ScreenBased,
    /// Structured semantic actions with DOM selectors (Google).
    ToolBased,
    /// Both approaches available.
    Hybrid,
}

// ─── Safety Policy ──────────────────────────────────────────────────────────

/// Safety policy for computer use actions.
///
/// Loaded from the manifest's `computer_use.safety` configuration.
/// All validations are enforced *before* the action is dispatched
/// to the execution environment.
#[derive(Debug, Clone)]
pub struct SafetyPolicy {
    /// Require human confirmation for consequential actions.
    pub confirmation_required: bool,
    /// Sandbox requirement level.
    pub sandbox_mode: SandboxMode,
    /// Whether all actions are logged for audit.
    pub action_logging: bool,
    /// Allowed domains for browser navigation (empty = unrestricted).
    pub domain_allowlist: HashSet<String>,
    /// Whether sensitive data access is blocked.
    pub sensitive_data_protection: bool,
    /// Max actions per model turn (0 = unlimited).
    pub max_actions_per_turn: u32,
    /// Timeout per individual action in milliseconds.
    pub action_timeout_ms: u32,
}

/// Sandbox enforcement level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxMode {
    Required,
    Recommended,
    Optional,
}

impl Default for SafetyPolicy {
    fn default() -> Self {
        Self {
            confirmation_required: true,
            sandbox_mode: SandboxMode::Recommended,
            action_logging: true,
            domain_allowlist: HashSet::new(),
            sensitive_data_protection: true,
            max_actions_per_turn: 0,
            action_timeout_ms: 30_000,
        }
    }
}

impl SafetyPolicy {
    /// Build a safety policy from a manifest's computer_use config.
    pub fn from_config(config: &ComputerUseConfig) -> Self {
        let mut policy = Self::default();
        if let Some(safety) = &config.safety {
            if let Some(v) = safety.get("confirmation_required").and_then(|v| v.as_bool()) {
                policy.confirmation_required = v;
            }
            if let Some(s) = safety.get("sandbox_mode").and_then(|v| v.as_str()) {
                policy.sandbox_mode = match s {
                    "required" => SandboxMode::Required,
                    "optional" => SandboxMode::Optional,
                    _ => SandboxMode::Recommended,
                };
            }
            if let Some(v) = safety.get("action_logging").and_then(|v| v.as_bool()) {
                policy.action_logging = v;
            }
            if let Some(v) = safety.get("sensitive_data_protection").and_then(|v| v.as_bool()) {
                policy.sensitive_data_protection = v;
            }
            if let Some(v) = safety.get("max_actions_per_turn").and_then(|v| v.as_u64()) {
                policy.max_actions_per_turn = v as u32;
            }
            if let Some(v) = safety.get("action_timeout_ms").and_then(|v| v.as_u64()) {
                policy.action_timeout_ms = v as u32;
            }
        }
        policy
    }

    /// Validate an action against this safety policy.
    pub fn validate_action(
        &self,
        action: &ComputerAction,
        actions_this_turn: u32,
    ) -> Result<(), SafetyViolation> {
        // Check max actions per turn
        if self.max_actions_per_turn > 0 && actions_this_turn >= self.max_actions_per_turn {
            return Err(SafetyViolation::MaxActionsExceeded {
                limit: self.max_actions_per_turn,
                attempted: actions_this_turn + 1,
            });
        }

        // Check domain allowlist for browser navigation
        if let ComputerAction::BrowserNavigate { url } = action {
            if !self.domain_allowlist.is_empty() {
                let domain = extract_domain(url);
                if !self.domain_allowlist.contains(&domain) {
                    return Err(SafetyViolation::DomainNotAllowed {
                        domain,
                        allowlist: self.domain_allowlist.iter().cloned().collect(),
                    });
                }
            }
        }

        // Check sensitive data protection for file operations
        if self.sensitive_data_protection {
            match action {
                ComputerAction::FileRead { path } | ComputerAction::FileWrite { path, .. } => {
                    if is_sensitive_path(path) {
                        return Err(SafetyViolation::SensitiveDataAccess {
                            path: path.clone(),
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}

/// A safety policy violation.
#[derive(Debug, Clone)]
pub enum SafetyViolation {
    /// Maximum actions per turn exceeded.
    MaxActionsExceeded { limit: u32, attempted: u32 },
    /// Navigation to a domain not in the allowlist.
    DomainNotAllowed {
        domain: String,
        allowlist: Vec<String>,
    },
    /// Attempted access to a sensitive file path.
    SensitiveDataAccess { path: String },
    /// Sandbox is required but environment is not sandboxed.
    SandboxRequired,
}

impl std::fmt::Display for SafetyViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxActionsExceeded { limit, attempted } => {
                write!(f, "Max actions per turn exceeded: limit={limit}, attempted={attempted}")
            }
            Self::DomainNotAllowed { domain, .. } => {
                write!(f, "Domain '{domain}' is not in the allowlist")
            }
            Self::SensitiveDataAccess { path } => {
                write!(f, "Access to sensitive path '{path}' is blocked")
            }
            Self::SandboxRequired => write!(f, "Sandbox environment is required"),
        }
    }
}

// ─── Provider Configuration ─────────────────────────────────────────────────

/// Provider-specific computer use configuration.
#[derive(Debug, Clone)]
pub struct CuProviderConfig {
    /// Provider's tool type identifier (e.g., "computer_20251124").
    pub tool_type: String,
    /// Required beta header, if any.
    pub beta_header: Option<String>,
    /// Implementation style (screen_based, tool_based, hybrid).
    pub implementation: ImplementationStyle,
    /// Specific model required for computer use, if any.
    pub model_requirement: Option<String>,
}

/// Extract provider-specific CU configuration from a manifest.
pub fn extract_provider_config(config: &ComputerUseConfig) -> Option<CuProviderConfig> {
    if !config.supported {
        return None;
    }

    let implementation = config
        .implementation
        .as_deref()
        .map(|s| match s {
            "tool_based" => ImplementationStyle::ToolBased,
            "hybrid" => ImplementationStyle::Hybrid,
            _ => ImplementationStyle::ScreenBased,
        })
        .unwrap_or(ImplementationStyle::ScreenBased);

    let mapping = config.provider_mapping.as_ref();

    let tool_type = mapping
        .and_then(|m| m.get("tool_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("computer_use")
        .to_string();

    let beta_header = mapping
        .and_then(|m| m.get("beta_header"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let model_requirement = mapping
        .and_then(|m| m.get("model_requirement"))
        .and_then(|v| v.as_str())
        .map(String::from);

    Some(CuProviderConfig {
        tool_type,
        beta_header,
        implementation,
        model_requirement,
    })
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn extract_domain(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .to_string()
}

fn is_sensitive_path(path: &str) -> bool {
    let sensitive_patterns = [
        ".ssh", ".gnupg", ".aws", "credentials", "secrets",
        ".env", "password", "token", ".kube/config",
    ];
    let lower = path.to_lowercase();
    sensitive_patterns.iter().any(|p| lower.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_policy_default() {
        let policy = SafetyPolicy::default();
        assert!(policy.confirmation_required);
        assert_eq!(policy.sandbox_mode, SandboxMode::Recommended);
        assert!(policy.action_logging);
    }

    #[test]
    fn test_safety_max_actions() {
        let policy = SafetyPolicy {
            max_actions_per_turn: 5,
            ..Default::default()
        };
        let action = ComputerAction::Screenshot { format: "png".into() };
        assert!(policy.validate_action(&action, 4).is_ok());
        assert!(policy.validate_action(&action, 5).is_err());
    }

    #[test]
    fn test_safety_domain_allowlist() {
        let mut policy = SafetyPolicy::default();
        policy.domain_allowlist.insert("example.com".into());
        policy.domain_allowlist.insert("safe.org".into());

        let ok_action = ComputerAction::BrowserNavigate {
            url: "https://example.com/page".into(),
        };
        assert!(policy.validate_action(&ok_action, 0).is_ok());

        let blocked_action = ComputerAction::BrowserNavigate {
            url: "https://evil.com/phish".into(),
        };
        assert!(policy.validate_action(&blocked_action, 0).is_err());
    }

    #[test]
    fn test_safety_sensitive_path() {
        let policy = SafetyPolicy::default();
        let action = ComputerAction::FileRead {
            path: "/home/user/.ssh/id_rsa".into(),
        };
        assert!(policy.validate_action(&action, 0).is_err());

        let safe = ComputerAction::FileRead {
            path: "/tmp/output.txt".into(),
        };
        assert!(policy.validate_action(&safe, 0).is_ok());
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(extract_domain("https://example.com/page"), "example.com");
        assert_eq!(extract_domain("http://localhost:8080/api"), "localhost");
        assert_eq!(extract_domain("https://sub.domain.co.uk/path"), "sub.domain.co.uk");
    }

    #[test]
    fn test_provider_config_extraction() {
        let config = ComputerUseConfig {
            supported: true,
            status: Some("beta".into()),
            implementation: Some("screen_based".into()),
            actions: None,
            safety: None,
            environment: None,
            provider_mapping: Some(std::collections::HashMap::from([
                ("tool_type".into(), serde_json::Value::String("computer_20251124".into())),
                ("beta_header".into(), serde_json::Value::String("computer-use-2025-11-24".into())),
            ])),
        };
        let prov = extract_provider_config(&config).unwrap();
        assert_eq!(prov.tool_type, "computer_20251124");
        assert_eq!(prov.beta_header.as_deref(), Some("computer-use-2025-11-24"));
        assert_eq!(prov.implementation, ImplementationStyle::ScreenBased);
    }

    #[test]
    fn test_unsupported_returns_none() {
        let config = ComputerUseConfig::default();
        assert!(extract_provider_config(&config).is_none());
    }
}
