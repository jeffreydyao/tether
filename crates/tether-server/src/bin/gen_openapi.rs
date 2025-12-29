//! Generates the OpenAPI specification to a JSON file.
//!
//! Run with: cargo run --bin gen-openapi -p tether-server
//!
//! The generated file is placed in the project root for consumption by:
//! - Web UI (for @hey-api/openapi-ts TypeScript client generation)
//! - MCP server (for rmcp-openapi tool generation)

use std::fs;
use std::path::PathBuf;

fn main() {
    println!("Generating OpenAPI specification...\n");

    // Generate the OpenAPI spec as JSON
    let json = tether_server::api::get_openapi_json();

    // Get project root (workspace root)
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("Could not find workspace root");

    // Output path in workspace root
    let output_path = workspace_root.join("openapi.json");

    // Write the spec
    fs::write(&output_path, &json)
        .unwrap_or_else(|e| panic!("Failed to write to {:?}: {}", output_path, e));

    println!("Written to: {}", output_path.display());

    // Parse to count paths and schemas
    if let Ok(spec) = serde_json::from_str::<serde_json::Value>(&json) {
        if let Some(paths) = spec.get("paths").and_then(|p| p.as_object()) {
            println!("Paths: {}", paths.len());
        }
        if let Some(components) = spec.get("components").and_then(|c| c.as_object()) {
            if let Some(schemas) = components.get("schemas").and_then(|s| s.as_object()) {
                println!("Schemas: {}", schemas.len());
            }
        }
    }

    println!("\nOpenAPI specification generated successfully!");
}
