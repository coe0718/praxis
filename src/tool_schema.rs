//! Tool Schema Generator — Zod-typed tool outputs for type safety.
//!
//! Generates JSON schemas for tools, enabling type-safe consumption.
//! Inspired by OpenMolt's Zod-typed outputs.

use serde::{Deserialize, Serialize};
use serde_json::json;

/// JSON Schema for a tool parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSchema {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
    pub required: bool,
    pub description: Option<String>,
    pub default: Option<serde_json::Value>,
}

/// JSON Schema for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ParameterSchema>,
    /// Generated Zod schema for TypeScript consumers.
    pub zod_schema: String,
}

/// Schema generator for tools.
pub struct SchemaGenerator;

impl SchemaGenerator {
    /// Generate JSON Schema from a tool definition.
    pub fn generate_tool_schema(
        name: &str,
        description: &str,
        params: Vec<(&str, &str, bool, Option<&str>)>,
    ) -> ToolSchema {
        let param_schemas: Vec<ParameterSchema> = params
            .into_iter()
            .map(|(name, ptype, required, desc)| ParameterSchema {
                name: name.to_string(),
                param_type: ptype.to_string(),
                required,
                description: desc.map(|s| s.to_string()),
                default: None,
            })
            .collect();

        let zod_schema = Self::generate_zod_schema(name, &param_schemas);

        ToolSchema {
            name: name.to_string(),
            description: description.to_string(),
            parameters: param_schemas,
            zod_schema,
        }
    }

    /// Generate Zod schema string for TypeScript.
    fn generate_zod_schema(name: &str, params: &[ParameterSchema]) -> String {
        let mut fields = Vec::new();

        for p in params {
            let _zod_type = match p.param_type.as_str() {
                "string" => "z.string()",
                "number" => "z.number()",
                "boolean" => "z.boolean()",
                "array" => "z.array(z.any())",
                "object" => "z.record(z.any())",
                _ => "z.any()",
            };

            let field = if p.required {
                format!("  {}{}", name, p.name)
            } else {
                format!("  {}{}.optional()", name, p.name)
            };
            fields.push(field);
        }

        format!(
            "const {}Schema = z.object({{\n{}\n}})",
            Self::to_pascal_case(name),
            fields.join("\n")
        )
    }

    fn to_pascal_case(s: &str) -> String {
        s.split('_')
            .map(|w| {
                let mut chars = w.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect()
    }

    /// Generate OpenAPI-compatible schema.
    pub fn to_openapi(&self, schema: &ToolSchema) -> serde_json::Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for p in &schema.parameters {
            let prop = json!({
                "type": p.param_type,
                "description": p.description.clone().unwrap_or_default()
            });
            properties.insert(p.name.clone(), prop);

            if p.required {
                required.push(p.name.clone());
            }
        }

        json!({
            "name": schema.name,
            "description": schema.description,
            "parameters": {
                "type": "object",
                "properties": properties,
                "required": required
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_generation() {
        let schema = SchemaGenerator::generate_tool_schema(
            "file-read",
            "Read file contents",
            vec![
                ("path", "string", true, Some("File path to read")),
                ("max_bytes", "number", false, Some("Max bytes to read")),
            ],
        );

        assert_eq!(schema.name, "file-read");
        assert_eq!(schema.parameters.len(), 2);
        assert!(schema.zod_schema.contains("path"));
    }
}