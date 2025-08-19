# YAML Directives Support

rust-yaml fully supports YAML directives as defined in the YAML 1.2 specification. This includes both `%YAML` version directives and `%TAG` tag shorthand directives.

## Overview

YAML directives provide metadata about the document and establish tag shorthand notations. They appear at the beginning of a YAML document, before any content.

## Supported Directives

### %YAML Directive

The `%YAML` directive specifies the YAML version the document conforms to.

```yaml
%YAML 1.2
---
key: value
```

**Features:**

- Specifies YAML version (e.g., 1.2, 1.1)
- Persists across multiple documents in a stream
- Optional - documents without version directive default to YAML 1.2

### %TAG Directive

The `%TAG` directive establishes shorthand notations for tag prefixes.

```yaml
%TAG ! tag:example.com,2024:
%TAG !! tag:yaml.org,2002:
---
!widget
name: Button
!!str "explicitly typed string"
```

**Features:**

- Primary handle (`!`) for application-specific tags
- Secondary handle (`!!`) for standard YAML tags
- Named handles (e.g., `!e!`) for custom namespaces
- Applies only to the following document (not persistent)

## Usage Examples

### Parsing YAML with Directives

```rust
use rust_yaml::Yaml;

let yaml_content = r#"
%YAML 1.2
%TAG ! tag:example.com,2024:
---
key: value
nested:
  item: !custom data
"#;

let yaml = Yaml::new();
let result = yaml.load_str(yaml_content)?;
```

### Emitting YAML with Directives

```rust
use rust_yaml::{BasicEmitter, Emitter, Value};

let mut emitter = BasicEmitter::new();

// Set YAML version
emitter.set_yaml_version(1, 2);

// Add TAG directives
emitter.add_tag_directive("!".to_string(), "tag:example.com,2024:".to_string());
emitter.add_tag_directive("!!".to_string(), "tag:yaml.org,2002:".to_string());

// Emit the document
let value = Value::from(/* your data */);
let mut output = Vec::new();
emitter.emit(&value, &mut output)?;
```

## Directive Rules

### Document Boundaries

- Directives must appear before the document content
- Directives are separated from content by `---` (explicit) or the start of content (implicit)
- Multiple documents can have different directives

```yaml
%YAML 1.2
%TAG ! tag:example.com,2024:
---
doc1: with directives
...
%TAG ! tag:other.com,2024:
---
doc2: different tag namespace
```

### Persistence

- **%YAML directive**: Persists across all documents in the stream until overridden
- **%TAG directive**: Applies only to the immediately following document

### Tag Resolution

When TAG directives are defined:

- `!foo` → Expands using primary handle prefix
- `!!str` → Expands using secondary handle prefix  
- `!e!widget` → Expands using named handle prefix
- `!<tag:explicit.com,2024:type>` → Verbatim tag (no expansion)

## Implementation Details

### Scanner Level

The scanner recognizes directive tokens:

- `YamlDirective(major, minor)` - For %YAML directives
- `TagDirective(handle, prefix)` - For %TAG directives

### Parser Level

The parser:

- Collects directives before document start
- Attaches directives to `DocumentStart` events
- Maintains YAML version across documents
- Resets TAG directives after each document

### Composer Level

The composer:

- Preserves directive information for round-trip support
- Handles version-specific type resolution (future enhancement)

### Emitter Level

The emitter:

- Can emit directives before document content
- Preserves directive formatting
- Ensures proper document markers when directives are present

## Best Practices

1. **Version Specification**: Use `%YAML 1.2` for documents requiring strict YAML 1.2 compliance

2. **Tag Namespaces**: Define clear, URI-based tag namespaces for custom types:

   ```yaml
   %TAG ! tag:myapp.com,2024:
   ```

3. **Round-trip Preservation**: When modifying YAML documents, preserve original directives:

   ```rust
   // Parse with directive preservation
   let parsed = yaml.load_str(input)?;

   // Emit with same directives
   let mut emitter = BasicEmitter::new();
   emitter.set_yaml_version(1, 2);
   emitter.emit(&parsed, &mut output)?;
   ```

4. **Multi-document Streams**: Be aware of directive scope in multi-document files

## Limitations

Currently, rust-yaml:

- Parses and preserves directives but doesn't enforce version-specific behavior differences
- Doesn't fully resolve custom tags to type handlers (planned enhancement)
- Supports common tag handles (!,  !!, !name!)

## Future Enhancements

Planned improvements for directive support:

- [ ] Full tag resolution with custom type handlers
- [ ] YAML 1.1 vs 1.2 type resolution differences
- [ ] Schema validation based on tag directives
- [ ] Warning/error on unsupported version numbers

## Examples

See the `tests/directives.rs` and `tests/directive_roundtrip.rs` test files for comprehensive examples of directive usage.
