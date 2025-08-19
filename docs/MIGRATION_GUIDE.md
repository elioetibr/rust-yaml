# Migration Guide: Moving to rust-yaml

This guide helps you migrate from other Rust YAML libraries to rust-yaml, highlighting the benefits and providing practical examples.

## Quick Migration Reference

| From | To rust-yaml | Benefits |
|------|-------------|----------|
| `serde_yaml` (deprecated) | `rust_yaml` | Security, active maintenance, full YAML 1.2 |
| `yaml-rust` | `rust_yaml` | Better performance, security, modern API |
| `yaml-rust2` | `rust_yaml` | Enhanced security, complete spec support |
| `serde_yml` | `rust_yaml` | Full YAML features beyond serde scope |

## Migration from serde_yaml (Deprecated)

### ⚠️ **Critical: serde_yaml is deprecated and has unfixed security vulnerabilities**

### Before (Unsafe)

```rust
use serde_yaml::{self, Value};
use std::collections::HashMap;

// Basic parsing - NO security protection
let value: Value = serde_yaml::from_str(untrusted_input)?;

// Serialization
let output = serde_yaml::to_string(&value)?;

// Serde integration
#[derive(Serialize, Deserialize)]
struct Config {
    name: String,
    port: u16,
}
let config: Config = serde_yaml::from_str(input)?;
```

### After (Secure)

```rust
use rust_yaml::{Yaml, YamlConfig, Limits, Value};

// Secure parsing with protection against attacks
let config = YamlConfig {
    limits: Limits::strict(),  // Protects against billion laughs, deep nesting
    ..YamlConfig::default()
};
let yaml = Yaml::with_config(config);
let value = yaml.load_str(untrusted_input)?;

// Serialization with better formatting
let output = yaml.dump_str(&value)?;

// Serde integration (when needed)
use rust_yaml::serde_support;
#[derive(Serialize, Deserialize)]
struct Config {
    name: String,
    port: u16,
}
let config: Config = serde_support::from_yaml(&value)?;
```

#### Migration Benefits

- ✅ **Security**: Protection against all known YAML attacks
- ✅ **Maintenance**: Active development vs. deprecated library
- ✅ **Performance**: Better memory usage and parsing speed
- ✅ **Features**: Full YAML 1.2 support vs. limited subset

## Migration from yaml-rust

### Before

```rust
use yaml_rust::{YamlLoader, YamlEmitter, Yaml};
use std::io;

// Loading documents
let docs = YamlLoader::load_from_str(input)?;
let doc = &docs[0];

// Accessing values
let name = doc["config"]["name"].as_str().unwrap();
let port = doc["config"]["port"].as_i64().unwrap();

// Emitting
let mut out_str = String::new();
{
    let mut emitter = YamlEmitter::new(&mut out_str);
    emitter.dump(doc)?;
}
```

### After

```rust
use rust_yaml::{Yaml, Value};

// Loading with security - same API, better protection
let yaml = Yaml::new();
let docs = yaml.load_all_str(input)?;
let doc = &docs[0];

// Type-safe value access with better error handling
if let Value::Mapping(root) = doc {
    if let Some(Value::Mapping(config)) = root.get(&Value::String("config".to_string())) {
        if let Some(Value::String(name)) = config.get(&Value::String("name".to_string())) {
            // Safe string access
        }
        if let Some(Value::Int(port)) = config.get(&Value::String("port".to_string())) {
            // Safe integer access
        }
    }
}

// Simpler emitting with better formatting
let output = yaml.dump_str(doc)?;
```

#### Migration Benefits

- ✅ **Security**: Built-in protection vs. no security in yaml-rust
- ✅ **Error Handling**: Detailed error messages vs. basic errors
- ✅ **Performance**: Multiple optimization modes vs. single approach
- ✅ **API Safety**: Reduced unwrap() calls, better error propagation

#### Migration from yaml-rust2

### Before

```rust
use yaml_rust2::{YamlLoader, Yaml, YamlEmitter};

// Basic loading
let docs = YamlLoader::load_from_str(input)?;

// Hash-based access
let value = &docs[0]["key"];
```

### After

```rust
use rust_yaml::{Yaml, Value};

// Enhanced loading with security and performance options
let yaml = Yaml::new();
let docs = yaml.load_all_str(input)?;

// Type-safe access with comprehensive error handling
if let Some(Value::Mapping(map)) = docs.get(0) {
    if let Some(value) = map.get(&Value::String("key".to_string())) {
        // Process value safely
    }
}
```

#### Migration Benefits

- ✅ **Security**: Comprehensive protection vs. basic limits
- ✅ **Spec Compliance**: Full YAML 1.2 vs. subset implementation
- ✅ **Memory Efficiency**: Zero-copy options vs. standard allocation
- ✅ **Advanced Features**: Streaming, multiple composers, detailed configuration

## Migration from serde_yml

### Before

```rust
use serde_yml;
use serde::{Deserialize, Serialize};

// Serde-focused approach
#[derive(Serialize, Deserialize)]
struct Config {
    database: DatabaseConfig,
}

let config: Config = serde_yml::from_str(input)?;
let output = serde_yml::to_string(&config)?;
```

### After

```rust
use rust_yaml::{Yaml, YamlConfig, Limits, serde_support};
use serde::{Deserialize, Serialize};

// YAML-first approach with optional serde integration
let config = YamlConfig {
    limits: Limits::strict(),
    ..YamlConfig::default()
};
let yaml = Yaml::with_config(config);

// Option 1: Native YAML processing (recommended for complex YAML)
let value = yaml.load_str(input)?;
// Manipulate YAML structure directly
// value.insert_key(), value.add_comment(), etc.

// Option 2: Serde integration when needed
#[derive(Serialize, Deserialize)]
struct Config {
    database: DatabaseConfig,
}
let config: Config = serde_support::from_yaml(&value)?;
let output = yaml.dump_str(&serde_support::to_yaml(&config)?)?;
```

#### Migration Benefits

- ✅ **Flexibility**: Native YAML operations + serde when needed
- ✅ **Security**: Production-grade protection vs. basic validation
- ✅ **Features**: Full YAML 1.2 features vs. serde-compatible subset
- ✅ **Performance**: Multiple processing modes vs. single approach

## Common Migration Patterns

### 1. Configuration File Processing

#### Old Pattern (Various Libraries)

```rust
// Unsafe: No validation, vulnerable to attacks
let config: MyConfig = some_yaml_lib::from_str(&fs::read_to_string("config.yaml")?)?;
```

#### New Pattern (rust-yaml)

```rust
// Safe: Validated, protected, comprehensive error handling
let yaml_content = fs::read_to_string("config.yaml")?;
let config = YamlConfig {
    limits: Limits::strict(),
    loader_type: LoaderType::Safe,
    ..YamlConfig::default()
};
let yaml = Yaml::with_config(config);

match yaml.load_str(&yaml_content) {
    Ok(value) => {
        let config: MyConfig = serde_support::from_yaml(&value)?;
        // Config is safely loaded and validated
    }
    Err(e) => {
        eprintln!("Configuration error: {}", e);
        // Detailed error with line/column information
    }
}
```

### 2. API Input Processing

#### Old Pattern (Vulnerable)

```rust
// Web API endpoint - DANGEROUS with old libraries
#[post("/api/data")]
async fn process_yaml(body: String) -> Result<Json<Response>, Error> {
    let data: ApiData = serde_yaml::from_str(&body)?;  // ⚠️ VULNERABLE
    Ok(Json(Response::from(data)))
}
```

#### New Pattern (Secure)

```rust
// Web API endpoint - SECURE with rust-yaml
#[post("/api/data")]
async fn process_yaml(body: String) -> Result<Json<Response>, Error> {
    let config = YamlConfig {
        limits: Limits::strict(),           // Prevent attacks
        loader_type: LoaderType::Safe,      // Safe parsing only
        ..YamlConfig::default()
    };
    let yaml = Yaml::with_config(config);

    match yaml.load_str(&body) {
        Ok(value) => {
            let data: ApiData = serde_support::from_yaml(&value)?;
            Ok(Json(Response::from(data)))
        }
        Err(e) => {
            warn!("YAML attack attempted: {}", e);
            Err(Error::BadRequest("Invalid YAML format"))
        }
    }
}
```

### 3. Document Processing

#### Old Pattern (Limited)

```rust
// Basic document processing
let docs = YamlLoader::load_from_str(input)?;
for doc in docs {
    // Limited manipulation capabilities
    process_basic(&doc);
}
```

#### New Pattern (Advanced)

```rust
// Advanced document processing with streaming
let yaml = Yaml::new();
let docs = yaml.load_all_str(input)?;

for doc in docs {
    // Rich manipulation capabilities
    if let Value::Mapping(mut map) = doc {
        // Add metadata
        map.insert(
            Value::String("processed_at".to_string()),
            Value::String(chrono::Utc::now().to_rfc3339())
        );

        // Add comments
        // doc.add_comment("# Processed by rust-yaml");

        // Re-serialize with preserved formatting
        let output = yaml.dump_str(&Value::Mapping(map))?;
        println!("{}", output);
    }
}
```

## Performance Optimization After Migration

### Memory-Efficient Processing

```rust
// For large documents: use zero-copy composer
use rust_yaml::composer_borrowed::{ZeroCopyComposer, BorrowedComposer};

let mut composer = ZeroCopyComposer::new(large_yaml_content);
while composer.check_document() {
    if let Some(doc) = composer.compose_document()? {
        // Process document with minimal allocations
        process_borrowed(&doc);
    }
}
```

### Streaming for Very Large Files

```rust
// For huge files: use streaming parser
use rust_yaml::streaming::{StreamConfig, stream_from_string};

let config = StreamConfig::large_file();  // Optimized for big files
let parser = stream_from_string(huge_yaml_content, config);

for event in parser {
    match event? {
        Event { event_type: EventType::Scalar { value, .. }, .. } => {
            // Process each scalar as it's parsed
            process_scalar_streaming(&value);
        }
        _ => {}
    }
}
```

## Security Checklist After Migration

- ✅ **Input Validation**: Use `Limits::strict()` for untrusted input
- ✅ **Error Handling**: Catch and log potential attack attempts
- ✅ **Resource Monitoring**: Monitor parsing time and memory usage
- ✅ **Safe Loading**: Use `LoaderType::Safe` to disable dangerous features
- ✅ **Testing**: Test with malicious YAML samples to verify protection

## Getting Help

1. **Documentation**: Check the comprehensive API docs
2. **Examples**: Review the `examples/` directory for common patterns
3. **Migration Issues**: File issues on GitHub with "migration" tag
4. **Performance Questions**: Use the benchmark examples to validate performance
5. **Security Concerns**: Report any security findings responsibly

## Summary

Migrating to rust-yaml provides:

- 🔒 **Immediate Security**: Protection against all known YAML attacks
- 🚀 **Better Performance**: Multiple optimization strategies
- 📋 **Complete Features**: Full YAML 1.2 specification support
- 🔧 **Modern API**: Type-safe, error-resistant design
- 🛡️ **Future-Proof**: Active development and maintenance

The migration effort is minimal, but the security and feature benefits are substantial.
