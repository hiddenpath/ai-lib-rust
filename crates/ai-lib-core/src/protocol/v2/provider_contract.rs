//! ProviderContract types — content_block_mapping for manifest-driven encoding.
//!
//! PT-079 / ALR-DOC-002：与 ai-protocol `schemas/v2/provider-contract.json` 对齐的子集。

use serde::Deserialize;
use std::collections::HashMap;

/// Parsed ProviderContract (encoding-relevant fields).
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderContract {
    pub contract_version: String,
    pub provider_id: String,
    pub api_style: String,
    pub request_mapping: RequestMappingContract,
}

/// Request mapping section used by manifest encoder.
#[derive(Debug, Clone, Deserialize)]
pub struct RequestMappingContract {
    pub message_format: String,
    #[serde(default)]
    pub role_mapping: HashMap<String, String>,
    #[serde(default)]
    pub content_block_mapping: Option<ContentBlockMapping>,
}

/// Declarative ContentBlock → wire JSON mapping.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ContentBlockMapping {
    #[serde(default)]
    pub text: Option<TextBlockMapping>,
    #[serde(default)]
    pub image: Option<ImageBlockMapping>,
    #[serde(default)]
    pub document: Option<DocumentBlockMapping>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TextBlockMapping {
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default)]
    pub wrapper: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImageBlockMapping {
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub base64_field: Option<String>,
    #[serde(default)]
    pub url_field: Option<String>,
}

/// Document block mapping per PT-079-R1 schema.
#[derive(Debug, Clone, Deserialize)]
pub struct DocumentBlockMapping {
    pub format: String,
    #[serde(default)]
    pub type_field: Option<String>,
    #[serde(default)]
    pub default_mime_type: Option<String>,
    #[serde(default)]
    pub ref_resolution: Option<String>,
}

impl DocumentBlockMapping {
    pub fn default_mime(&self) -> &str {
        self.default_mime_type
            .as_deref()
            .unwrap_or("application/pdf")
    }

    pub fn rejects_ref_before_encode(&self) -> bool {
        self.ref_resolution
            .as_deref()
            .unwrap_or("error_before_encode")
            == "error_before_encode"
    }
}

#[cfg(test)]
mod tests {
    use super::DocumentBlockMapping;
    use crate::protocol::v2::contracts;

    #[test]
    fn anthropic_contract_has_document_mapping() {
        let contract = contracts::anthropic_messages_contract().unwrap();
        let mapping = contract
            .request_mapping
            .content_block_mapping
            .as_ref()
            .and_then(|m| m.document.as_ref())
            .expect("document mapping");
        assert_eq!(mapping.format, "anthropic_document");
        assert!(mapping.rejects_ref_before_encode());
    }

    #[test]
    fn gemini_contract_has_document_mapping() {
        let contract = contracts::gemini_generate_contract().unwrap();
        let mapping = contract
            .request_mapping
            .content_block_mapping
            .as_ref()
            .and_then(|m| m.document.as_ref())
            .expect("document mapping");
        assert_eq!(mapping.format, "gemini_inline_data");
    }
}
