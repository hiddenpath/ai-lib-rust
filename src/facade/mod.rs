//! Developer-friendly facade layer (optional).
//!
//! The core runtime remains manifest-first and protocol-driven. This facade provides
//! ergonomic helpers like `Provider` and `ModelRef` without hardcoding provider logic
//! into the core execution engine.

pub mod provider;
pub mod chat;
pub mod prelude;

