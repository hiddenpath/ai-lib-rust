//! Schema generation utilities.

use serde_json::json;

/// Generator for JSON schemas with customization options.
#[derive(Debug, Clone, Default)]
pub struct SchemaGenerator {
    title: Option<String>,
    description: Option<String>,
    properties: Vec<(String, serde_json::Value)>,
    required: Vec<String>,
    additional_properties: bool,
}

impl SchemaGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn add_property(mut self, name: impl Into<String>, schema: serde_json::Value) -> Self {
        self.properties.push((name.into(), schema));
        self
    }

    pub fn set_required(mut self, required: &[String]) -> Self {
        self.required = required.to_vec();
        self
    }

    pub fn set_additional_properties(mut self, additional: bool) -> Self {
        self.additional_properties = additional;
        self
    }

    pub fn build(self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        map.insert("type".into(), json!("object"));

        let mut properties = serde_json::Map::new();
        for (name, schema) in self.properties {
            properties.insert(name, schema);
        }
        map.insert("properties".into(), properties.into());

        if !self.required.is_empty() {
            map.insert("required".into(), self.required.into());
        }

        if !self.additional_properties {
            map.insert("additionalProperties".into(), json!(false));
        }

        if let Some(title) = self.title {
            map.insert("title".into(), title.into());
        }
        if let Some(desc) = self.description {
            map.insert("description".into(), desc.into());
        }

        map.into()
    }
}

pub fn schema_from_type_name(type_name: &str) -> serde_json::Value {
    match type_name {
        "string" => json!({"type": "string"}),
        "integer" => json!({"type": "integer"}),
        "number" => json!({"type": "number"}),
        "boolean" => json!({"type": "boolean"}),
        "array" => json!({"type": "array"}),
        "object" => json!({"type": "object"}),
        "null" => json!({"type": "null"}),
        _ => json!({"type": "object"}),
    }
}

pub fn json_schema_from_type<T: schemars::JsonSchema>() -> serde_json::Value {
    let schema = schemars::schema_for!(T);
    serde_json::to_value(&schema).unwrap_or_else(|_| json!({}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_generator_basic() {
        let generator = SchemaGenerator::new()
            .add_property("name", json!({"type": "string"}))
            .add_property("age", json!({"type": "integer"}));

        let schema = generator.build();
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["name"]["type"], "string");
        assert_eq!(schema["properties"]["age"]["type"], "integer");
    }

    #[test]
    fn test_schema_generator_with_required() {
        let generator = SchemaGenerator::new()
            .add_property("name", json!({"type": "string"}))
            .set_required(&vec!["name".to_string()]);

        let schema = generator.build();
        assert!(schema["required"].is_array());
        assert_eq!(schema["required"][0], "name");
    }

    #[test]
    fn test_schema_generator_additional_properties_false() {
        let generator = SchemaGenerator::new().set_additional_properties(false);
        let schema = generator.build();
        assert_eq!(schema["additionalProperties"], false);
    }

    #[test]
    fn test_schema_from_type_name() {
        let string_schema = schema_from_type_name("string");
        assert_eq!(string_schema["type"], "string");

        let integer_schema = schema_from_type_name("integer");
        assert_eq!(integer_schema["type"], "integer");

        let array_schema = schema_from_type_name("array");
        assert_eq!(array_schema["type"], "array");
    }
}
