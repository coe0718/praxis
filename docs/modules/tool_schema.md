# Tool Schema

> Tool schema generator — Zod-typed tool outputs for type safety. Generates JSON schemas for tools, enabling type-safe consumption.

## Overview

The `tool_schema` module converts Rust tool definitions into structured JSON schemas and TypeScript Zod validation schemas. It defines `ParameterSchema` (name, type, required flag, description, default value) and `ToolSchema` (name, description, parameters, generated Zod string). A `SchemaGenerator` struct provides static methods to generate schemas and convert them to OpenAPI-compatible JSON.

The Zod schema generator maps Rust types to Zod types:
- `string` → `z.string()`
- `number` → `z.number()`
- `boolean` → `z.boolean()`
- `array` → `z.array(z.any())`
- `object` → `z.record(z.any())`
- Everything else → `z.any()`

Required parameters are included directly in the Zod object; optional parameters get `.optional()`. Schema names are converted from snake_case to PascalCase via `to_pascal_case()`.

## Architecture

### Core Types

| Type | Description |
|------|-------------|
| `ParameterSchema` | Tool parameter definition: `name`, `param_type`, `required`, `description`, `default`. |
| `ToolSchema` | Complete tool schema: `name`, `description`, `parameters`, `zod_schema` (generated TypeScript Zod string). |
| `SchemaGenerator` | Static methods: `generate_tool_schema()`, `to_openapi()`. |

## Public API

```rust
// Parameter schema
pub struct ParameterSchema {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: Option<String>,
    pub default: Option<serde_json::Value>,
}

// Tool schema
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ParameterSchema>,
    pub zod_schema: String,
}

// Schema generator
pub struct SchemaGenerator;
impl SchemaGenerator {
    pub fn generate_tool_schema(
        name: &str,
        description: &str,
        params: Vec<(&str, &str, bool, Option<&str>)>,
    ) -> ToolSchema;

    pub fn to_openapi(&self, schema: &ToolSchema) -> serde_json::Value;
}
```

## Configuration

No `praxis.toml` section. Schemas are generated programmatically.

### Example

```rust
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

// Generated Zod schema
println!("{}", schema.zod_schema);
// Output:
// const FileReadSchema = z.object({
//   pathz.string()
//   max_bytesz.number().optional()
// })

// Convert to OpenAPI
let openapi = SchemaGenerator.to_openapi(&schema);
println!("{}", serde_json::to_string_pretty(&openapi).unwrap());
```

### OpenAPI Output

```json
{
  "name": "file-read",
  "description": "Read file contents",
  "parameters": {
    "type": "object",
    "properties": {
      "path": { "type": "string", "description": "File path to read" },
      "max_bytes": { "type": "number", "description": "Max bytes to read" }
    },
    "required": ["path"]
  }
}
```

## Dependencies

- `serde` / `serde_json` — serialization and JSON schema generation

## Source

`src/tool_schema.rs`