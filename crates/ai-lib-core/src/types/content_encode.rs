//! Provider-specific content block encoding — thin wrappers over manifest encoder.
//!
//! 多模态内容块编码；主路径委托 `manifest_encode`（PT-079 / ALR-DOC-002）。

use serde_json::Value;

use crate::error::Error;
use crate::types::manifest_encode::{
    encode_blocks_for_anthropic_contract, encode_blocks_for_gemini_contract,
};
use crate::types::message::ContentBlock;

/// Encode unified content blocks into Anthropic Messages API `content` array items.
pub fn encode_blocks_for_anthropic(blocks: &[ContentBlock]) -> Result<Vec<Value>, Error> {
    encode_blocks_for_anthropic_contract(blocks)
}

/// Encode unified content blocks into Gemini `parts` array.
pub fn encode_blocks_for_gemini(blocks: &[ContentBlock]) -> Result<Value, Error> {
    encode_blocks_for_gemini_contract(blocks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::message::ContentBlock;

    const PDF_B64: &str = "JVBERi0xLjQK";

    #[test]
    fn anthropic_document_base64_shape() {
        let blocks = vec![ContentBlock::document_base64(
            PDF_B64.into(),
            Some("application/pdf".into()),
            Some("paper.pdf".into()),
        )];
        let encoded = encode_blocks_for_anthropic(&blocks).unwrap();
        assert_eq!(encoded[0]["type"], "document");
        assert_eq!(encoded[0]["source"]["type"], "base64");
        assert_eq!(encoded[0]["source"]["media_type"], "application/pdf");
        assert_eq!(encoded[0]["source"]["data"], PDF_B64);
    }

    #[test]
    fn anthropic_document_ref_rejected() {
        let blocks = vec![ContentBlock::document_ref(
            "upload://abc".into(),
            Some("application/pdf".into()),
            Some("paper.pdf".into()),
        )];
        assert!(encode_blocks_for_anthropic(&blocks).is_err());
    }

    #[test]
    fn gemini_document_inline_data_shape() {
        let blocks = vec![
            ContentBlock::text("Summarize"),
            ContentBlock::document_base64(PDF_B64.into(), Some("application/pdf".into()), None),
        ];
        let parts = encode_blocks_for_gemini(&blocks).unwrap();
        let arr = parts.as_array().unwrap();
        assert_eq!(arr[0]["text"], "Summarize");
        assert_eq!(arr[1]["inlineData"]["mimeType"], "application/pdf");
        assert_eq!(arr[1]["inlineData"]["data"], PDF_B64);
    }

    #[test]
    fn gemini_document_ref_rejected() {
        let blocks = vec![ContentBlock::document_ref(
            "upload://abc".into(),
            Some("application/pdf".into()),
            None,
        )];
        assert!(encode_blocks_for_gemini(&blocks).is_err());
    }
}
