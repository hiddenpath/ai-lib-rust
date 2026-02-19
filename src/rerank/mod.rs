//! Rerank（文档重排序）模块：通过 Provider API（如 Cohere Rerank）按相关性对文档重排序。

mod client;
mod types;

pub use client::{RerankerClient, RerankerClientBuilder};
pub use types::{RerankOptions, RerankResult};
