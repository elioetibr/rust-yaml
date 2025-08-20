//! Main YAML API interface

use crate::{
    BasicEmitter, CommentPreservingConstructor, CommentedValue, Constructor, Emitter, Limits,
    Result, RoundTripConstructor, SafeConstructor, Schema, SchemaValidator, Value,
};
use std::io::{Read, Write};

/// Configuration for YAML processing
#[derive(Debug, Clone)]
pub struct YamlConfig {
    /// Type of loader/dumper to use
    pub loader_type: LoaderType,
    /// Whether to use pure Rust implementation (no C extensions)
    pub pure: bool,
    /// Whether to preserve quote styles during round-trip
    pub preserve_quotes: bool,
    /// Default flow style for output
    pub default_flow_style: Option<bool>,
    /// Whether to allow duplicate keys
    pub allow_duplicate_keys: bool,
    /// Text encoding to use
    pub encoding: String,
    /// Whether to add explicit document start markers
    pub explicit_start: Option<bool>,
    /// Whether to add explicit document end markers
    pub explicit_end: Option<bool>,
    /// Line width for output formatting
    pub width: Option<usize>,
    /// Whether to allow unicode characters
    pub allow_unicode: bool,
    /// Indentation settings
    pub indent: IndentConfig,
    /// Whether to preserve comments during round-trip operations
    pub preserve_comments: bool,
    /// Resource limits for secure processing
    pub limits: Limits,
    /// Enable safe mode (restricts dangerous features)
    pub safe_mode: bool,
    /// Enable strict mode (fail on ambiguous constructs)
    pub strict_mode: bool,
}

/// Type of YAML loader/dumper
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoaderType {
    /// Safe loader - only basic YAML types, no code execution
    Safe,
    /// Base loader - minimal type set
    Base,
    /// Round-trip loader - preserves formatting and comments (future)
    RoundTrip,
    /// Full loader - all features including potentially unsafe operations
    Full,
}

/// Indentation configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndentConfig {
    /// Base indentation
    pub indent: usize,
    /// Map indentation
    pub map_indent: Option<usize>,
    /// Sequence indentation
    pub sequence_indent: Option<usize>,
    /// Sequence dash offset
    pub sequence_dash_offset: usize,
}

impl Default for YamlConfig {
    fn default() -> Self {
        Self {
            loader_type: LoaderType::Safe,
            pure: true,
            preserve_quotes: false,
            default_flow_style: None,
            allow_duplicate_keys: false,
            encoding: "utf-8".to_string(),
            explicit_start: None,
            explicit_end: None,
            width: Some(80),
            allow_unicode: true,
            indent: IndentConfig::default(),
            preserve_comments: false,
            limits: Limits::default(),
            safe_mode: false,
            strict_mode: false,
        }
    }
}

impl YamlConfig {
    /// Creates a secure configuration for untrusted input
    pub fn secure() -> Self {
        Self {
            loader_type: LoaderType::Safe,
            pure: true,
            preserve_quotes: false,
            default_flow_style: None,
            allow_duplicate_keys: false,
            encoding: "utf-8".to_string(),
            explicit_start: None,
            explicit_end: None,
            width: Some(80),
            allow_unicode: true,
            indent: IndentConfig::default(),
            preserve_comments: false,
            limits: Limits::strict(),
            safe_mode: true,
            strict_mode: true,
        }
    }
}

impl Default for IndentConfig {
    fn default() -> Self {
        Self {
            indent: 2,
            map_indent: None,
            sequence_indent: None,
            sequence_dash_offset: 0,
        }
    }
}

/// Main YAML processing interface
#[derive(Debug, Clone)]
pub struct Yaml {
    config: YamlConfig,
}

impl Yaml {
    /// Create a new YAML processor with default configuration
    pub fn new() -> Self {
        Self {
            config: YamlConfig::default(),
        }
    }

    /// Create a new YAML processor with the specified loader type
    pub fn with_loader(loader_type: LoaderType) -> Self {
        let mut config = YamlConfig::default();
        config.loader_type = loader_type;
        Self { config }
    }

    /// Create a new YAML processor with custom configuration
    pub const fn with_config(config: YamlConfig) -> Self {
        Self { config }
    }

    /// Get the current configuration
    pub const fn config(&self) -> &YamlConfig {
        &self.config
    }

    /// Get a mutable reference to the configuration
    pub const fn config_mut(&mut self) -> &mut YamlConfig {
        &mut self.config
    }

    /// Load YAML from a string
    pub fn load_str(&self, input: &str) -> Result<Value> {
        self.load(input.as_bytes())
    }

    /// Load YAML from a reader
    pub fn load<R: Read>(&self, mut reader: R) -> Result<Value> {
        let mut buffer = String::new();
        reader.read_to_string(&mut buffer)?;

        // For now, return a placeholder implementation
        // This will be replaced with the actual parser implementation
        self.parse_yaml_string(&buffer)
    }

    /// Load all YAML documents from a string
    pub fn load_all_str(&self, input: &str) -> Result<Vec<Value>> {
        self.load_all(input.as_bytes())
    }

    /// Load all YAML documents from a reader
    pub fn load_all<R: Read>(&self, mut reader: R) -> Result<Vec<Value>> {
        let mut buffer = String::new();
        reader.read_to_string(&mut buffer)?;

        // For now, return a placeholder implementation
        // This will be replaced with the actual parser implementation
        self.parse_yaml_documents(&buffer)
    }

    /// Dump a YAML value to a string
    pub fn dump_str(&self, value: &Value) -> Result<String> {
        let mut buffer = Vec::new();
        self.dump(value, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    /// Dump a YAML value to a writer
    pub fn dump<W: Write>(&self, value: &Value, writer: W) -> Result<()> {
        // For now, return a placeholder implementation
        // This will be replaced with the actual emitter implementation
        self.emit_yaml_value(value, writer)
    }

    /// Dump all YAML documents to a string
    pub fn dump_all_str(&self, values: &[Value]) -> Result<String> {
        let mut buffer = Vec::new();
        self.dump_all(values, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    /// Dump all YAML documents to a writer
    pub fn dump_all<W: Write>(&self, values: &[Value], mut writer: W) -> Result<()> {
        for (i, value) in values.iter().enumerate() {
            if i > 0 {
                writeln!(writer, "---")?;
            }
            self.dump(value, &mut writer)?;
        }
        Ok(())
    }

    /// Load YAML from a string with comment preservation (RoundTrip mode only)
    pub fn load_str_with_comments(&self, input: &str) -> Result<CommentedValue> {
        if !self.config.preserve_comments || self.config.loader_type != LoaderType::RoundTrip {
            // If not in round-trip mode, parse normally and wrap in CommentedValue
            let value = self.load_str(input)?;
            return Ok(CommentedValue::new(value));
        }

        self.parse_yaml_string_with_comments(input)
    }

    /// Dump a CommentedValue to a string, preserving comments
    pub fn dump_str_with_comments(&self, value: &CommentedValue) -> Result<String> {
        let mut buffer = Vec::new();
        self.dump_with_comments(value, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    /// Dump a CommentedValue to a writer, preserving comments
    pub fn dump_with_comments<W: Write>(&self, value: &CommentedValue, writer: W) -> Result<()> {
        self.emit_commented_value(value, writer)
    }

    /// Validate a YAML value against a schema
    pub fn validate_with_schema(&self, value: &Value, schema: &Schema) -> Result<()> {
        let validator = SchemaValidator::new(schema.clone());
        validator.validate_with_report(value)
    }

    /// Load and validate YAML from a string with schema validation
    pub fn load_str_with_schema(&self, input: &str, schema: &Schema) -> Result<Value> {
        let value = self.load_str(input)?;
        self.validate_with_schema(&value, schema)?;
        Ok(value)
    }

    /// Load and validate all YAML documents from a string with schema validation
    pub fn load_all_str_with_schema(&self, input: &str, schema: &Schema) -> Result<Vec<Value>> {
        let values = self.load_all_str(input)?;
        for value in &values {
            self.validate_with_schema(value, schema)?;
        }
        Ok(values)
    }

    // Placeholder implementations - will be replaced with actual parser/emitter

    fn parse_yaml_string(&self, input: &str) -> Result<Value> {
        // Use our complete parsing pipeline: Scanner -> Parser -> Composer -> Constructor
        match self.config.loader_type {
            LoaderType::Safe => {
                let mut constructor =
                    SafeConstructor::with_limits(input.to_string(), self.config.limits.clone());
                (constructor.construct()?).map_or_else(|| Ok(Value::Null), Ok)
            }
            _ => {
                // For now, all loader types use SafeConstructor
                // Future versions will implement different constructors
                let mut constructor =
                    SafeConstructor::with_limits(input.to_string(), self.config.limits.clone());
                (constructor.construct()?).map_or_else(|| Ok(Value::Null), Ok)
            }
        }
    }

    fn parse_yaml_documents(&self, input: &str) -> Result<Vec<Value>> {
        // Use the proper parsing pipeline to handle multi-document streams
        let mut constructor =
            SafeConstructor::with_limits(input.to_string(), self.config.limits.clone());
        let mut documents = Vec::new();

        // Try to construct documents until no more are available
        while constructor.check_data() {
            if let Some(doc) = constructor.construct()? {
                documents.push(doc);
            } else {
                break;
            }
        }

        if documents.is_empty() {
            documents.push(Value::Null);
        }

        Ok(documents)
    }

    fn emit_yaml_value<W: Write>(&self, value: &Value, writer: W) -> Result<()> {
        // Use the proper emitter implementation
        let mut emitter = BasicEmitter::with_indent(self.config.indent.indent);
        emitter.emit(value, writer)?;
        Ok(())
    }

    fn parse_yaml_string_with_comments(&self, input: &str) -> Result<CommentedValue> {
        // Use the round-trip constructor for comment preservation
        let mut constructor =
            RoundTripConstructor::with_limits(input.to_string(), self.config.limits.clone());

        match constructor.construct_commented()? {
            Some(commented_value) => Ok(commented_value),
            None => Ok(CommentedValue::new(Value::Null)),
        }
    }

    fn emit_commented_value<W: Write>(&self, value: &CommentedValue, writer: W) -> Result<()> {
        // Use the proper emitter implementation with comment support
        let mut emitter = BasicEmitter::with_indent(self.config.indent.indent);
        emitter.emit_commented_value_public(value, writer)?;
        Ok(())
    }

    fn emit_yaml_documents<W: Write>(&self, values: &[Value], mut writer: W) -> Result<()> {
        for (i, value) in values.iter().enumerate() {
            if i > 0 {
                writeln!(writer, "---")?;
            }
            self.emit_yaml_value(value, &mut writer)?;
        }
        Ok(())
    }
}

impl Default for Yaml {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_creation() {
        let yaml = Yaml::new();
        assert_eq!(yaml.config().loader_type, LoaderType::Safe);

        let yaml_rt = Yaml::with_loader(LoaderType::RoundTrip);
        assert_eq!(yaml_rt.config().loader_type, LoaderType::RoundTrip);
    }

    #[test]
    fn test_basic_scalar_parsing() {
        let yaml = Yaml::new();

        assert_eq!(yaml.load_str("null").unwrap(), Value::Null);
        assert_eq!(yaml.load_str("true").unwrap(), Value::Bool(true));
        assert_eq!(yaml.load_str("false").unwrap(), Value::Bool(false));
        assert_eq!(yaml.load_str("42").unwrap(), Value::Int(42));
        assert_eq!(yaml.load_str("3.14").unwrap(), Value::Float(3.14));
        assert_eq!(
            yaml.load_str("hello").unwrap(),
            Value::String("hello".to_string())
        );
        assert_eq!(
            yaml.load_str("\"quoted\"").unwrap(),
            Value::String("quoted".to_string())
        );
    }

    #[test]
    fn test_basic_scalar_dumping() {
        let yaml = Yaml::new();

        assert_eq!(yaml.dump_str(&Value::Null).unwrap().trim(), "null");
        assert_eq!(yaml.dump_str(&Value::Bool(true)).unwrap().trim(), "true");
        assert_eq!(yaml.dump_str(&Value::Int(42)).unwrap().trim(), "42");
        assert_eq!(yaml.dump_str(&Value::Float(3.14)).unwrap().trim(), "3.14");
        assert_eq!(
            yaml.dump_str(&Value::String("hello".to_string()))
                .unwrap()
                .trim(),
            "hello"
        );
    }

    #[test]
    fn test_multi_document() {
        let yaml = Yaml::new();
        let input = "doc1\n---\ndoc2\n---\ndoc3";
        let docs = yaml.load_all_str(input).unwrap();

        assert_eq!(docs.len(), 3);
        assert_eq!(docs[0], Value::String("doc1".to_string()));
        assert_eq!(docs[1], Value::String("doc2".to_string()));
        assert_eq!(docs[2], Value::String("doc3".to_string()));
    }

    #[test]
    fn test_config_modification() {
        let mut yaml = Yaml::new();
        yaml.config_mut().loader_type = LoaderType::Full;
        yaml.config_mut().allow_unicode = false;

        assert_eq!(yaml.config().loader_type, LoaderType::Full);
        assert!(!yaml.config().allow_unicode);
    }
}
