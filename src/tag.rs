//! YAML tag resolution and handling system
//!
//! This module implements the full YAML 1.2 tag resolution mechanism,
//! including support for custom tag handlers and schema validation.

use crate::{Error, Result, Value};
use std::collections::HashMap;
use std::fmt;

/// Tag handle types as defined in YAML 1.2 spec
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TagHandle {
    /// Primary handle (!)
    Primary,
    /// Secondary handle (!!)
    Secondary,
    /// Named handle (e.g., !e!)
    Named(String),
    /// Verbatim tag (e.g., !<tag:example.com,2024:type>)
    Verbatim,
}

impl fmt::Display for TagHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Primary => write!(f, "!"),
            Self::Secondary => write!(f, "!!"),
            Self::Named(name) => write!(f, "!{}!", name),
            Self::Verbatim => write!(f, "!<>"),
        }
    }
}

/// A resolved YAML tag
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tag {
    /// The fully resolved tag URI
    pub uri: String,
    /// The original tag representation (for round-trip)
    pub original: String,
    /// Tag kind for quick identification
    pub kind: TagKind,
}

/// Tag kinds for quick type identification
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum TagKind {
    /// Core YAML types
    Null,
    Bool,
    Int,
    Float,
    Str,
    /// Collection types
    Seq,
    Map,
    /// Extended types
    Binary,
    Timestamp,
    Set,
    Omap,
    Pairs,
    /// Custom application type
    Custom(String),
}

/// Tag resolution context
pub struct TagResolver {
    /// Tag directives (handle -> prefix)
    directives: HashMap<String, String>,
    /// Custom tag handlers
    handlers: HashMap<String, Box<dyn TagHandler>>,
    /// Default schema
    schema: Schema,
}

impl fmt::Debug for TagResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TagResolver")
            .field("directives", &self.directives)
            .field("handlers_count", &self.handlers.len())
            .field("schema", &self.schema)
            .finish()
    }
}

impl TagResolver {
    /// Create a new tag resolver with default schema
    pub fn new() -> Self {
        Self::with_schema(Schema::Core)
    }

    /// Create a new tag resolver with specific schema
    pub fn with_schema(schema: Schema) -> Self {
        let mut resolver = Self {
            directives: HashMap::new(),
            handlers: HashMap::new(),
            schema,
        };

        // Initialize default tag directives
        resolver.directives.insert("!".to_string(), "!".to_string());
        resolver
            .directives
            .insert("!!".to_string(), "tag:yaml.org,2002:".to_string());

        resolver
    }

    /// Add a tag directive
    pub fn add_directive(&mut self, handle: String, prefix: String) {
        self.directives.insert(handle, prefix);
    }

    /// Clear all tag directives
    pub fn clear_directives(&mut self) {
        self.directives.clear();
        // Re-add defaults
        self.directives.insert("!".to_string(), "!".to_string());
        self.directives
            .insert("!!".to_string(), "tag:yaml.org,2002:".to_string());
    }

    /// Register a custom tag handler
    pub fn register_handler(&mut self, tag_uri: String, handler: Box<dyn TagHandler>) {
        self.handlers.insert(tag_uri, handler);
    }

    /// Resolve a tag string to a full Tag
    pub fn resolve(&self, tag_str: &str) -> Result<Tag> {
        let (uri, original) = if tag_str.starts_with("tag:") {
            // Already a full URI
            (tag_str.to_string(), tag_str.to_string())
        } else if tag_str.starts_with("!<") && tag_str.ends_with('>') {
            // Verbatim tag
            let uri = tag_str[2..tag_str.len() - 1].to_string();
            (uri, tag_str.to_string())
        } else if tag_str.starts_with("!!") {
            // Secondary handle
            let suffix = &tag_str[2..];
            let prefix = self
                .directives
                .get("!!")
                .cloned()
                .unwrap_or_else(|| "tag:yaml.org,2002:".to_string());
            (format!("{}{}", prefix, suffix), tag_str.to_string())
        } else if tag_str.starts_with('!') {
            // Check for named handle
            if let Some(end) = tag_str[1..].find('!') {
                let handle_name = &tag_str[1..end + 1];
                let handle = format!("!{}!", handle_name);
                let suffix = &tag_str[end + 2..];

                if let Some(prefix) = self.directives.get(&handle) {
                    (format!("{}{}", prefix, suffix), tag_str.to_string())
                } else {
                    // Unknown named handle, treat as primary
                    let prefix = self
                        .directives
                        .get("!")
                        .cloned()
                        .unwrap_or_else(|| "!".to_string());
                    (format!("{}{}", prefix, &tag_str[1..]), tag_str.to_string())
                }
            } else {
                // Primary handle
                let suffix = &tag_str[1..];
                let prefix = self
                    .directives
                    .get("!")
                    .cloned()
                    .unwrap_or_else(|| "!".to_string());
                (format!("{}{}", prefix, suffix), tag_str.to_string())
            }
        } else {
            // No tag prefix, use implicit tagging based on schema
            (
                self.schema.default_tag_for(tag_str),
                format!("!{}", tag_str),
            )
        };

        let kind = Self::identify_tag_kind(&uri);

        Ok(Tag {
            uri,
            original,
            kind,
        })
    }

    /// Identify the kind of tag from its URI
    fn identify_tag_kind(uri: &str) -> TagKind {
        match uri {
            "tag:yaml.org,2002:null" => TagKind::Null,
            "tag:yaml.org,2002:bool" => TagKind::Bool,
            "tag:yaml.org,2002:int" => TagKind::Int,
            "tag:yaml.org,2002:float" => TagKind::Float,
            "tag:yaml.org,2002:str" => TagKind::Str,
            "tag:yaml.org,2002:seq" => TagKind::Seq,
            "tag:yaml.org,2002:map" => TagKind::Map,
            "tag:yaml.org,2002:binary" => TagKind::Binary,
            "tag:yaml.org,2002:timestamp" => TagKind::Timestamp,
            "tag:yaml.org,2002:set" => TagKind::Set,
            "tag:yaml.org,2002:omap" => TagKind::Omap,
            "tag:yaml.org,2002:pairs" => TagKind::Pairs,
            _ => TagKind::Custom(uri.to_string()),
        }
    }

    /// Apply a tag to a value
    pub fn apply_tag(&self, tag: &Tag, value: &str) -> Result<Value> {
        // Check for custom handler first
        if let Some(handler) = self.handlers.get(&tag.uri) {
            return handler.construct(value);
        }

        // Use built-in tag handling
        match &tag.kind {
            TagKind::Null => Ok(Value::Null),
            TagKind::Bool => self.construct_bool(value),
            TagKind::Int => self.construct_int(value),
            TagKind::Float => self.construct_float(value),
            TagKind::Str => Ok(Value::String(value.to_string())),
            TagKind::Binary => self.construct_binary(value),
            TagKind::Timestamp => self.construct_timestamp(value),
            _ => Ok(Value::String(value.to_string())), // Default to string
        }
    }

    /// Construct a boolean from a tagged value
    fn construct_bool(&self, value: &str) -> Result<Value> {
        match value.to_lowercase().as_str() {
            "true" | "yes" | "on" => Ok(Value::Bool(true)),
            "false" | "no" | "off" => Ok(Value::Bool(false)),
            _ => Err(Error::Type {
                expected: "boolean".to_string(),
                found: format!("'{}'", value),
                position: crate::Position::start(),
                context: None,
            }),
        }
    }

    /// Construct an integer from a tagged value
    fn construct_int(&self, value: &str) -> Result<Value> {
        // Handle different integer formats
        let parsed = if value.starts_with("0x") || value.starts_with("0X") {
            // Hexadecimal
            i64::from_str_radix(&value[2..], 16)
        } else if value.starts_with("0o") || value.starts_with("0O") {
            // Octal
            i64::from_str_radix(&value[2..], 8)
        } else if value.starts_with("0b") || value.starts_with("0B") {
            // Binary
            i64::from_str_radix(&value[2..], 2)
        } else {
            // Decimal (with underscore support)
            value.replace('_', "").parse::<i64>()
        };

        parsed.map(Value::Int).map_err(|_| Error::Type {
            expected: "integer".to_string(),
            found: format!("'{}'", value),
            position: crate::Position::start(),
            context: None,
        })
    }

    /// Construct a float from a tagged value
    fn construct_float(&self, value: &str) -> Result<Value> {
        match value.to_lowercase().as_str() {
            ".inf" | "+.inf" => Ok(Value::Float(f64::INFINITY)),
            "-.inf" => Ok(Value::Float(f64::NEG_INFINITY)),
            ".nan" => Ok(Value::Float(f64::NAN)),
            _ => value
                .replace('_', "")
                .parse::<f64>()
                .map(Value::Float)
                .map_err(|_| Error::Type {
                    expected: "float".to_string(),
                    found: format!("'{}'", value),
                    position: crate::Position::start(),
                    context: None,
                }),
        }
    }

    /// Construct binary data from a tagged value (base64)
    fn construct_binary(&self, value: &str) -> Result<Value> {
        use base64::{engine::general_purpose::STANDARD, Engine as _};

        // Remove whitespace from base64 string
        let clean = value
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>();

        match STANDARD.decode(&clean) {
            Ok(bytes) => {
                // Try to convert to UTF-8 string, otherwise store as binary marker
                match String::from_utf8(bytes) {
                    Ok(s) => Ok(Value::String(s)),
                    Err(_) => Ok(Value::String(format!(
                        "[binary data: {} bytes]",
                        clean.len() / 4 * 3
                    ))),
                }
            }
            Err(_) => Err(Error::Type {
                expected: "base64-encoded binary".to_string(),
                found: format!("invalid base64: '{}'", value),
                position: crate::Position::start(),
                context: None,
            }),
        }
    }

    /// Construct a timestamp from a tagged value
    fn construct_timestamp(&self, value: &str) -> Result<Value> {
        // For now, just store as tagged string
        // A full implementation would parse ISO 8601 timestamps
        Ok(Value::String(format!("timestamp:{}", value)))
    }
}

impl Default for TagResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// YAML schemas define tag resolution rules
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Schema {
    /// Core schema (YAML 1.2)
    Core,
    /// JSON schema (subset of YAML)
    Json,
    /// Failsafe schema (minimal)
    Failsafe,
}

impl Schema {
    /// Get the default tag for untagged values based on schema
    pub fn default_tag_for(&self, _value: &str) -> String {
        match self {
            Self::Core => "tag:yaml.org,2002:str".to_string(),
            Self::Json => "tag:yaml.org,2002:str".to_string(),
            Self::Failsafe => "tag:yaml.org,2002:str".to_string(),
        }
    }

    /// Check if implicit typing is allowed
    pub fn allows_implicit_typing(&self) -> bool {
        match self {
            Self::Core => true,
            Self::Json => true,
            Self::Failsafe => false,
        }
    }
}

/// Trait for custom tag handlers
pub trait TagHandler: Send + Sync {
    /// Construct a value from the tagged string
    fn construct(&self, value: &str) -> Result<Value>;

    /// Represent a value as a string for this tag
    fn represent(&self, value: &Value) -> Result<String>;
}

/// Example custom tag handler for a Point type
pub struct PointTagHandler;

impl TagHandler for PointTagHandler {
    fn construct(&self, value: &str) -> Result<Value> {
        // Parse "x,y" format
        let parts: Vec<&str> = value.split(',').collect();
        if parts.len() != 2 {
            return Err(Error::Type {
                expected: "point (x,y)".to_string(),
                found: value.to_string(),
                position: crate::Position::start(),
                context: None,
            });
        }

        let x = parts[0].trim().parse::<f64>().map_err(|_| Error::Type {
            expected: "number".to_string(),
            found: parts[0].to_string(),
            position: crate::Position::start(),
            context: None,
        })?;

        let y = parts[1].trim().parse::<f64>().map_err(|_| Error::Type {
            expected: "number".to_string(),
            found: parts[1].to_string(),
            position: crate::Position::start(),
            context: None,
        })?;

        // Store as a sequence for now
        Ok(Value::Sequence(vec![Value::Float(x), Value::Float(y)]))
    }

    fn represent(&self, value: &Value) -> Result<String> {
        if let Value::Sequence(seq) = value {
            if seq.len() == 2 {
                if let (Some(Value::Float(x)), Some(Value::Float(y))) = (seq.get(0), seq.get(1)) {
                    return Ok(format!("{},{}", x, y));
                }
            }
        }
        Err(Error::Type {
            expected: "point sequence".to_string(),
            found: format!("{:?}", value),
            position: crate::Position::start(),
            context: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_resolution() {
        let mut resolver = TagResolver::new();

        // Test standard tags
        let tag = resolver.resolve("!!str").unwrap();
        assert_eq!(tag.uri, "tag:yaml.org,2002:str");
        assert_eq!(tag.kind, TagKind::Str);

        let tag = resolver.resolve("!!int").unwrap();
        assert_eq!(tag.uri, "tag:yaml.org,2002:int");
        assert_eq!(tag.kind, TagKind::Int);

        // Test primary handle
        resolver.add_directive("!".to_string(), "tag:example.com,2024:".to_string());
        let tag = resolver.resolve("!custom").unwrap();
        assert_eq!(tag.uri, "tag:example.com,2024:custom");

        // Test named handle
        resolver.add_directive("!e!".to_string(), "tag:example.com,2024:".to_string());
        let tag = resolver.resolve("!e!widget").unwrap();
        assert_eq!(tag.uri, "tag:example.com,2024:widget");

        // Test verbatim tag
        let tag = resolver.resolve("!<tag:explicit.com,2024:type>").unwrap();
        assert_eq!(tag.uri, "tag:explicit.com,2024:type");
    }

    #[test]
    fn test_tag_construction() {
        let resolver = TagResolver::new();

        // Test boolean construction
        let tag = Tag {
            uri: "tag:yaml.org,2002:bool".to_string(),
            original: "!!bool".to_string(),
            kind: TagKind::Bool,
        };

        assert_eq!(resolver.apply_tag(&tag, "true").unwrap(), Value::Bool(true));
        assert_eq!(
            resolver.apply_tag(&tag, "false").unwrap(),
            Value::Bool(false)
        );
        assert_eq!(resolver.apply_tag(&tag, "yes").unwrap(), Value::Bool(true));
        assert_eq!(resolver.apply_tag(&tag, "no").unwrap(), Value::Bool(false));

        // Test integer construction
        let tag = Tag {
            uri: "tag:yaml.org,2002:int".to_string(),
            original: "!!int".to_string(),
            kind: TagKind::Int,
        };

        assert_eq!(resolver.apply_tag(&tag, "42").unwrap(), Value::Int(42));
        assert_eq!(resolver.apply_tag(&tag, "0x2A").unwrap(), Value::Int(42));
        assert_eq!(resolver.apply_tag(&tag, "0o52").unwrap(), Value::Int(42));
        assert_eq!(
            resolver.apply_tag(&tag, "0b101010").unwrap(),
            Value::Int(42)
        );
        assert_eq!(resolver.apply_tag(&tag, "1_234").unwrap(), Value::Int(1234));

        // Test float construction
        let tag = Tag {
            uri: "tag:yaml.org,2002:float".to_string(),
            original: "!!float".to_string(),
            kind: TagKind::Float,
        };

        assert_eq!(
            resolver.apply_tag(&tag, "3.14").unwrap(),
            Value::Float(3.14)
        );
        assert_eq!(
            resolver.apply_tag(&tag, ".inf").unwrap(),
            Value::Float(f64::INFINITY)
        );
        assert_eq!(
            resolver.apply_tag(&tag, "-.inf").unwrap(),
            Value::Float(f64::NEG_INFINITY)
        );
        assert!(matches!(resolver.apply_tag(&tag, ".nan").unwrap(), Value::Float(f) if f.is_nan()));
    }

    #[test]
    fn test_custom_tag_handler() {
        let mut resolver = TagResolver::new();

        // Register custom point handler
        resolver.register_handler(
            "tag:example.com,2024:point".to_string(),
            Box::new(PointTagHandler),
        );

        // Resolve and apply custom tag
        resolver.add_directive("!".to_string(), "tag:example.com,2024:".to_string());
        let tag = resolver.resolve("!point").unwrap();

        let value = resolver.apply_tag(&tag, "3.5, 7.2").unwrap();
        assert_eq!(
            value,
            Value::Sequence(vec![Value::Float(3.5), Value::Float(7.2)])
        );
    }
}
