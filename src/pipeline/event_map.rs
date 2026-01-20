//! Event mapping (JSON Value -> StreamingEvent)
//!
//! Two modes:
//! - If manifest provides `streaming.event_map`, use rule-based mapping
//! - Otherwise, fallback to built-in adapter mapping (e.g. openai-style)

use crate::pipeline::{Mapper, PipelineError};
use crate::protocol::EventMapRule;
use crate::protocol::ToolUseMapping;
use crate::types::events::StreamingEvent;
use crate::utils::JsonPathEvaluator;
use crate::{BoxStream, PipeResult};
use futures::{stream, StreamExt};
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use tracing::debug;

#[derive(Clone)]
struct CompiledRule {
    matcher: JsonPathEvaluator,
    emit: String,
    extract: Vec<(String, String)>, // (field_name, json_path)
}

pub struct RuleBasedEventMapper {
    rules: Vec<CompiledRule>,
}

impl RuleBasedEventMapper {
    pub fn new(rules: &[EventMapRule]) -> Result<Self, PipelineError> {
        let mut compiled = Vec::new();
        for r in rules {
            let matcher = JsonPathEvaluator::new(&r.match_expr).map_err(|e| {
                PipelineError::InvalidJsonPath {
                    path: r.match_expr.clone(),
                    error: e.to_string(),
                    hint: None,
                }
            })?;
            let mut extract = Vec::new();
            if let Some(map) = &r.fields {
                for (k, v) in map {
                    extract.push((k.clone(), v.clone()));
                }
            }
            compiled.push(CompiledRule {
                matcher,
                emit: r.emit.clone(),
                extract,
            });
        }
        Ok(Self { rules: compiled })
    }

    fn build_event(
        emit: &str,
        frame: &Value,
        extract: &[(String, String)],
    ) -> Option<StreamingEvent> {
        match emit {
            "PartialContentDelta" => {
                // Expect extracted `content` or infer from common openai path
                let mut content: Option<String> = None;
                for (k, p) in extract {
                    if k == "content" {
                        content = crate::utils::PathMapper::get_string(frame, p);
                    }
                }
                let content = content.or_else(|| {
                    crate::utils::PathMapper::get_string(frame, "$.choices[0].delta.content")
                })?;

                // Filter out empty content to avoid unnecessary events and ensure consistency
                // with fallback mapper behavior. Empty content can occur when providers send
                // frames with null/empty delta.content (e.g., during tool calls or finish events).
                if content.is_empty() {
                    return None;
                }

                Some(StreamingEvent::PartialContentDelta {
                    content,
                    sequence_id: None,
                })
            }
            "Metadata" => {
                // usage optional
                let usage = crate::utils::PathMapper::get_path(frame, "$.usage").cloned();
                Some(StreamingEvent::Metadata {
                    usage,
                    finish_reason: None,
                    stop_reason: None,
                })
            }
            "StreamEnd" => Some(StreamingEvent::StreamEnd {
                finish_reason: None,
            }),
            _ => None,
        }
    }
}

#[async_trait::async_trait]
impl Mapper for RuleBasedEventMapper {
    async fn map(
        &self,
        input: BoxStream<'static, Value>,
    ) -> PipeResult<BoxStream<'static, StreamingEvent>> {
        let rules = self.rules.clone();

        let mapped = stream::unfold((input, false), move |(mut input, mut ended)| {
            let rules = rules.clone();
            async move {
                if ended {
                    return None;
                }

                while let Some(item) = input.next().await {
                    match item {
                        Ok(frame) => {
                            for r in &rules {
                                if r.matcher.matches(&frame) {
                                    if let Some(ev) = RuleBasedEventMapper::build_event(
                                        &r.emit, &frame, &r.extract,
                                    ) {
                                        return Some((Ok(ev), (input, ended)));
                                    }
                                    // Rule matched but build_event returned None (e.g., empty content filtered)
                                    // This is expected behavior, continue to next rule or frame
                                }
                            }

                            // If no rule matched, skip this frame silently
                            // This is normal for frames that don't match any event pattern
                            // (e.g., ping frames, metadata-only frames, etc.)
                            continue;
                        }
                        Err(e) => return Some((Err(e), (input, ended))),
                    }
                }

                // EOF: emit StreamEnd exactly once
                ended = true;
                Some((
                    Ok(StreamingEvent::StreamEnd {
                        finish_reason: None,
                    }),
                    (input, ended),
                ))
            }
        });

        Ok(Box::pin(mapped))
    }
}

/// Fallback openai-style mapping when no event_map rules are provided.
pub struct OpenAiStyleEventMapper;

#[async_trait::async_trait]
impl Mapper for OpenAiStyleEventMapper {
    async fn map(
        &self,
        input: BoxStream<'static, Value>,
    ) -> PipeResult<BoxStream<'static, StreamingEvent>> {
        let stream = stream::unfold((input, false), move |(mut input, mut ended)| async move {
            if ended {
                return None;
            }

            while let Some(item) = input.next().await {
                match item {
                    Ok(frame) => {
                        // content delta
                        if let Some(content) = crate::utils::PathMapper::get_string(
                            &frame,
                            "$.choices[0].delta.content",
                        ) {
                            if !content.is_empty() {
                                return Some((
                                    Ok(StreamingEvent::PartialContentDelta {
                                        content,
                                        sequence_id: None,
                                    }),
                                    (input, ended),
                                ));
                            }
                        }

                        // usage metadata (rare in streaming but possible)
                        if let Some(usage) =
                            crate::utils::PathMapper::get_path(&frame, "$.usage").cloned()
                        {
                            return Some((
                                Ok(StreamingEvent::Metadata {
                                    usage: Some(usage),
                                    finish_reason: None,
                                    stop_reason: None,
                                }),
                                (input, ended),
                            ));
                        }

                        continue;
                    }
                    Err(e) => return Some((Err(e), (input, ended))),
                }
            }

            ended = true;
            Some((
                Ok(StreamingEvent::StreamEnd {
                    finish_reason: None,
                }),
                (input, ended),
            ))
        });

        Ok(Box::pin(stream))
    }
}

pub fn create_event_mapper(rules: &[EventMapRule]) -> Result<Box<dyn Mapper>, PipelineError> {
    Ok(Box::new(RuleBasedEventMapper::new(rules)?))
}

/// Manifest-driven path mapper for streaming frames.
/// Supports:
/// - content_path (text deltas)
/// - tool_call_path (OpenAI-style tool_calls delta array)
/// - usage_path (usage metadata)
pub struct PathEventMapper {
    content_path: String,
    tool_call_path: String,
    usage_path: String,
    tool_use: Option<ToolUseMapping>,
}

impl PathEventMapper {
    pub fn new(
        content_path: Option<String>,
        tool_call_path: Option<String>,
        usage_path: Option<String>,
        tool_use: Option<ToolUseMapping>,
    ) -> Self {
        Self {
            content_path: content_path.unwrap_or_else(|| "$.choices[0].delta.content".to_string()),
            tool_call_path: tool_call_path
                .unwrap_or_else(|| "$.choices[0].delta.tool_calls".to_string()),
            usage_path: usage_path.unwrap_or_else(|| "$.usage".to_string()),
            tool_use,
        }
    }
}

fn debug_toolcall_enabled() -> bool {
    std::env::var("AI_LIB_DEBUG_TOOLCALL").ok().as_deref() == Some("1")
}

fn extract_toolcall_id(tc: &Value) -> Option<String> {
    crate::utils::PathMapper::get_string(tc, "id")
        .or_else(|| crate::utils::PathMapper::get_string(tc, "tool_call_id"))
        .or_else(|| crate::utils::PathMapper::get_string(tc, "delta.id"))
        .or_else(|| crate::utils::PathMapper::get_string(tc, "delta.tool_call_id"))
}

fn extract_toolcall_name(tc: &Value) -> Option<String> {
    crate::utils::PathMapper::get_string(tc, "function.name")
        .or_else(|| crate::utils::PathMapper::get_string(tc, "name"))
        .or_else(|| crate::utils::PathMapper::get_string(tc, "delta.function.name"))
        .or_else(|| crate::utils::PathMapper::get_string(tc, "delta.name"))
}

fn extract_toolcall_arguments(tc: &Value) -> Option<String> {
    // Common variants:
    // - function.arguments: string
    // - arguments: string
    // - delta.function.arguments: string
    // - delta.arguments: string
    // Sometimes providers may emit object already; stringify it.
    if let Some(v) = crate::utils::PathMapper::get_path(tc, "function.arguments") {
        if let Some(s) = v.as_str() {
            return Some(s.to_string());
        }
        if v.is_object() || v.is_array() {
            return serde_json::to_string(v).ok();
        }
    }
    if let Some(v) = crate::utils::PathMapper::get_path(tc, "arguments") {
        if let Some(s) = v.as_str() {
            return Some(s.to_string());
        }
        if v.is_object() || v.is_array() {
            return serde_json::to_string(v).ok();
        }
    }
    if let Some(v) = crate::utils::PathMapper::get_path(tc, "delta.function.arguments") {
        if let Some(s) = v.as_str() {
            return Some(s.to_string());
        }
        if v.is_object() || v.is_array() {
            return serde_json::to_string(v).ok();
        }
    }
    if let Some(v) = crate::utils::PathMapper::get_path(tc, "delta.arguments") {
        if let Some(s) = v.as_str() {
            return Some(s.to_string());
        }
        if v.is_object() || v.is_array() {
            return serde_json::to_string(v).ok();
        }
    }
    None
}

fn extract_by_tooling(
    tc: &Value,
    tool_use: &ToolUseMapping,
) -> (Option<String>, Option<String>, Option<String>) {
    let id = tool_use
        .id_path
        .as_deref()
        .and_then(|p| crate::utils::PathMapper::get_string(tc, p));
    let name = tool_use
        .name_path
        .as_deref()
        .and_then(|p| crate::utils::PathMapper::get_string(tc, p));
    let args = tool_use.input_path.as_deref().and_then(|p| {
        let v = crate::utils::PathMapper::get_path(tc, p)?;
        if let Some(s) = v.as_str() {
            Some(s.to_string())
        } else if v.is_object() || v.is_array() {
            serde_json::to_string(v).ok()
        } else {
            serde_json::to_string(v).ok()
        }
    });
    (id, name, args)
}

#[async_trait::async_trait]
impl Mapper for PathEventMapper {
    async fn map(
        &self,
        input: BoxStream<'static, Value>,
    ) -> PipeResult<BoxStream<'static, StreamingEvent>> {
        let content_path = self.content_path.clone();
        let tool_call_path = self.tool_call_path.clone();
        let usage_path = self.usage_path.clone();
        let tool_use = self.tool_use.clone();

        // State is local to each stream to avoid cross-request contamination.
        let stream = stream::unfold(
            (
                input,
                VecDeque::<StreamingEvent>::new(),
                false,
                HashSet::<String>::new(),
                HashMap::<u32, String>::new(),
            ),
            move |(mut input, mut q, mut ended, mut started_ids, mut index_to_id)| {
                let content_path = content_path.clone();
                let tool_call_path = tool_call_path.clone();
                let usage_path = usage_path.clone();
                let tool_use = tool_use.clone();
                async move {
                    if let Some(ev) = q.pop_front() {
                        return Some((Ok(ev), (input, q, ended, started_ids, index_to_id)));
                    }
                    if ended {
                        return None;
                    }

                    while let Some(item) = input.next().await {
                        match item {
                            Ok(frame) => {
                                // content delta
                                if let Some(content) =
                                    crate::utils::PathMapper::get_string(&frame, &content_path)
                                {
                                    if !content.is_empty() {
                                        q.push_back(StreamingEvent::PartialContentDelta {
                                            content,
                                            sequence_id: None,
                                        });
                                    }
                                }

                                // usage
                                if let Some(usage) =
                                    crate::utils::PathMapper::get_path(&frame, &usage_path).cloned()
                                {
                                    q.push_back(StreamingEvent::Metadata {
                                        usage: Some(usage),
                                        finish_reason: None,
                                        stop_reason: None,
                                    });
                                }

                                // tool calls (OpenAI delta style)
                                if let Some(tc_val) =
                                    crate::utils::PathMapper::get_path(&frame, &tool_call_path)
                                {
                                    if debug_toolcall_enabled() {
                                        debug!(
                                            tool_call_path = tool_call_path.as_str(),
                                            tool_call_delta = %tc_val,
                                            frame = %frame,
                                            "tool_call delta observed"
                                        );
                                    }
                                    if let Some(arr) = tc_val.as_array() {
                                        for (idx, tc) in arr.iter().enumerate() {
                                            // Determine tool-call index (some providers omit id on subsequent deltas)
                                            let tc_index: u32 =
                                                crate::utils::PathMapper::get_path(tc, "index")
                                                    .and_then(|v| v.as_u64())
                                                    .map(|v| v as u32)
                                                    .unwrap_or(idx as u32);

                                            // Prefer protocol tooling mapping if present
                                            let (mut id, mut name, mut args) =
                                                if let Some(ref tu) = tool_use {
                                                    extract_by_tooling(tc, tu)
                                                } else {
                                                    (None, None, None)
                                                };

                                            // Fallback to openai-style variants if tooling mapping didn't yield values
                                            if id.is_none() {
                                                id = extract_toolcall_id(tc);
                                            }
                                            if name.is_none() {
                                                name = extract_toolcall_name(tc);
                                            }
                                            if args.is_none() {
                                                args = extract_toolcall_arguments(tc);
                                            }

                                            // If we saw an id, remember it for this index
                                            if let Some(ref real_id) = id {
                                                index_to_id.insert(tc_index, real_id.clone());
                                            } else {
                                                // Otherwise, try to recover id from prior frames using index
                                                id = index_to_id.get(&tc_index).cloned();
                                            }

                                            if let (Some(id), Some(name)) =
                                                (id.clone(), name.clone())
                                            {
                                                if !started_ids.contains(&id) {
                                                    started_ids.insert(id.clone());
                                                    q.push_back(StreamingEvent::ToolCallStarted {
                                                        tool_call_id: id.clone(),
                                                        tool_name: name,
                                                        index: Some(tc_index),
                                                    });
                                                }
                                            }

                                            if let (Some(id), Some(arguments)) = (id, args) {
                                                q.push_back(StreamingEvent::PartialToolCall {
                                                    tool_call_id: id,
                                                    arguments,
                                                    index: Some(tc_index),
                                                    is_complete: None,
                                                });
                                            }
                                        }
                                    }
                                }

                                if let Some(ev) = q.pop_front() {
                                    return Some((
                                        Ok(ev),
                                        (input, q, ended, started_ids, index_to_id),
                                    ));
                                }
                                continue;
                            }
                            Err(e) => {
                                return Some((Err(e), (input, q, ended, started_ids, index_to_id)))
                            }
                        }
                    }

                    ended = true;
                    Some((
                        Ok(StreamingEvent::StreamEnd {
                            finish_reason: None,
                        }),
                        (input, q, ended, started_ids, index_to_id),
                    ))
                }
            },
        );

        Ok(Box::pin(stream))
    }
}
