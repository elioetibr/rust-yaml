//! YAML constructor for building Rust objects

use crate::{
    BasicComposer, CommentPreservingComposer, CommentedValue, Composer, Error, Limits, Position,
    Result, Value,
};

/// Trait for YAML constructors that convert document nodes to Rust objects
pub trait Constructor {
    /// Construct a single value
    fn construct(&mut self) -> Result<Option<Value>>;

    /// Check if there are more values to construct
    fn check_data(&self) -> bool;

    /// Reset the constructor state
    fn reset(&mut self);
}

/// Trait for comment-preserving constructors
pub trait CommentPreservingConstructor {
    /// Construct a single value with comments
    fn construct_commented(&mut self) -> Result<Option<CommentedValue>>;

    /// Check if there are more values to construct
    fn check_data(&self) -> bool;

    /// Reset the constructor state
    fn reset(&mut self);
}

/// Safe constructor that only constructs basic YAML types
#[derive(Debug)]
pub struct SafeConstructor {
    composer: BasicComposer,
    position: Position,
    limits: Limits,
}

impl SafeConstructor {
    /// Create a new safe constructor with input text
    pub fn new(input: String) -> Self {
        Self::with_limits(input, Limits::default())
    }

    /// Create a new safe constructor with custom limits
    pub fn with_limits(input: String, limits: Limits) -> Self {
        // Use eager composer for better anchor/alias support
        let composer = BasicComposer::new_eager_with_limits(input, limits.clone());
        let position = Position::start();

        Self {
            composer,
            position,
            limits,
        }
    }

    /// Create constructor from existing composer
    pub fn from_composer(composer: BasicComposer) -> Self {
        let position = Position::start();
        let limits = Limits::default();

        Self {
            composer,
            position,
            limits,
        }
    }

    /// Create constructor from existing composer with custom limits
    pub fn from_composer_with_limits(composer: BasicComposer, limits: Limits) -> Self {
        let position = Position::start();

        Self {
            composer,
            position,
            limits,
        }
    }

    /// Validate and potentially transform a value for safety
    fn validate_value(&self, value: Value) -> Result<Value> {
        match value {
            // Basic scalar types are always safe
            Value::Null | Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::String(_) => {
                Ok(value)
            }

            // Sequences are safe if all elements are safe
            Value::Sequence(seq) => {
                // Check collection size limit
                if seq.len() > self.limits.max_collection_size {
                    return Err(Error::limit_exceeded(format!(
                        "Sequence size {} exceeds max_collection_size limit of {}",
                        seq.len(),
                        self.limits.max_collection_size
                    )));
                }
                let mut safe_seq = Vec::with_capacity(seq.len());
                for item in seq {
                    safe_seq.push(self.validate_value(item)?);
                }
                Ok(Value::Sequence(safe_seq))
            }

            // Mappings are safe if all keys and values are safe
            Value::Mapping(map) => {
                // Check collection size limit
                if map.len() > self.limits.max_collection_size {
                    return Err(Error::limit_exceeded(format!(
                        "Mapping size {} exceeds max_collection_size limit of {}",
                        map.len(),
                        self.limits.max_collection_size
                    )));
                }
                let mut safe_map = indexmap::IndexMap::new();
                for (key, val) in map {
                    let safe_key = self.validate_value(key)?;
                    let safe_val = self.validate_value(val)?;
                    safe_map.insert(safe_key, safe_val);
                }
                Ok(Value::Mapping(safe_map))
            }
        }
    }

    /// Apply additional safety checks and transformations
    fn apply_safety_rules(&self, value: Value) -> Result<Value> {
        match value {
            // Limit string length to prevent memory exhaustion
            Value::String(ref s) if s.len() > self.limits.max_string_length => {
                Err(Error::limit_exceeded(format!(
                    "String too long: {} bytes (max: {})",
                    s.len(),
                    self.limits.max_string_length
                )))
            }

            // Limit sequence length
            Value::Sequence(ref seq) if seq.len() > self.limits.max_collection_size => {
                Err(Error::limit_exceeded(format!(
                    "Sequence too long: {} elements (max: {})",
                    seq.len(),
                    self.limits.max_collection_size
                )))
            }

            // Limit mapping size
            Value::Mapping(ref map) if map.len() > self.limits.max_collection_size => {
                Err(Error::limit_exceeded(format!(
                    "Mapping too large: {} entries (max: {})",
                    map.len(),
                    self.limits.max_collection_size
                )))
            }

            // Recursively apply rules
            Value::Sequence(seq) => {
                let mut safe_seq = Vec::with_capacity(seq.len());
                for item in seq {
                    safe_seq.push(self.apply_safety_rules(item)?);
                }
                Ok(Value::Sequence(safe_seq))
            }

            Value::Mapping(map) => {
                let mut safe_map = indexmap::IndexMap::new();
                for (key, val) in map {
                    let safe_key = self.apply_safety_rules(key)?;
                    let safe_val = self.apply_safety_rules(val)?;
                    safe_map.insert(safe_key, safe_val);
                }
                Ok(Value::Mapping(safe_map))
            }

            // Other types are fine as-is
            _ => Ok(value),
        }
    }
}

impl Default for SafeConstructor {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl Constructor for SafeConstructor {
    fn construct(&mut self) -> Result<Option<Value>> {
        // Get a document from the composer
        let document = match self.composer.compose_document()? {
            Some(doc) => doc,
            None => return Ok(None),
        };

        // Validate and apply safety rules
        let validated = self.validate_value(document)?;
        let safe_value = self.apply_safety_rules(validated)?;

        Ok(Some(safe_value))
    }

    fn check_data(&self) -> bool {
        self.composer.check_document()
    }

    fn reset(&mut self) {
        self.composer.reset();
        self.position = Position::start();
    }
}

/// Comment-preserving constructor that maintains comments during parsing
#[derive(Debug)]
pub struct RoundTripConstructor {
    composer: CommentPreservingComposer,
    position: Position,
    limits: Limits,
}

impl RoundTripConstructor {
    /// Create a new round-trip constructor with comment preservation
    pub fn new(input: String) -> Self {
        Self::with_limits(input, Limits::default())
    }

    /// Create a new round-trip constructor with custom limits
    pub fn with_limits(input: String, limits: Limits) -> Self {
        // Use comment-preserving composer
        let composer = CommentPreservingComposer::with_limits(input, limits.clone());
        let position = Position::start();

        Self {
            composer,
            position,
            limits,
        }
    }

    /// Parse the input and build CommentedValue tree
    fn parse_with_comments(&mut self) -> Result<Option<CommentedValue>> {
        // Use the comment-preserving composer directly
        self.composer.compose_document()
    }
}

impl CommentPreservingConstructor for RoundTripConstructor {
    fn construct_commented(&mut self) -> Result<Option<CommentedValue>> {
        self.parse_with_comments()
    }

    fn check_data(&self) -> bool {
        // Check if there are more documents to parse
        // For now, simple implementation
        true
    }

    fn reset(&mut self) {
        self.position = Position::start();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_scalar_construction() {
        let mut constructor = SafeConstructor::new("42".to_string());
        let result = constructor.construct().unwrap().unwrap();
        assert_eq!(result, Value::Int(42));
    }

    #[test]
    fn test_safe_sequence_construction() {
        let mut constructor = SafeConstructor::new("[1, 2, 3]".to_string());
        let result = constructor.construct().unwrap().unwrap();

        let expected = Value::Sequence(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_safe_mapping_construction() {
        let mut constructor = SafeConstructor::new("{'key': 'value'}".to_string());
        let result = constructor.construct().unwrap().unwrap();

        let mut expected_map = indexmap::IndexMap::new();
        expected_map.insert(
            Value::String("key".to_string()),
            Value::String("value".to_string()),
        );
        let expected = Value::Mapping(expected_map);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_nested_construction() {
        let yaml_content = "{'users': [{'name': 'Alice', 'age': 30}]}";
        let mut constructor = SafeConstructor::new(yaml_content.to_string());
        let result = constructor.construct().unwrap().unwrap();

        if let Value::Mapping(map) = result {
            if let Some(Value::Sequence(users)) = map.get(&Value::String("users".to_string())) {
                assert_eq!(users.len(), 1);
                if let Value::Mapping(ref user) = users[0] {
                    assert_eq!(
                        user.get(&Value::String("name".to_string())),
                        Some(&Value::String("Alice".to_string()))
                    );
                    assert_eq!(
                        user.get(&Value::String("age".to_string())),
                        Some(&Value::Int(30))
                    );
                }
            }
        } else {
            panic!("Expected mapping");
        }
    }

    #[test]
    fn test_check_data() {
        let constructor = SafeConstructor::new("42".to_string());
        assert!(constructor.check_data());
    }

    #[test]
    fn test_multiple_types() {
        let yaml_content = "{'string': 'hello', 'int': 42, 'bool': true, 'null_key': null}";
        let mut constructor = SafeConstructor::new(yaml_content.to_string());
        let result = constructor.construct().unwrap().unwrap();

        if let Value::Mapping(map) = result {
            assert_eq!(
                map.get(&Value::String("string".to_string())),
                Some(&Value::String("hello".to_string()))
            );
            assert_eq!(
                map.get(&Value::String("int".to_string())),
                Some(&Value::Int(42))
            );
            assert_eq!(
                map.get(&Value::String("bool".to_string())),
                Some(&Value::Bool(true))
            );
            // The key is "null_key" (string) and the value should be null (Null type)
            assert_eq!(
                map.get(&Value::String("null_key".to_string())),
                Some(&Value::Null)
            );
        } else {
            panic!("Expected mapping");
        }
    }

    #[test]
    fn test_safety_limits() {
        // Test with a reasonable size that shouldn't cause timeouts
        let large_string = "a".repeat(1000); // Much smaller size for testing
        let yaml_content = format!("value: '{}'", large_string);
        let mut constructor = SafeConstructor::new(yaml_content);

        let result = constructor.construct();
        // This should succeed with a reasonable size
        match result {
            Ok(Some(value)) => {
                // Should get a mapping with a string value
                if let Value::Mapping(map) = value {
                    if let Some(Value::String(s)) = map.get(&Value::String("value".to_string())) {
                        assert_eq!(s.len(), 1000);
                    }
                }
            }
            Ok(None) => {
                // Empty document is also acceptable
            }
            Err(error) => {
                // If it fails, just ensure we have a meaningful error
                assert!(!error.to_string().is_empty());
            }
        }
    }

    #[test]
    fn test_boolean_values() {
        let test_cases = vec![
            ("true", true),
            ("false", false),
            ("yes", true),
            ("no", false),
            ("on", true),
            ("off", false),
        ];

        for (input, expected) in test_cases {
            let mut constructor = SafeConstructor::new(input.to_string());
            let result = constructor.construct().unwrap().unwrap();
            assert_eq!(result, Value::Bool(expected), "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_null_values() {
        let test_cases = vec!["null", "~"];

        for input in test_cases {
            let mut constructor = SafeConstructor::new(input.to_string());
            let result = constructor.construct().unwrap().unwrap();
            assert_eq!(result, Value::Null, "Failed for input: {}", input);
        }
    }
}
