use ai_lib_core::types::message::{ContentBlock, Message, MessageContent, MessageRole};

use super::budget::{ContextBudget, ModelCapacity};
use super::error::AssembleError;
use super::token_estimate::estimate_message_tokens;

/// Options for deterministic context assembly (no LLM summarization).
#[derive(Debug, Clone)]
pub struct AssembleOptions {
    pub budget: ContextBudget,
    pub capacity: ModelCapacity,
    /// Replace tool payloads larger than this (chars) with `tool_placeholder`.
    pub tool_fold_threshold_chars: usize,
    pub tool_placeholder: String,
}

impl Default for AssembleOptions {
    fn default() -> Self {
        Self {
            budget: ContextBudget::from_capacity(ModelCapacity::UNKNOWN, 2),
            capacity: ModelCapacity::UNKNOWN,
            tool_fold_threshold_chars: 8_192,
            tool_placeholder: "[tool output truncated]".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AssembleReport {
    pub messages: Vec<Message>,
    pub dropped_prefix: usize,
    pub folded_tool_segments: usize,
}

pub struct MessageAssembler;

impl MessageAssembler {
    pub fn assemble(
        messages: &[Message],
        options: &AssembleOptions,
    ) -> Result<AssembleReport, AssembleError> {
        if messages.is_empty() {
            return Err(AssembleError::EmptyInput);
        }

        let mut working: Vec<Message> = messages.to_vec();
        let folded_tool_segments = fold_oversized_tool_content(
            &mut working,
            options.tool_fold_threshold_chars,
            &options.tool_placeholder,
        );

        let budget = options.budget.max_input_tokens;
        let min_tail = options.budget.min_tail_messages;
        let start = select_suffix_start(&working, budget, min_tail);
        let dropped_prefix = start;

        Ok(AssembleReport {
            messages: working[start..].to_vec(),
            dropped_prefix,
            folded_tool_segments,
        })
    }
}

fn fold_oversized_tool_content(
    messages: &mut [Message],
    threshold: usize,
    placeholder: &str,
) -> usize {
    let mut folded = 0usize;

    for message in messages.iter_mut() {
        match &mut message.content {
            MessageContent::Text(text) if message.role == MessageRole::Tool => {
                if text.len() > threshold {
                    *text = placeholder.to_string();
                    folded += 1;
                }
            }
            MessageContent::Blocks(blocks) => {
                for block in blocks.iter_mut() {
                    if let ContentBlock::ToolResult { content, .. } = block {
                        let serialized = content.to_string();
                        if serialized.len() > threshold {
                            *content = serde_json::Value::String(placeholder.to_string());
                            folded += 1;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    folded
}

fn select_suffix_start(messages: &[Message], budget: u32, min_tail: usize) -> usize {
    let n = messages.len();
    if n == 0 {
        return 0;
    }

    let mut start = n;
    let mut used = 0u32;

    for i in (0..n).rev() {
        let cost = estimate_message_tokens(&messages[i]);
        let kept = n - start;

        if kept >= min_tail && start < n && used.saturating_add(cost) > budget {
            break;
        }

        if start == n && cost > budget && i + 1 == n {
            start = i;
            break;
        }

        used = used.saturating_add(cost);
        start = i;
    }

    start = trim_leading_orphan_tools(messages, start, n);
    start = extend_for_tool_chain(messages, start, n, budget);

    start.min(n)
}

fn trim_leading_orphan_tools(messages: &[Message], start: usize, end: usize) -> usize {
    let mut s = start;
    while s < end && messages[s].role == MessageRole::Tool {
        s += 1;
    }
    s
}

/// If the kept window ends with tool results, walk backward to include the initiating assistant.
fn extend_for_tool_chain(messages: &[Message], start: usize, end: usize, budget: u32) -> usize {
    if start == 0 || start >= end {
        return start;
    }

    let s = start;
    if messages[end - 1].role != MessageRole::Tool {
        return s;
    }

    let mut i = start;
    while i < end && messages[i].role == MessageRole::Tool {
        i += 1;
    }

    if i < end {
        return s;
    }

    for j in (0..start).rev() {
        if messages[j].role == MessageRole::Assistant {
            let candidate = j;
            let slice = &messages[candidate..end];
            let cost: u32 = slice.iter().map(estimate_message_tokens).sum();
            if cost <= budget {
                return candidate;
            }
            break;
        }
    }

    trim_leading_orphan_tools(messages, start, end)
}

mod tests {
    use super::*;
    use ai_lib_core::types::message::Message;

    fn opts(budget: u32, min_tail: usize) -> AssembleOptions {
        AssembleOptions {
            budget: ContextBudget::new(budget, 0, min_tail),
            ..Default::default()
        }
    }

    #[test]
    fn drops_oldest_when_over_budget() {
        let messages: Vec<Message> = (0..20)
            .map(|i| Message::user(format!("msg-{i}-{}", "x".repeat(40))))
            .collect();

        let report = MessageAssembler::assemble(&messages, &opts(120, 1)).unwrap();
        assert!(report.dropped_prefix > 0);
        assert!(!report.messages.is_empty());
        let tokens: u32 = report
            .messages
            .iter()
            .map(estimate_message_tokens)
            .sum();
        assert!(tokens <= 200);
    }

    #[test]
    fn keeps_minimum_tail_messages() {
        let messages = vec![
            Message::user("old"),
            Message::user("mid"),
            Message::assistant("newest"),
        ];

        let report = MessageAssembler::assemble(&messages, &opts(10, 2)).unwrap();
        assert!(report.messages.len() >= 2);
    }

    #[test]
    fn folds_oversized_tool_text() {
        let huge = "x".repeat(20_000);
        let messages = vec![
            Message::user("q"),
            Message::tool("call_1", huge),
            Message::assistant("done"),
        ];

        let report = MessageAssembler::assemble(&messages, &opts(50_000, 1)).unwrap();
        assert_eq!(report.folded_tool_segments, 1);
        let tool = report
            .messages
            .iter()
            .find(|m| m.role == MessageRole::Tool)
            .unwrap();
        if let MessageContent::Text(text) = &tool.content {
            assert_eq!(text, "[tool output truncated]");
        } else {
            panic!("expected text tool content");
        }
    }

    #[test]
    fn does_not_start_with_orphan_tool() {
        let messages = vec![
            Message::user("u1"),
            Message::assistant("a1"),
            Message::tool("call_1", "result"),
            Message::user("u2"),
            Message::assistant("a2"),
        ];

        let report = MessageAssembler::assemble(&messages, &opts(30, 1)).unwrap();
        assert_ne!(report.messages.first().unwrap().role, MessageRole::Tool);
    }

    #[test]
    fn empty_input_errors() {
        let err = MessageAssembler::assemble(&[], &opts(100, 1)).unwrap_err();
        assert_eq!(err, AssembleError::EmptyInput);
    }

    #[test]
    fn budget_from_capacity_subtracts_output_reserve() {
        let budget = ContextBudget::from_capacity(ModelCapacity::new(128_000, 8_192), 2);
        assert_eq!(budget.max_input_tokens, 119_808);
        assert_eq!(budget.reserve_output_tokens, 8_192);
    }

    #[test]
    fn token_estimate_heuristic() {
        assert_eq!(crate::context::estimate_tokens("abcd"), 1);
        assert_eq!(crate::context::estimate_tokens("abcdefgh"), 2);
    }
}
