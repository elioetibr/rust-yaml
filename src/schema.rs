//! YAML schema validation system

use crate::{Error, Position, Result, Value};
use regex::Regex;
use std::collections::HashMap;
use std::fmt;

/// Schema validation error with detailed context
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Path to the invalid value (e.g., "config.database.port")
    pub path: String,
    /// The validation rule that failed
    pub rule: String,
    /// Human-readable error message
    pub message: String,
    /// The invalid value
    pub value: Value,
    /// Position in the YAML document (if available)
    pub position: Option<Position>,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Validation error at '{}': {}", self.path, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Result type for schema validation
pub type ValidationResult<T> = std::result::Result<T, Vec<ValidationError>>;

/// Custom validator function type
pub type ValidatorFn = Box<dyn Fn(&Value, &str) -> Result<()> + Send + Sync>;

/// Schema validation rules
#[derive(Debug, Clone)]
pub enum SchemaRule {
    /// Validate type (string, number, boolean, array, object, null)
    Type(ValueType),
    /// String must match regex pattern
    Pattern(Regex),
    /// String/Array length constraints
    Length {
        /// Minimum length (inclusive)
        min: Option<usize>,
        /// Maximum length (inclusive)
        max: Option<usize>,
    },
    /// Number range constraints
    Range {
        /// Minimum value (inclusive)
        min: Option<f64>,
        /// Maximum value (inclusive)
        max: Option<f64>,
    },
    /// Value must be one of the specified values
    Enum(Vec<Value>),
    /// Object property validation
    Properties(HashMap<String, Schema>),
    /// Array item validation
    Items(Box<Schema>),
    /// Required properties for objects
    Required(Vec<String>),
    /// Additional properties allowed for objects
    AdditionalProperties(bool),
    /// Custom validation function  
    Custom(String),
    /// Conditional validation (if-then-else)
    Conditional {
        /// Condition to check
        if_schema: Box<Schema>,
        /// Schema to apply if condition matches
        then_schema: Option<Box<Schema>>,
        /// Schema to apply if condition doesn't match
        else_schema: Option<Box<Schema>>,
    },
    /// Value must not match this schema (negation)
    Not(Box<Schema>),
    /// Value must match any of these schemas (OR)
    AnyOf(Vec<Schema>),
    /// Value must match all of these schemas (AND)
    AllOf(Vec<Schema>),
    /// Value must match exactly one of these schemas (XOR)
    OneOf(Vec<Schema>),
}

/// Supported value types for validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueType {
    /// String type
    String,
    /// Number type (float or integer)
    Number,
    /// Integer type
    Integer,
    /// Boolean type
    Boolean,
    /// Array/Sequence type
    Array,
    /// Object/Mapping type
    Object,
    /// Null type
    Null,
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueType::String => write!(f, "string"),
            ValueType::Number => write!(f, "number"),
            ValueType::Integer => write!(f, "integer"),
            ValueType::Boolean => write!(f, "boolean"),
            ValueType::Array => write!(f, "array"),
            ValueType::Object => write!(f, "object"),
            ValueType::Null => write!(f, "null"),
        }
    }
}

/// A complete schema definition
#[derive(Debug, Clone)]
pub struct Schema {
    /// Schema title/description
    pub title: Option<String>,
    /// Schema description
    pub description: Option<String>,
    /// Validation rules for this schema
    pub rules: Vec<SchemaRule>,
    /// Whether this schema is optional
    pub optional: bool,
    /// Default value if not provided
    pub default: Option<Value>,
}

impl Schema {
    /// Create a new empty schema
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            rules: Vec::new(),
            optional: false,
            default: None,
        }
    }

    /// Create a schema with a specific type
    pub fn with_type(value_type: ValueType) -> Self {
        Self {
            title: None,
            description: None,
            rules: vec![SchemaRule::Type(value_type)],
            optional: false,
            default: None,
        }
    }

    /// Add a validation rule
    pub fn rule(mut self, rule: SchemaRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Add multiple validation rules
    pub fn rules(mut self, rules: Vec<SchemaRule>) -> Self {
        self.rules.extend(rules);
        self
    }

    /// Set title and description
    pub fn info(mut self, title: &str, description: &str) -> Self {
        self.title = Some(title.to_string());
        self.description = Some(description.to_string());
        self
    }

    /// Mark schema as optional
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Set default value
    pub fn default_value(mut self, value: Value) -> Self {
        self.default = Some(value);
        self
    }

    /// Validate a value against this schema
    pub fn validate(&self, value: &Value, path: &str) -> ValidationResult<()> {
        let mut errors = Vec::new();

        // Apply each validation rule
        for rule in &self.rules {
            if let Err(rule_errors) = self.apply_rule(rule, value, path) {
                errors.extend(rule_errors);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Apply a single validation rule
    fn apply_rule(&self, rule: &SchemaRule, value: &Value, path: &str) -> ValidationResult<()> {
        match rule {
            SchemaRule::Type(expected_type) => self.validate_type(expected_type, value, path),
            SchemaRule::Pattern(regex) => self.validate_pattern(regex, value, path),
            SchemaRule::Length { min, max } => self.validate_length(*min, *max, value, path),
            SchemaRule::Range { min, max } => self.validate_range(*min, *max, value, path),
            SchemaRule::Enum(allowed_values) => self.validate_enum(allowed_values, value, path),
            SchemaRule::Properties(properties) => self.validate_properties(properties, value, path),
            SchemaRule::Items(item_schema) => self.validate_items(item_schema, value, path),
            SchemaRule::Required(required_props) => {
                self.validate_required(required_props, value, path)
            }
            SchemaRule::AdditionalProperties(allowed) => {
                self.validate_additional_properties(*allowed, value, path)
            }
            SchemaRule::Custom(name) => self.validate_custom(name, value, path),
            SchemaRule::Conditional {
                if_schema,
                then_schema,
                else_schema,
            } => self.validate_conditional(
                if_schema,
                then_schema.as_ref().map(|v| &**v),
                else_schema.as_ref().map(|v| &**v),
                value,
                path,
            ),
            SchemaRule::Not(schema) => self.validate_not(schema, value, path),
            SchemaRule::AnyOf(schemas) => self.validate_any_of(schemas, value, path),
            SchemaRule::AllOf(schemas) => self.validate_all_of(schemas, value, path),
            SchemaRule::OneOf(schemas) => self.validate_one_of(schemas, value, path),
        }
    }

    /// Validate value type
    fn validate_type(
        &self,
        expected_type: &ValueType,
        value: &Value,
        path: &str,
    ) -> ValidationResult<()> {
        let actual_type = match value {
            Value::String(_) => ValueType::String,
            Value::Int(_) => ValueType::Integer,
            Value::Float(_) => ValueType::Number,
            Value::Bool(_) => ValueType::Boolean,
            Value::Sequence(_) => ValueType::Array,
            Value::Mapping(_) => ValueType::Object,
            Value::Null => ValueType::Null,
        };

        // Allow integer to be considered as number
        let type_matches = match (expected_type, &actual_type) {
            (ValueType::Number, ValueType::Integer) => true,
            _ => expected_type == &actual_type,
        };

        if type_matches {
            Ok(())
        } else {
            Err(vec![ValidationError {
                path: path.to_string(),
                rule: "type".to_string(),
                message: format!("Expected {}, got {}", expected_type, actual_type),
                value: value.clone(),
                position: None,
            }])
        }
    }

    /// Validate regex pattern
    fn validate_pattern(&self, regex: &Regex, value: &Value, path: &str) -> ValidationResult<()> {
        if let Value::String(s) = value {
            if regex.is_match(s) {
                Ok(())
            } else {
                Err(vec![ValidationError {
                    path: path.to_string(),
                    rule: "pattern".to_string(),
                    message: format!("String '{}' does not match required pattern", s),
                    value: value.clone(),
                    position: None,
                }])
            }
        } else {
            Err(vec![ValidationError {
                path: path.to_string(),
                rule: "pattern".to_string(),
                message: "Pattern validation can only be applied to strings".to_string(),
                value: value.clone(),
                position: None,
            }])
        }
    }

    /// Validate length constraints
    fn validate_length(
        &self,
        min: Option<usize>,
        max: Option<usize>,
        value: &Value,
        path: &str,
    ) -> ValidationResult<()> {
        let length = match value {
            Value::String(s) => s.len(),
            Value::Sequence(seq) => seq.len(),
            _ => {
                return Err(vec![ValidationError {
                    path: path.to_string(),
                    rule: "length".to_string(),
                    message: "Length validation can only be applied to strings or arrays"
                        .to_string(),
                    value: value.clone(),
                    position: None,
                }]);
            }
        };

        let mut errors = Vec::new();

        if let Some(min_len) = min {
            if length < min_len {
                errors.push(ValidationError {
                    path: path.to_string(),
                    rule: "minLength".to_string(),
                    message: format!("Length {} is less than minimum {}", length, min_len),
                    value: value.clone(),
                    position: None,
                });
            }
        }

        if let Some(max_len) = max {
            if length > max_len {
                errors.push(ValidationError {
                    path: path.to_string(),
                    rule: "maxLength".to_string(),
                    message: format!("Length {} is greater than maximum {}", length, max_len),
                    value: value.clone(),
                    position: None,
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate numeric range
    fn validate_range(
        &self,
        min: Option<f64>,
        max: Option<f64>,
        value: &Value,
        path: &str,
    ) -> ValidationResult<()> {
        let number = match value {
            Value::Int(i) => *i as f64,
            Value::Float(f) => *f,
            _ => {
                return Err(vec![ValidationError {
                    path: path.to_string(),
                    rule: "range".to_string(),
                    message: "Range validation can only be applied to numbers".to_string(),
                    value: value.clone(),
                    position: None,
                }]);
            }
        };

        let mut errors = Vec::new();

        if let Some(min_val) = min {
            if number < min_val {
                errors.push(ValidationError {
                    path: path.to_string(),
                    rule: "minimum".to_string(),
                    message: format!("Value {} is less than minimum {}", number, min_val),
                    value: value.clone(),
                    position: None,
                });
            }
        }

        if let Some(max_val) = max {
            if number > max_val {
                errors.push(ValidationError {
                    path: path.to_string(),
                    rule: "maximum".to_string(),
                    message: format!("Value {} is greater than maximum {}", number, max_val),
                    value: value.clone(),
                    position: None,
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate enum values
    fn validate_enum(
        &self,
        allowed_values: &[Value],
        value: &Value,
        path: &str,
    ) -> ValidationResult<()> {
        if allowed_values.contains(value) {
            Ok(())
        } else {
            Err(vec![ValidationError {
                path: path.to_string(),
                rule: "enum".to_string(),
                message: format!(
                    "Value is not one of the allowed values: {:?}",
                    allowed_values
                ),
                value: value.clone(),
                position: None,
            }])
        }
    }

    /// Validate object properties
    fn validate_properties(
        &self,
        properties: &HashMap<String, Schema>,
        value: &Value,
        path: &str,
    ) -> ValidationResult<()> {
        if let Value::Mapping(map) = value {
            let mut errors = Vec::new();

            for (prop_name, prop_schema) in properties {
                let prop_path = if path.is_empty() {
                    prop_name.clone()
                } else {
                    format!("{}.{}", path, prop_name)
                };

                // Find the property in the mapping
                let prop_value = map
                    .iter()
                    .find(|(k, _)| {
                        if let Value::String(key_str) = k {
                            key_str == prop_name
                        } else {
                            false
                        }
                    })
                    .map(|(_, v)| v);

                match prop_value {
                    Some(value) => {
                        // Validate the property
                        if let Err(prop_errors) = prop_schema.validate(value, &prop_path) {
                            errors.extend(prop_errors);
                        }
                    }
                    None => {
                        // Property is missing
                        if !prop_schema.optional {
                            errors.push(ValidationError {
                                path: prop_path,
                                rule: "required".to_string(),
                                message: format!("Required property '{}' is missing", prop_name),
                                value: Value::Null,
                                position: None,
                            });
                        }
                    }
                }
            }

            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        } else {
            Err(vec![ValidationError {
                path: path.to_string(),
                rule: "properties".to_string(),
                message: "Properties validation can only be applied to objects".to_string(),
                value: value.clone(),
                position: None,
            }])
        }
    }

    /// Validate array items
    fn validate_items(
        &self,
        item_schema: &Schema,
        value: &Value,
        path: &str,
    ) -> ValidationResult<()> {
        if let Value::Sequence(seq) = value {
            let mut errors = Vec::new();

            for (index, item) in seq.iter().enumerate() {
                let item_path = format!("{}[{}]", path, index);
                if let Err(item_errors) = item_schema.validate(item, &item_path) {
                    errors.extend(item_errors);
                }
            }

            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        } else {
            Err(vec![ValidationError {
                path: path.to_string(),
                rule: "items".to_string(),
                message: "Items validation can only be applied to arrays".to_string(),
                value: value.clone(),
                position: None,
            }])
        }
    }

    /// Validate required properties
    fn validate_required(
        &self,
        required_props: &[String],
        value: &Value,
        path: &str,
    ) -> ValidationResult<()> {
        if let Value::Mapping(map) = value {
            let mut errors = Vec::new();

            for required_prop in required_props {
                let has_property = map.keys().any(|k| {
                    if let Value::String(key_str) = k {
                        key_str == required_prop
                    } else {
                        false
                    }
                });

                if !has_property {
                    let prop_path = if path.is_empty() {
                        required_prop.clone()
                    } else {
                        format!("{}.{}", path, required_prop)
                    };

                    errors.push(ValidationError {
                        path: prop_path,
                        rule: "required".to_string(),
                        message: format!("Required property '{}' is missing", required_prop),
                        value: Value::Null,
                        position: None,
                    });
                }
            }

            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        } else {
            Err(vec![ValidationError {
                path: path.to_string(),
                rule: "required".to_string(),
                message: "Required validation can only be applied to objects".to_string(),
                value: value.clone(),
                position: None,
            }])
        }
    }

    /// Validate additional properties
    fn validate_additional_properties(
        &self,
        allowed: bool,
        value: &Value,
        _path: &str,
    ) -> ValidationResult<()> {
        // This would be implemented in conjunction with Properties validation
        // For now, always allow additional properties
        if !allowed {
            // TODO: Check for additional properties not defined in schema
        }
        Ok(())
    }

    /// Validate using custom function
    fn validate_custom(&self, name: &str, value: &Value, path: &str) -> ValidationResult<()> {
        // For now, custom validation always passes
        // In a full implementation, this would call user-provided functions
        Ok(())
    }

    /// Validate conditional logic
    fn validate_conditional(
        &self,
        if_schema: &Schema,
        then_schema: Option<&Schema>,
        else_schema: Option<&Schema>,
        value: &Value,
        path: &str,
    ) -> ValidationResult<()> {
        // Check if the "if" condition matches
        let if_matches = if_schema.validate(value, path).is_ok();

        if if_matches {
            if let Some(then_schema) = then_schema {
                then_schema.validate(value, path)
            } else {
                Ok(())
            }
        } else if let Some(else_schema) = else_schema {
            else_schema.validate(value, path)
        } else {
            Ok(())
        }
    }

    /// Validate negation (NOT)
    fn validate_not(&self, schema: &Schema, value: &Value, path: &str) -> ValidationResult<()> {
        if schema.validate(value, path).is_ok() {
            Err(vec![ValidationError {
                path: path.to_string(),
                rule: "not".to_string(),
                message: "Value must not match the specified schema".to_string(),
                value: value.clone(),
                position: None,
            }])
        } else {
            Ok(())
        }
    }

    /// Validate any of (OR)
    fn validate_any_of(
        &self,
        schemas: &[Schema],
        value: &Value,
        path: &str,
    ) -> ValidationResult<()> {
        for schema in schemas {
            if schema.validate(value, path).is_ok() {
                return Ok(());
            }
        }

        Err(vec![ValidationError {
            path: path.to_string(),
            rule: "anyOf".to_string(),
            message: "Value must match at least one of the specified schemas".to_string(),
            value: value.clone(),
            position: None,
        }])
    }

    /// Validate all of (AND)
    fn validate_all_of(
        &self,
        schemas: &[Schema],
        value: &Value,
        path: &str,
    ) -> ValidationResult<()> {
        let mut all_errors = Vec::new();

        for schema in schemas {
            if let Err(errors) = schema.validate(value, path) {
                all_errors.extend(errors);
            }
        }

        if all_errors.is_empty() {
            Ok(())
        } else {
            Err(all_errors)
        }
    }

    /// Validate one of (XOR)
    fn validate_one_of(
        &self,
        schemas: &[Schema],
        value: &Value,
        path: &str,
    ) -> ValidationResult<()> {
        let mut valid_count = 0;

        for schema in schemas {
            if schema.validate(value, path).is_ok() {
                valid_count += 1;
            }
        }

        if valid_count == 1 {
            Ok(())
        } else if valid_count == 0 {
            Err(vec![ValidationError {
                path: path.to_string(),
                rule: "oneOf".to_string(),
                message: "Value must match exactly one of the specified schemas (matched none)"
                    .to_string(),
                value: value.clone(),
                position: None,
            }])
        } else {
            Err(vec![ValidationError {
                path: path.to_string(),
                rule: "oneOf".to_string(),
                message: format!(
                    "Value must match exactly one of the specified schemas (matched {})",
                    valid_count
                ),
                value: value.clone(),
                position: None,
            }])
        }
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

/// Schema validator for YAML documents
#[derive(Debug)]
pub struct SchemaValidator {
    /// Root schema for validation
    pub schema: Schema,
    /// Whether to collect all errors or stop at first error
    pub collect_all_errors: bool,
}

impl SchemaValidator {
    /// Create a new schema validator
    pub fn new(schema: Schema) -> Self {
        Self {
            schema,
            collect_all_errors: true,
        }
    }

    /// Create a validator that stops at first error
    pub fn fail_fast(schema: Schema) -> Self {
        Self {
            schema,
            collect_all_errors: false,
        }
    }

    /// Validate a YAML value against the schema
    pub fn validate(&self, value: &Value) -> ValidationResult<()> {
        self.schema.validate(value, "")
    }

    /// Validate and return a formatted error report
    pub fn validate_with_report(&self, value: &Value) -> Result<()> {
        match self.validate(value) {
            Ok(()) => Ok(()),
            Err(errors) => {
                let error_messages: Vec<String> =
                    errors.iter().map(|e| format!("  - {}", e)).collect();

                let message = format!(
                    "Schema validation failed with {} error(s):\n{}",
                    errors.len(),
                    error_messages.join("\n")
                );

                Err(Error::parse(Position::start(), message))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    #[test]
    fn test_type_validation() {
        let schema = Schema::with_type(ValueType::String);

        // Valid case
        assert!(schema
            .validate(&Value::String("hello".to_string()), "test")
            .is_ok());

        // Invalid case
        assert!(schema.validate(&Value::Int(42), "test").is_err());
    }

    #[test]
    fn test_range_validation() {
        let schema = Schema::new()
            .rule(SchemaRule::Type(ValueType::Number))
            .rule(SchemaRule::Range {
                min: Some(0.0),
                max: Some(100.0),
            });

        // Valid cases
        assert!(schema.validate(&Value::Int(50), "test").is_ok());
        assert!(schema.validate(&Value::Float(75.5), "test").is_ok());

        // Invalid cases
        assert!(schema.validate(&Value::Int(-1), "test").is_err());
        assert!(schema.validate(&Value::Int(101), "test").is_err());
    }

    #[test]
    fn test_object_validation() {
        let mut properties = HashMap::new();
        properties.insert("name".to_string(), Schema::with_type(ValueType::String));
        properties.insert(
            "age".to_string(),
            Schema::with_type(ValueType::Integer).rule(SchemaRule::Range {
                min: Some(0.0),
                max: Some(150.0),
            }),
        );

        let schema = Schema::with_type(ValueType::Object)
            .rule(SchemaRule::Properties(properties))
            .rule(SchemaRule::Required(vec!["name".to_string()]));

        // Valid case
        let mut map = IndexMap::new();
        map.insert(
            Value::String("name".to_string()),
            Value::String("Alice".to_string()),
        );
        map.insert(Value::String("age".to_string()), Value::Int(30));

        assert!(schema.validate(&Value::Mapping(map), "test").is_ok());

        // Invalid case - missing required property
        let mut invalid_map = IndexMap::new();
        invalid_map.insert(Value::String("age".to_string()), Value::Int(30));

        assert!(schema
            .validate(&Value::Mapping(invalid_map), "test")
            .is_err());
    }
}
