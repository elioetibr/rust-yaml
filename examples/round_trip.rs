//! Round-trip utility: reads a YAML file, parses it, then re-serializes it
//! to a file prefixed with "parsed_".
//!
//! Usage: cargo run --example round_trip -- <file.yaml> [file2.yaml ...]

use rust_yaml::Yaml;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        eprintln!("Usage: cargo run --example round_trip -- <file.yaml> [file2.yaml ...]");
        std::process::exit(1);
    }

    let yaml = Yaml::with_config(rust_yaml::YamlConfig {
        preserve_comments: true,
        loader_type: rust_yaml::LoaderType::RoundTrip,
        emit_anchors: false,
        indent: rust_yaml::IndentConfig {
            indent: 2,
            sequence_indent: Some(0),
            ..Default::default()
        },
        ..Default::default()
    });

    for path_str in &args {
        let path = Path::new(path_str);

        let file_name = path
            .file_name()
            .ok_or_else(|| format!("Invalid path: {}", path_str))?;

        let output_path = path
            .parent()
            .unwrap_or(Path::new("."))
            .join(format!("parsed_{}", file_name.to_string_lossy()));

        println!("--- Processing: {}", path.display());

        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

        match yaml.load_all_str(&content) {
            Ok(value) => match yaml.dump_all_str(&value) {
                Ok(serialized) => {
                    std::fs::write(&output_path, &serialized)
                        .map_err(|e| format!("Failed to write {}: {}", output_path.display(), e))?;
                    println!("    Parsed OK -> {}", output_path.display());
                }
                Err(e) => {
                    eprintln!("    DUMP ERROR: {}", e);
                }
            },
            Err(e) => {
                eprintln!("    PARSE ERROR: {}", e);
            }
        }
    }

    Ok(())
}
