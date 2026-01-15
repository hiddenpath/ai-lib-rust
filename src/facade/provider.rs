use crate::{AiClient, AiClientBuilder, Result};

/// Provider identifier facade for developer ergonomics.
///
/// This does NOT replace manifest-first configuration. It simply helps construct model strings
/// like `"provider/model"` consistently, and can optionally provide a default model via env.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Provider {
    OpenAI,
    Anthropic,
    Gemini,
    Groq,
    DeepSeek,
    OpenRouter,
    Ollama,
    /// Arbitrary provider id (matches `provider_id` / manifest id).
    Custom(String),
}

impl Provider {
    pub fn id(&self) -> &str {
        match self {
            Provider::OpenAI => "openai",
            Provider::Anthropic => "anthropic",
            Provider::Gemini => "gemini",
            Provider::Groq => "groq",
            Provider::DeepSeek => "deepseek",
            Provider::OpenRouter => "openrouter",
            Provider::Ollama => "ollama",
            Provider::Custom(s) => s.as_str(),
        }
    }

    /// Construct a full model reference string `"provider/model"`.
    pub fn model(&self, model: impl AsRef<str>) -> ModelRef {
        ModelRef::new(self.clone(), model.as_ref().to_string())
    }

    /// Default model name for this provider (best-effort).
    ///
    /// Precedence:
    /// 1) `AI_LIB_DEFAULT_MODEL_<PROVIDER_ID_UPPER>`
    /// 2) a conservative built-in default (only when we have a known good example)
    pub fn default_model_name(&self) -> Option<String> {
        let key = format!(
            "AI_LIB_DEFAULT_MODEL_{}",
            self.id().replace('-', "_").to_uppercase()
        );
        if let Ok(v) = std::env::var(key) {
            let v = v.trim().to_string();
            if !v.is_empty() {
                return Some(v);
            }
        }

        // Keep built-ins minimal to avoid provider-specific coupling.
        match self {
            Provider::DeepSeek => Some("deepseek-chat".to_string()),
            _ => None,
        }
    }
}

/// A provider + model pair.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModelRef {
    provider: Provider,
    model: String,
}

impl ModelRef {
    pub fn new(provider: Provider, model: String) -> Self {
        Self { provider, model }
    }

    pub fn provider(&self) -> &Provider {
        &self.provider
    }

    pub fn model_name(&self) -> &str {
        &self.model
    }

    pub fn as_str(&self) -> String {
        format!("{}/{}", self.provider.id(), self.model)
    }

    /// Build an `AiClient` for this model reference using the default builder.
    pub async fn build_client(&self) -> Result<AiClient> {
        AiClient::new(&self.as_str()).await
    }

    /// Build an `AiClient` for this model reference using an existing builder.
    pub async fn build_client_with(&self, builder: AiClientBuilder) -> Result<AiClient> {
        builder.build(&self.as_str()).await
    }
}

/// Convenience: create a client from a provider, using provider default model.
pub async fn client_from_provider(provider: Provider) -> Result<AiClient> {
    let Some(model) = provider.default_model_name() else {
        return Err(crate::Error::validation(format!(
            "No default model for provider '{}'. Use Provider::model(\"...\") to specify one, or set AI_LIB_DEFAULT_MODEL_{}",
            provider.id(),
            provider.id().replace('-', "_").to_uppercase()
        )));
    };
    provider.model(model).build_client().await
}

