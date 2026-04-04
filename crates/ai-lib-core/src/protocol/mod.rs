//! 协议规范层：负责加载、验证和管理 AI-Protocol 规范文件。
//!
//! # Protocol Specification Layer
//!
//! This module handles loading, validating, and managing AI-Protocol specifications.
//! It provides the foundation for the protocol-driven architecture where all provider
//! behaviors are defined declaratively rather than through hardcoded logic.
//!
//! ## Overview
//!
//! The protocol layer is responsible for:
//! - Loading protocol manifests from various sources (local files, URLs, GitHub)
//! - Validating manifests against the AI-Protocol JSON Schema
//! - Providing structured access to protocol configuration
//! - Managing authentication, streaming, and endpoint configurations
//!
//! ## Module Structure
//!
//! | Module | Description |
//! |--------|-------------|
//! | [`loader`] | Protocol loading from local files, remote URLs, and GitHub |
//! | [`schema`] | Protocol schema definitions and versioning |
//! | [`validator`] | Protocol validation using JSON Schema |
//! | [`manifest`] | Protocol manifest structure and operations |
//! | [`config`] | Configuration structures (streaming, auth, endpoints) |
//! | [`error`] | Protocol-specific error types |
//! | [`request`] | Unified request format for cross-provider compatibility |
//!
//! ## Example
//!
//! ```rust,no_run
//! use ai_lib_core::protocol::{ProtocolLoader, ProtocolValidator};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load a protocol manifest
//!     let loader = ProtocolLoader::new();
//!     let manifest = loader.load_provider("openai").await?;
//!     
//!     // Validate the manifest
//!     let validator = ProtocolValidator::new()?;
//!     validator.validate(&manifest)?;
//!     
//!     println!(
//!         "Protocol: {} v{}",
//!         manifest.name.as_deref().unwrap_or(&manifest.id),
//!         manifest.version.as_deref().unwrap_or("unknown")
//!     );
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod error;
#[cfg(not(target_arch = "wasm32"))]
pub mod loader;
/// In-memory YAML manifest parse + validate (WASI and host; no async / remote loader).
pub mod wasm_manifest;
pub mod manifest;
pub mod request;
pub mod schema;
pub mod v2;
pub mod validator;

// Re-export main types for convenient access
pub use config::*;
pub use error::ProtocolError;
#[cfg(not(target_arch = "wasm32"))]
pub use loader::ProtocolLoader;
pub use wasm_manifest::load_manifest_validated;
pub use manifest::ProtocolManifest;
pub use request::UnifiedRequest;
pub use schema::ProtocolSchema;
pub use v2::{CapabilitiesV2, Capability, FeatureFlags, ManifestV2};
pub use validator::ProtocolValidator;
