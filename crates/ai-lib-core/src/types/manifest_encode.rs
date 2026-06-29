//! Manifest-driven ContentBlock encoding (PT-079 / ALR-DOC-002).
//!
//! 按 ProviderContract `content_block_mapping` 将统一 ContentBlock 编码为厂商 wire JSON。

use serde_json::{json, Value};

use crate::error::Error;
use crate::protocol::v2::contracts;
use crate::protocol::v2::provider_contract::{DocumentBlockMapping, ProviderContract};
use crate::protocol::ProtocolError;
use crate::types::message::{ContentBlock, DocumentSource, ImageSource};

fn validation(msg: impl Into<String>) -> Error {
    Error::Protocol(ProtocolError::ValidationError(msg.into()))
}

fn document_mapping(contract: &ProviderContract) -> Result<&DocumentBlockMapping, Error> {
    contract
        .request_mapping
        .content_block_mapping
        .as_ref()
        .and_then(|m| m.document.as_ref())
        .ok_or_else(|| {
            validation(format!(
                "ProviderContract {} missing content_block_mapping.document",
                contract.provider_id
            ))
        })
}

/// Encode blocks using the embedded Anthropic Messages ProviderContract.
pub fn encode_blocks_anthropic(
    contract: &ProviderContract,
    blocks: &[ContentBlock],
) -> Result<Vec<Value>, Error> {
    if contract.api_style != "anthropic_messages" {
        return Err(validation(format!(
            "expected anthropic_messages contract, got {}",
            contract.api_style
        )));
    }
    let doc_mapping = document_mapping(contract)?;
    blocks
        .iter()
        .map(|block| encode_anthropic_block(block, doc_mapping))
        .collect()
}

/// Encode blocks using the embedded Gemini generateContent ProviderContract.
pub fn encode_blocks_gemini(
    contract: &ProviderContract,
    blocks: &[ContentBlock],
) -> Result<Value, Error> {
    if contract.api_style != "gemini_generate" {
        return Err(validation(format!(
            "expected gemini_generate contract, got {}",
            contract.api_style
        )));
    }
    let doc_mapping = document_mapping(contract)?;
    let parts: Vec<Value> = blocks
        .iter()
        .map(|block| encode_gemini_block(block, doc_mapping))
        .collect::<Result<_, _>>()?;
    Ok(Value::Array(parts))
}

/// Convenience: load embedded Anthropic contract and encode.
pub fn encode_blocks_for_anthropic_contract(blocks: &[ContentBlock]) -> Result<Vec<Value>, Error> {
    let contract = contracts::anthropic_messages_contract()?;
    encode_blocks_anthropic(&contract, blocks)
}

/// Convenience: load embedded Gemini contract and encode.
pub fn encode_blocks_for_gemini_contract(blocks: &[ContentBlock]) -> Result<Value, Error> {
    let contract = contracts::gemini_generate_contract()?;
    encode_blocks_gemini(&contract, blocks)
}

fn encode_anthropic_block(
    block: &ContentBlock,
    doc_mapping: &DocumentBlockMapping,
) -> Result<Value, Error> {
    match block {
        ContentBlock::Text { text } => Ok(json!({ "type": "text", "text": text })),
        ContentBlock::Image { source } => Ok(json!({
            "type": "image",
            "source": encode_anthropic_media_source(source, "image")?,
        })),
        ContentBlock::Document { source } => encode_anthropic_document(source, doc_mapping),
        ContentBlock::Audio { .. } => Err(validation(
            "Anthropic Messages driver does not encode audio content blocks yet",
        )),
        ContentBlock::ToolUse { .. } | ContentBlock::ToolResult { .. } => Err(validation(
            "tool blocks must be encoded via Anthropic tool_use/tool_result paths",
        )),
    }
}

fn encode_anthropic_document(
    source: &DocumentSource,
    mapping: &DocumentBlockMapping,
) -> Result<Value, Error> {
    if mapping.format != "anthropic_document" {
        return Err(validation(format!(
            "unsupported document format for Anthropic: {}",
            mapping.format
        )));
    }
    if source.source_type == "ref" && mapping.rejects_ref_before_encode() {
        return Err(validation(
            "document ref must be resolved to base64 or url before sending to Anthropic",
        ));
    }
    let type_field = mapping.type_field.as_deref().unwrap_or("document");
    Ok(json!({
        "type": type_field,
        "source": encode_anthropic_document_source(source, mapping)?,
    }))
}

fn encode_anthropic_media_source(source: &ImageSource, kind: &str) -> Result<Value, Error> {
    match source.source_type.as_str() {
        "base64" => {
            let media_type = source
                .media_type
                .as_deref()
                .ok_or_else(|| validation(format!("{kind} base64 block requires media_type")))?;
            Ok(json!({
                "type": "base64",
                "media_type": media_type,
                "data": source.data,
            }))
        }
        "url" => Ok(json!({
            "type": "url",
            "url": source.data,
        })),
        other => Err(validation(format!(
            "unsupported {kind} source type: {other}"
        ))),
    }
}

fn encode_anthropic_document_source(
    source: &DocumentSource,
    mapping: &DocumentBlockMapping,
) -> Result<Value, Error> {
    match source.source_type.as_str() {
        "base64" => {
            let media_type = source
                .mime_type
                .as_deref()
                .unwrap_or_else(|| mapping.default_mime());
            Ok(json!({
                "type": "base64",
                "media_type": media_type,
                "data": source.data,
            }))
        }
        "url" => Ok(json!({
            "type": "url",
            "url": source.data,
        })),
        "ref" => Err(validation(
            "document ref must be resolved to base64 or url before sending to Anthropic",
        )),
        other => Err(validation(format!(
            "unsupported document source type: {other}"
        ))),
    }
}

fn encode_gemini_block(
    block: &ContentBlock,
    doc_mapping: &DocumentBlockMapping,
) -> Result<Value, Error> {
    match block {
        ContentBlock::Text { text } => Ok(json!({ "text": text })),
        ContentBlock::Image { source } => encode_gemini_inline_data(source, "image", doc_mapping),
        ContentBlock::Document { source } => encode_gemini_document_inline(source, doc_mapping),
        ContentBlock::Audio { .. } => Err(validation(
            "Gemini generateContent driver does not encode audio content blocks yet",
        )),
        ContentBlock::ToolUse { .. } | ContentBlock::ToolResult { .. } => Err(validation(
            "tool blocks must be encoded via Gemini functionCall/functionResponse paths",
        )),
    }
}

fn encode_gemini_inline_data(
    source: &ImageSource,
    kind: &str,
    mapping: &DocumentBlockMapping,
) -> Result<Value, Error> {
    if mapping.format != "gemini_inline_data" {
        return Err(validation(format!(
            "unsupported image/document format for Gemini: {}",
            mapping.format
        )));
    }
    if source.source_type != "base64" {
        return Err(validation(format!(
            "Gemini {kind} blocks require base64 inline data (got {})",
            source.source_type
        )));
    }
    let mime_type = source
        .media_type
        .as_deref()
        .ok_or_else(|| validation(format!("{kind} base64 block requires media_type")))?;
    Ok(json!({
        "inlineData": {
            "mimeType": mime_type,
            "data": source.data,
        }
    }))
}

fn encode_gemini_document_inline(
    source: &DocumentSource,
    mapping: &DocumentBlockMapping,
) -> Result<Value, Error> {
    if mapping.format != "gemini_inline_data" {
        return Err(validation(format!(
            "unsupported document format for Gemini: {}",
            mapping.format
        )));
    }
    if source.source_type == "ref" && mapping.rejects_ref_before_encode() {
        return Err(validation(
            "Gemini document blocks require base64 inline data; resolve ref before send",
        ));
    }
    if source.source_type != "base64" {
        return Err(validation(
            "Gemini document blocks require base64 inline data; resolve ref before send",
        ));
    }
    let mime_type = source
        .mime_type
        .as_deref()
        .unwrap_or_else(|| mapping.default_mime());
    Ok(json!({
        "inlineData": {
            "mimeType": mime_type,
            "data": source.data,
        }
    }))
}
