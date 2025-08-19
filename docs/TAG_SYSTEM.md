# YAML Tag System Documentation

This document describes the comprehensive tag system implemented in `rust-yaml` that provides full YAML 1.2 tag support with custom handlers and schema validation.

## Overview

The tag system in `rust-yaml` implements the complete YAML 1.2 tag resolution mechanism, allowing:

- **Full tag resolution**: Supports all YAML tag handle types (primary, secondary, named, verbatim)
- **Custom tag handlers**: Extensible system for application-specific types
- **Schema validation**: Support for Core, JSON, and Failsafe schemas
- **Binary data**: Built-in support for base64-encoded binary data
- **Tag directives**: Full support for %TAG directives

## Core Components

### TagResolver

The `TagResolver` is the central component that handles tag resolution and value construction:

```rust
use rust_yaml::tag::{TagResolver, Schema};

// Create with default Core schema
let resolver = TagResolver::new();

// Create with specific schema
let resolver = TagResolver::with_schema(Schema::Json);
```

### Tag Types

#### Primary Handle (!)

Used for application-specific types:

```yaml
!person
name: John Doe
age: 30
```

#### Secondary Handle (!!)

Used for standard YAML types:

```yaml
!!str 123        # Forces 123 to be a string
!!int "456"      # Forces "456" to be an integer
!!null anything  # Forces value to be null
```

#### Named Handles (!name!)

Custom handles defined by %TAG directives:

```yaml
%TAG !ex! tag:example.com,2024:
---
!ex!widget
  id: 123
```

#### Verbatim Tags (!<uri>)

Direct tag URIs:

```yaml
!<tag:example.com,2024:person>
name: John Doe
```

## Standard YAML Tags

### Core Schema Types

- `!!str` - String values
- `!!int` - Integer values
- `!!float` - Floating-point values
- `!!bool` - Boolean values (true/false, yes/no, on/off)
- `!!null` - Null values (null, ~)
- `!!seq` - Sequences (arrays)
- `!!map` - Mappings (objects)

### Extended Types

- `!!binary` - Base64-encoded binary data
- `!!timestamp` - Date/time values
- `!!set` - Sets (unique values)
- `!!omap` - Ordered mappings
- `!!pairs` - Key-value pairs

## Usage Examples

### Basic Type Coercion

```rust
use rust_yaml::Yaml;

let yaml = Yaml::new();

// Force string type
let yaml_str = r#"
number: !!str 123
"#;
let result = yaml.load_str(yaml_str).unwrap();
// number is String("123"), not Int(123)

// Force integer type
let yaml_str = r#"
value: !!int "456"
"#;
let result = yaml.load_str(yaml_str).unwrap();
// value is Int(456), not String("456")
```

### Binary Data

```rust
// Base64-encoded binary data
let yaml_str = r#"
data: !!binary |
  SGVsbG8gV29ybGQh
"#;
let result = yaml.load_str(yaml_str).unwrap();
// Automatically decodes to "Hello World!"
```

### Tag Directives

```yaml
%TAG ! tag:example.com,2024:
%TAG !! tag:yaml.org,2002:
---
!person
  name: !!str John Doe
  age: !!int "30"
```

### Multiple Documents with Different Tags

```yaml
%TAG ! tag:example.com,2024:
---
!widget
  id: 1
...
%TAG ! tag:another.com,2025:
---
!gadget
  id: 2
```

## Custom Tag Handlers

You can extend the tag system with custom handlers for application-specific types:

```rust
use rust_yaml::tag::{TagHandler, TagResolver, Tag};
use rust_yaml::{Result, Value};

struct PersonHandler;

impl TagHandler for PersonHandler {
    fn construct(&self, tag: &Tag, value: &str) -> Result<Value> {
        // Parse person data from YAML string
        // Return custom Person type as Value
        todo!("Implement person construction")
    }
}

// Register custom handler
let mut resolver = TagResolver::new();
resolver.add_handler("tag:example.com,2024:person".to_string(), Box::new(PersonHandler));
```

## Schema Types

### Core Schema (Default)

- Supports all YAML 1.2 types
- Boolean values: `true`, `false`, `yes`, `no`, `on`, `off`
- Null values: `null`, `~`

### JSON Schema

- More restrictive, JSON-compatible types only
- Boolean values: `true`, `false` only
- Null values: `null` only

### Failsafe Schema

- Most basic schema
- Only strings, sequences, and mappings
- No type coercion

```rust
use rust_yaml::tag::Schema;

let resolver = TagResolver::with_schema(Schema::Json);
```

## Tag Resolution Process

1. **Tag Scanning**: Scanner identifies tag tokens (`!`, `!!`, `!name!`, `!<uri>`)
2. **Parser Integration**: Parser associates tags with nodes
3. **Directive Processing**: %TAG directives update resolver mappings
4. **Tag Resolution**: TagResolver expands tag handles to full URIs
5. **Value Construction**: Tagged values are constructed according to their types

## Advanced Features

### Anchor and Tag Combination

```yaml
base: &base !!str 123
ref: *base
```

Both `base` and `ref` will be strings due to the `!!str` tag.

### Complex Tag Directives

```yaml
%TAG !foo! tag:example.com,2024/foo:
%TAG !bar! tag:example.com,2024/bar:
---
config:
  !foo!widget:
    id: 123
  !bar!settings:
    theme: dark
```

### Error Handling

The tag system provides detailed error messages for:

- Invalid tag formats
- Unknown tag handles
- Malformed tag directives
- Custom handler errors

## Best Practices

1. **Use standard tags** when possible for interoperability
2. **Define tag directives** at document start for custom types
3. **Implement custom handlers** for complex application types
4. **Choose appropriate schema** for your use case
5. **Document custom tags** for maintainability

## Implementation Notes

- Tag resolution is performed during composition, not scanning
- Tags are stored with events and applied during value construction
- Custom handlers can return any Value type
- Schema validation ensures type consistency
- Tag directives scope is per-document

## Performance Considerations

- Tag resolution has minimal overhead for standard types
- Custom handlers may impact performance depending on implementation
- Schema choice affects type resolution performance
- Tag directive parsing is optimized for common patterns

This comprehensive tag system ensures full YAML 1.2 compliance while providing extensibility for application-specific needs.
