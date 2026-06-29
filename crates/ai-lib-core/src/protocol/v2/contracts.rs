//! Embedded ProviderContract YAML (synced from ai-protocol v2/contracts).
//!
//! 嵌入的 ProviderContract 真源；合规测试与 manifest encoder 共用。

use crate::error::Error;
use crate::protocol::v2::manifest::ApiStyle;
use crate::protocol::ProtocolError;

use super::provider_contract::ProviderContract;

const ANTHROPIC_MESSAGES_CONTRACT: &str = include_str!("embedded/anthropic-messages.contract.yaml");
const GEMINI_GENERATE_CONTRACT: &str = include_str!("embedded/gemini-generate.contract.yaml");

fn parse_contract(yaml: &str) -> Result<ProviderContract, Error> {
    serde_yaml::from_str(yaml).map_err(|e| {
        Error::Protocol(ProtocolError::ValidationError(format!(
            "invalid ProviderContract YAML: {e}"
        )))
    })
}

/// Load embedded Anthropic Messages contract.
pub fn anthropic_messages_contract() -> Result<ProviderContract, Error> {
    parse_contract(ANTHROPIC_MESSAGES_CONTRACT)
}

/// Load embedded Gemini generateContent contract.
pub fn gemini_generate_contract() -> Result<ProviderContract, Error> {
    parse_contract(GEMINI_GENERATE_CONTRACT)
}

/// Resolve embedded contract for a driver API style.
pub fn contract_for_api_style(style: ApiStyle) -> Result<ProviderContract, Error> {
    match style {
        ApiStyle::AnthropicMessages => anthropic_messages_contract(),
        ApiStyle::GeminiGenerate => gemini_generate_contract(),
        ApiStyle::OpenAiCompatible | ApiStyle::Custom => {
            Err(Error::Protocol(ProtocolError::ValidationError(format!(
                "no embedded ProviderContract for api_style {style}"
            ))))
        }
    }
}
