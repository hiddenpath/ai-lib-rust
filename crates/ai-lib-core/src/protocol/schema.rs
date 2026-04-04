//! Protocol schema definitions and type mappings

use serde::{Deserialize, Serialize};

/// Protocol schema structure (for future schema validation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolSchema {
    pub version: String,
    pub definitions: SchemaDefinitions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDefinitions {
    // Schema definitions would go here
    // This is a placeholder for future schema validation enhancements
}
