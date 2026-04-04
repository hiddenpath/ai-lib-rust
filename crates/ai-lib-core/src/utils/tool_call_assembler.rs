use crate::types::tool::ToolCall;

/// Collects tool call events (started + argument fragments) into final ToolCall objects.
/// This is intentionally tolerant: if JSON parsing fails, it keeps the raw string.
#[derive(Default)]
pub struct ToolCallAssembler {
    tool_calls: Vec<ToolCall>,
}

impl ToolCallAssembler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_started(&mut self, id: String, name: String) {
        if self.tool_calls.iter().any(|t| t.id == id) {
            return;
        }
        self.tool_calls.push(ToolCall {
            id,
            name,
            arguments: serde_json::Value::String(String::new()),
        });
    }

    pub fn on_partial(&mut self, id: &str, fragment: &str) {
        if let Some(tc) = self.tool_calls.iter_mut().find(|t| t.id == id) {
            match &mut tc.arguments {
                serde_json::Value::String(s) => s.push_str(fragment),
                _ => tc.arguments = serde_json::Value::String(fragment.to_string()),
            }
        }
    }

    pub fn finalize(mut self) -> Vec<ToolCall> {
        for tc in &mut self.tool_calls {
            if let serde_json::Value::String(s) = &tc.arguments {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
                        tc.arguments = v;
                    }
                }
            }
        }
        self.tool_calls
    }
}
