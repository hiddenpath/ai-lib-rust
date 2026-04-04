//! Embedding types and data structures.

use serde::{Deserialize, Serialize};

/// A single embedding vector with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub index: usize,
    pub vector: Vec<f32>,
    #[serde(default = "default_object_type")]
    pub object_type: String,
}

fn default_object_type() -> String {
    "embedding".to_string()
}

impl Embedding {
    pub fn new(index: usize, vector: Vec<f32>) -> Self {
        Self {
            index,
            vector,
            object_type: "embedding".to_string(),
        }
    }

    pub fn dimensions(&self) -> usize {
        self.vector.len()
    }
}

/// Request for generating embeddings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub input: EmbeddingInput,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EmbeddingInput {
    Single(String),
    Batch(Vec<String>),
}

impl EmbeddingRequest {
    pub fn single(model: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            input: EmbeddingInput::Single(text.into()),
            model: model.into(),
            dimensions: None,
            encoding_format: None,
            user: None,
        }
    }

    pub fn batch(model: impl Into<String>, texts: Vec<String>) -> Self {
        Self {
            input: EmbeddingInput::Batch(texts),
            model: model.into(),
            dimensions: None,
            encoding_format: None,
            user: None,
        }
    }

    pub fn with_dimensions(mut self, dimensions: usize) -> Self {
        self.dimensions = Some(dimensions);
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

impl EmbeddingUsage {
    pub fn new(prompt_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            total_tokens: prompt_tokens,
        }
    }

    pub fn add(&mut self, other: &EmbeddingUsage) {
        self.prompt_tokens += other.prompt_tokens;
        self.total_tokens += other.total_tokens;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub embeddings: Vec<Embedding>,
    pub model: String,
    pub usage: EmbeddingUsage,
    #[serde(default = "default_list_type")]
    pub object: String,
}

fn default_list_type() -> String {
    "list".to_string()
}

impl EmbeddingResponse {
    pub fn new(embeddings: Vec<Embedding>, model: String, usage: EmbeddingUsage) -> Self {
        Self {
            embeddings,
            model,
            usage,
            object: "list".to_string(),
        }
    }

    pub fn first(&self) -> Option<&Embedding> {
        self.embeddings.first()
    }
    pub fn len(&self) -> usize {
        self.embeddings.len()
    }
    pub fn is_empty(&self) -> bool {
        self.embeddings.is_empty()
    }

    pub fn from_openai_format(data: &serde_json::Value) -> crate::Result<Self> {
        let embeddings = data["data"]
            .as_array()
            .ok_or_else(|| crate::Error::parsing("Missing 'data' array"))?
            .iter()
            .map(|item| {
                let index = item["index"].as_u64().unwrap_or(0) as usize;
                let vector: Vec<f32> = item["embedding"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                            .collect()
                    })
                    .unwrap_or_default();
                Embedding::new(index, vector)
            })
            .collect();
        let model = data["model"].as_str().unwrap_or("unknown").to_string();
        let usage = EmbeddingUsage {
            prompt_tokens: data["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: data["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
        };
        Ok(Self::new(embeddings, model, usage))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingModel {
    pub id: String,
    pub name: String,
    pub max_input_tokens: u32,
    pub dimensions: usize,
    pub provider: String,
}

impl EmbeddingModel {
    pub fn text_embedding_3_small() -> Self {
        Self {
            id: "text-embedding-3-small".into(),
            name: "Text Embedding 3 Small".into(),
            max_input_tokens: 8191,
            dimensions: 1536,
            provider: "openai".into(),
        }
    }
    pub fn text_embedding_3_large() -> Self {
        Self {
            id: "text-embedding-3-large".into(),
            name: "Text Embedding 3 Large".into(),
            max_input_tokens: 8191,
            dimensions: 3072,
            provider: "openai".into(),
        }
    }
}
