#![allow(clippy::uninlined_format_args)]
#![allow(clippy::clone_on_copy)]
#![allow(unused_mut)]
#![allow(clippy::approx_constant)]

use indexmap::IndexMap;
use rust_yaml::{Error, Position, Value};

mod test_value {
    use super::*;

    #[test]
    fn test_value_creation() {
        assert_eq!(Value::Null, Value::Null);
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_eq!(Value::Bool(false), Value::Bool(false));
        assert_eq!(Value::Int(42), Value::Int(42));
        assert_eq!(Value::Float(3.14), Value::Float(3.14));
        assert_eq!(
            Value::String("hello".to_string()),
            Value::String("hello".to_string())
        );
    }

    #[test]
    fn test_value_from_conversions() {
        assert_eq!(Value::from(true), Value::Bool(true));
        assert_eq!(Value::from(42i32), Value::Int(42));
        assert_eq!(Value::from(42i64), Value::Int(42));
        assert_eq!(Value::from(3.14f64), Value::Float(3.14));
        assert_eq!(Value::from("hello"), Value::String("hello".to_string()));
        assert_eq!(
            Value::from("hello".to_string()),
            Value::String("hello".to_string())
        );
    }

    #[test]
    fn test_value_sequence() {
        let seq = vec![Value::Int(1), Value::Int(2), Value::Int(3)];
        let value = Value::Sequence(seq.clone());

        assert!(value.is_sequence());
        assert_eq!(value.as_sequence(), Some(&seq));
        assert_eq!(value.len(), Some(3));
    }

    #[test]
    fn test_value_mapping() {
        let mut map = IndexMap::new();
        map.insert(
            Value::String("key1".to_string()),
            Value::String("value1".to_string()),
        );
        map.insert(Value::String("key2".to_string()), Value::Int(42));

        let value = Value::Mapping(map.clone());

        assert!(value.is_mapping());
        assert_eq!(value.as_mapping(), Some(&map));
        assert_eq!(value.len(), Some(2));
    }

    #[test]
    fn test_value_type_checking() {
        assert!(Value::Null.is_null());
        assert!(Value::Bool(true).is_bool());
        assert!(Value::Int(42).is_int());
        assert!(Value::Float(3.14).is_float());
        assert!(Value::String("hello".to_string()).is_string());

        // Test number detection
        assert!(Value::Int(42).is_number());
        assert!(Value::Float(3.14).is_number());
        assert!(!Value::String("42".to_string()).is_number());
    }

    #[test]
    fn test_value_access() {
        let mut map = IndexMap::new();
        map.insert(
            Value::String("name".to_string()),
            Value::String("Alice".to_string()),
        );
        map.insert(Value::String("age".to_string()), Value::Int(30));

        let value = Value::Mapping(map);

        // Test key access
        assert_eq!(
            value.get_str("name"),
            Some(&Value::String("Alice".to_string()))
        );
        assert_eq!(value.get_str("age"), Some(&Value::Int(30)));
        assert_eq!(value.get_str("unknown"), None);

        // Test index access for sequences
        let seq_value = Value::Sequence(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert_eq!(seq_value.get_index(0), Some(&Value::Int(1)));
        assert_eq!(seq_value.get_index(1), Some(&Value::Int(2)));
        assert_eq!(seq_value.get_index(5), None);
    }

    #[test]
    fn test_value_equality() {
        // Test basic equality
        assert_eq!(Value::Int(42), Value::Int(42));
        assert_ne!(Value::Int(42), Value::Int(43));

        // Test sequence equality
        let seq1 = Value::Sequence(vec![Value::Int(1), Value::Int(2)]);
        let seq2 = Value::Sequence(vec![Value::Int(1), Value::Int(2)]);
        let seq3 = Value::Sequence(vec![Value::Int(1), Value::Int(3)]);

        assert_eq!(seq1, seq2);
        assert_ne!(seq1, seq3);

        // Test mapping equality (order preserved)
        let mut map1 = IndexMap::new();
        map1.insert(Value::String("a".to_string()), Value::Int(1));
        map1.insert(Value::String("b".to_string()), Value::Int(2));

        let mut map2 = IndexMap::new();
        map2.insert(Value::String("a".to_string()), Value::Int(1));
        map2.insert(Value::String("b".to_string()), Value::Int(2));

        assert_eq!(Value::Mapping(map1), Value::Mapping(map2));
    }

    #[test]
    fn test_value_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(Value::Int(42));
        set.insert(Value::String("hello".to_string()));
        set.insert(Value::Bool(true));

        // Should not insert duplicates
        set.insert(Value::Int(42));
        assert_eq!(set.len(), 3);

        // NaN handling
        set.insert(Value::Float(f64::NAN));
        set.insert(Value::Float(f64::NAN));
        assert_eq!(set.len(), 4); // NaN should hash consistently
    }

    #[test]
    fn test_empty_containers() {
        let empty_seq = Value::Sequence(vec![]);
        assert_eq!(empty_seq.len(), Some(0));
        assert!(empty_seq.is_empty());

        let empty_map = Value::Mapping(IndexMap::new());
        assert_eq!(empty_map.len(), Some(0));
        assert!(empty_map.is_empty());

        let string_val = Value::String(String::new());
        assert!(string_val.is_empty());
    }
}

mod test_position {
    use super::*;

    #[test]
    fn test_position_creation() {
        let pos = Position::new();
        assert_eq!(pos.line, 1);
        assert_eq!(pos.column, 1);
        assert_eq!(pos.index, 0);
    }

    #[test]
    fn test_position_advance() {
        let mut pos = Position::new();

        // Advance by regular character
        pos = pos.advance('a');
        assert_eq!(pos.line, 1);
        assert_eq!(pos.column, 2);
        assert_eq!(pos.index, 1);

        // Advance by newline
        pos = pos.advance('\n');
        assert_eq!(pos.line, 2);
        assert_eq!(pos.column, 1);
        assert_eq!(pos.index, 2);
    }

    #[test]
    fn test_position_advance_str() {
        let mut pos = Position::new();
        pos = pos.advance_str("hello\nworld");

        assert_eq!(pos.line, 2);
        assert_eq!(pos.column, 6); // "world" has 5 chars + 1 for column start
        assert_eq!(pos.index, 11);
    }

    #[test]
    fn test_position_clone() {
        let pos1 = Position::new();
        let pos2 = pos1.clone();
        let pos2 = pos2.advance('a');

        // Original should be unchanged
        assert_eq!(pos1.line, 1);
        assert_eq!(pos1.column, 1);

        // Clone should be modified
        assert_eq!(pos2.line, 1);
        assert_eq!(pos2.column, 2);
    }

    #[test]
    fn test_position_display() {
        let pos = Position {
            line: 5,
            column: 10,
            index: 50,
        };
        let display = format!("{}", pos);
        assert!(display.contains('5'));
        assert!(display.contains("10"));
    }
}

mod test_error {
    use super::*;

    #[test]
    fn test_error_creation() {
        let pos = Position::new();

        let parse_error = Error::parse(pos.clone(), "invalid syntax");
        assert!(matches!(parse_error, Error::Parse { .. }));

        let scan_error = Error::scan(pos.clone(), "unexpected character");
        assert!(matches!(scan_error, Error::Scan { .. }));

        let construction_error = Error::construction(pos.clone(), "type mismatch");
        assert!(matches!(construction_error, Error::Construction { .. }));
    }

    #[test]
    fn test_error_display() {
        let pos = Position {
            line: 3,
            column: 5,
            index: 20,
        };
        let error = Error::parse(pos, "missing closing bracket");

        let error_msg = error.to_string();
        assert!(error_msg.contains("line 3"));
        assert!(error_msg.contains("column 5"));
        assert!(error_msg.contains("missing closing bracket"));
    }

    #[test]
    fn test_error_position() {
        let pos = Position {
            line: 10,
            column: 20,
            index: 100,
        };
        let error = Error::parse(pos.clone(), "test error");

        if let Error::Parse { position, .. } = error {
            assert_eq!(position.line, 10);
            assert_eq!(position.column, 20);
            assert_eq!(position.index, 100);
        } else {
            panic!("Expected Parse error");
        }
    }

    #[test]
    fn test_error_eq() {
        let pos1 = Position {
            line: 1,
            column: 1,
            index: 0,
        };
        let pos2 = Position {
            line: 1,
            column: 1,
            index: 0,
        };

        let error1 = Error::parse(pos1, "test message");
        let error2 = Error::parse(pos2, "test message");

        assert_eq!(error1, error2);
    }

    #[test]
    fn test_all_error_types() {
        let pos = Position::new();

        let errors = vec![
            Error::parse(pos.clone(), "parse error"),
            Error::scan(pos.clone(), "scan error"),
            Error::construction(pos.clone(), "construction error"),
            Error::emission("emission error"),
            Error::type_error(pos.clone(), "expected_type", "found_type"),
            Error::value_error(pos.clone(), "value error"),
            Error::config_error("config error"),
        ];

        // All should be different types
        for (i, error1) in errors.iter().enumerate() {
            for (j, error2) in errors.iter().enumerate() {
                if i != j {
                    assert_ne!(
                        std::mem::discriminant(error1),
                        std::mem::discriminant(error2)
                    );
                }
            }
        }
    }
}
