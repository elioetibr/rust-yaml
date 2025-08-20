//! Performance benchmarks for schema validation

use criterion::{criterion_group, criterion_main, Criterion};
use indexmap::IndexMap;
use regex::Regex;
use rust_yaml::{Schema, SchemaRule, SchemaValidator, Value, ValueType, Yaml};
use std::collections::HashMap;

fn bench_basic_type_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("basic_type_validation");

    // Create different type validators
    let string_schema = Schema::with_type(ValueType::String);
    let string_validator = SchemaValidator::new(string_schema);

    let int_schema = Schema::with_type(ValueType::Integer);
    let int_validator = SchemaValidator::new(int_schema);

    let bool_schema = Schema::with_type(ValueType::Boolean);
    let bool_validator = SchemaValidator::new(bool_schema);

    // Test values
    let string_value = Value::String("hello world".to_string());
    let int_value = Value::Int(42);
    let bool_value = Value::Bool(true);

    group.bench_function("string_type_validation", |b| {
        b.iter(|| {
            string_validator
                .validate(std::hint::black_box(&string_value))
                .unwrap()
        });
    });

    group.bench_function("integer_type_validation", |b| {
        b.iter(|| {
            int_validator
                .validate(std::hint::black_box(&int_value))
                .unwrap()
        });
    });

    group.bench_function("boolean_type_validation", |b| {
        b.iter(|| {
            bool_validator
                .validate(std::hint::black_box(&bool_value))
                .unwrap()
        });
    });

    // Test validation failures
    group.bench_function("type_validation_failure", |b| {
        b.iter(|| {
            let _ = string_validator.validate(std::hint::black_box(&int_value));
        });
    });

    group.finish();
}

fn bench_constraint_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("constraint_validation");

    // Range validation
    let age_schema = Schema::with_type(ValueType::Integer).rule(SchemaRule::Range {
        min: Some(0.0),
        max: Some(150.0),
    });
    let age_validator = SchemaValidator::new(age_schema);

    // Length validation
    let name_schema = Schema::with_type(ValueType::String).rule(SchemaRule::Length {
        min: Some(2),
        max: Some(50),
    });
    let name_validator = SchemaValidator::new(name_schema);

    // Pattern validation
    let email_pattern = Regex::new(r"^[^@]+@[^@]+\.[^@]+$").unwrap();
    let email_schema =
        Schema::with_type(ValueType::String).rule(SchemaRule::Pattern(email_pattern));
    let email_validator = SchemaValidator::new(email_schema);

    // Test values
    let valid_age = Value::Int(30);
    let valid_name = Value::String("Alice Johnson".to_string());
    let valid_email = Value::String("alice@example.com".to_string());

    group.bench_function("range_validation", |b| {
        b.iter(|| {
            age_validator
                .validate(std::hint::black_box(&valid_age))
                .unwrap()
        });
    });

    group.bench_function("length_validation", |b| {
        b.iter(|| {
            name_validator
                .validate(std::hint::black_box(&valid_name))
                .unwrap()
        });
    });

    group.bench_function("pattern_validation", |b| {
        b.iter(|| {
            email_validator
                .validate(std::hint::black_box(&valid_email))
                .unwrap()
        });
    });

    group.finish();
}

fn bench_object_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("object_validation");

    // Create user schema
    let mut user_properties = HashMap::new();
    user_properties.insert(
        "name".to_string(),
        Schema::with_type(ValueType::String).rule(SchemaRule::Length {
            min: Some(2),
            max: Some(50),
        }),
    );
    user_properties.insert(
        "email".to_string(),
        Schema::with_type(ValueType::String).rule(SchemaRule::Pattern(
            Regex::new(r"^[^@]+@[^@]+\.[^@]+$").unwrap(),
        )),
    );
    user_properties.insert(
        "age".to_string(),
        Schema::with_type(ValueType::Integer).rule(SchemaRule::Range {
            min: Some(18.0),
            max: Some(100.0),
        }),
    );

    let user_schema = Schema::with_type(ValueType::Object)
        .rule(SchemaRule::Properties(user_properties))
        .rule(SchemaRule::Required(vec![
            "name".to_string(),
            "email".to_string(),
        ]));

    let user_validator = SchemaValidator::new(user_schema);

    // Create test user object
    let mut user_map = IndexMap::new();
    user_map.insert(
        Value::String("name".to_string()),
        Value::String("Alice Johnson".to_string()),
    );
    user_map.insert(
        Value::String("email".to_string()),
        Value::String("alice@example.com".to_string()),
    );
    user_map.insert(Value::String("age".to_string()), Value::Int(30));
    let user_value = Value::Mapping(user_map);

    group.bench_function("simple_object_validation", |b| {
        b.iter(|| {
            user_validator
                .validate(std::hint::black_box(&user_value))
                .unwrap()
        });
    });

    // Create nested object schema
    let mut address_properties = HashMap::new();
    address_properties.insert("street".to_string(), Schema::with_type(ValueType::String));
    address_properties.insert("city".to_string(), Schema::with_type(ValueType::String));
    address_properties.insert("zip".to_string(), Schema::with_type(ValueType::String));

    let address_schema = Schema::with_type(ValueType::Object)
        .rule(SchemaRule::Properties(address_properties))
        .rule(SchemaRule::Required(vec![
            "street".to_string(),
            "city".to_string(),
        ]));

    let mut person_properties = HashMap::new();
    person_properties.insert("name".to_string(), Schema::with_type(ValueType::String));
    person_properties.insert("age".to_string(), Schema::with_type(ValueType::Integer));
    person_properties.insert("address".to_string(), address_schema);

    let person_schema = Schema::with_type(ValueType::Object)
        .rule(SchemaRule::Properties(person_properties))
        .rule(SchemaRule::Required(vec!["name".to_string()]));

    let person_validator = SchemaValidator::new(person_schema);

    // Create nested test object
    let mut address_map = IndexMap::new();
    address_map.insert(
        Value::String("street".to_string()),
        Value::String("123 Main St".to_string()),
    );
    address_map.insert(
        Value::String("city".to_string()),
        Value::String("Anytown".to_string()),
    );
    address_map.insert(
        Value::String("zip".to_string()),
        Value::String("12345".to_string()),
    );

    let mut person_map = IndexMap::new();
    person_map.insert(
        Value::String("name".to_string()),
        Value::String("John Doe".to_string()),
    );
    person_map.insert(Value::String("age".to_string()), Value::Int(35));
    person_map.insert(
        Value::String("address".to_string()),
        Value::Mapping(address_map),
    );
    let person_value = Value::Mapping(person_map);

    group.bench_function("nested_object_validation", |b| {
        b.iter(|| {
            person_validator
                .validate(std::hint::black_box(&person_value))
                .unwrap()
        });
    });

    group.finish();
}

fn bench_array_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("array_validation");

    // Simple array validation
    let number_list_schema = Schema::with_type(ValueType::Array).rule(SchemaRule::Items(Box::new(
        Schema::with_type(ValueType::Integer).rule(SchemaRule::Range {
            min: Some(0.0),
            max: Some(100.0),
        }),
    )));

    let array_validator = SchemaValidator::new(number_list_schema);

    // Create test arrays of different sizes
    let small_array = Value::Sequence(vec![Value::Int(10), Value::Int(20), Value::Int(30)]);

    let medium_array = Value::Sequence((0..50).map(Value::Int).collect());
    let large_array = Value::Sequence((0..200).map(Value::Int).collect());

    group.bench_function("small_array_validation", |b| {
        b.iter(|| {
            array_validator
                .validate(std::hint::black_box(&small_array))
                .unwrap()
        });
    });

    group.bench_function("medium_array_validation", |b| {
        b.iter(|| {
            array_validator
                .validate(std::hint::black_box(&medium_array))
                .unwrap()
        });
    });

    group.bench_function("large_array_validation", |b| {
        b.iter(|| {
            array_validator
                .validate(std::hint::black_box(&large_array))
                .unwrap()
        });
    });

    group.finish();
}

fn bench_complex_logical_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_logical_validation");

    // AnyOf validation
    let anyof_schema = Schema::new().rule(SchemaRule::AnyOf(vec![
        Schema::with_type(ValueType::String),
        Schema::with_type(ValueType::Integer),
        Schema::with_type(ValueType::Boolean),
    ]));
    let anyof_validator = SchemaValidator::new(anyof_schema);

    // AllOf validation
    let allof_schema = Schema::new().rule(SchemaRule::AllOf(vec![
        Schema::with_type(ValueType::String),
        Schema::new().rule(SchemaRule::Length {
            min: Some(5),
            max: Some(20),
        }),
        Schema::new().rule(SchemaRule::Pattern(Regex::new(r"^[a-zA-Z\s]+$").unwrap())),
    ]));
    let allof_validator = SchemaValidator::new(allof_schema);

    // OneOf validation
    let oneof_schema = Schema::new().rule(SchemaRule::OneOf(vec![
        Schema::with_type(ValueType::String).rule(SchemaRule::Length {
            min: None,
            max: Some(5),
        }),
        Schema::with_type(ValueType::String).rule(SchemaRule::Length {
            min: Some(10),
            max: None,
        }),
    ]));
    let oneof_validator = SchemaValidator::new(oneof_schema);

    // Test values
    let string_value = Value::String("hello world".to_string());
    let short_string = Value::String("hi".to_string());

    group.bench_function("anyof_validation", |b| {
        b.iter(|| {
            anyof_validator
                .validate(std::hint::black_box(&string_value))
                .unwrap()
        });
    });

    group.bench_function("allof_validation", |b| {
        b.iter(|| {
            allof_validator
                .validate(std::hint::black_box(&string_value))
                .unwrap()
        });
    });

    group.bench_function("oneof_validation", |b| {
        b.iter(|| {
            oneof_validator
                .validate(std::hint::black_box(&short_string))
                .unwrap()
        });
    });

    group.finish();
}

fn bench_yaml_integration_with_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("yaml_integration_validation");

    let yaml = Yaml::new();

    // Create comprehensive schema
    let mut user_properties = HashMap::new();
    user_properties.insert("name".to_string(), Schema::with_type(ValueType::String));
    user_properties.insert(
        "email".to_string(),
        Schema::with_type(ValueType::String).rule(SchemaRule::Pattern(
            Regex::new(r"^[^@]+@[^@]+\.[^@]+$").unwrap(),
        )),
    );
    user_properties.insert(
        "age".to_string(),
        Schema::with_type(ValueType::Integer).rule(SchemaRule::Range {
            min: Some(0.0),
            max: Some(150.0),
        }),
    );

    let user_schema = Schema::with_type(ValueType::Object)
        .rule(SchemaRule::Properties(user_properties))
        .rule(SchemaRule::Required(vec![
            "name".to_string(),
            "email".to_string(),
        ]));

    // Test YAML inputs of different complexities
    let simple_yaml = r#"
name: "Alice"
email: "alice@example.com"
age: 30
"#;

    let complex_yaml = r#"
name: "Bob Johnson"
email: "bob.johnson@company.example.com"
age: 45
active: true
roles:
  - "user"
  - "admin"
preferences:
  theme: "dark"
  notifications: true
  language: "en-US"
"#;

    group.bench_function("simple_yaml_with_validation", |b| {
        b.iter(|| {
            yaml.load_str_with_schema(
                std::hint::black_box(simple_yaml),
                std::hint::black_box(&user_schema),
            )
            .unwrap()
        });
    });

    group.bench_function("complex_yaml_with_validation", |b| {
        b.iter(|| {
            yaml.load_str_with_schema(
                std::hint::black_box(complex_yaml),
                std::hint::black_box(&user_schema),
            )
            .unwrap()
        });
    });

    // Multi-document validation
    let multi_doc_yaml = r#"
name: "User One"
email: "user1@example.com"
age: 25
---
name: "User Two"  
email: "user2@example.com"
age: 35
---
name: "User Three"
email: "user3@example.com"
age: 45
"#;

    group.bench_function("multi_document_validation", |b| {
        b.iter(|| {
            yaml.load_all_str_with_schema(
                std::hint::black_box(multi_doc_yaml),
                std::hint::black_box(&user_schema),
            )
            .unwrap()
        });
    });

    group.finish();
}

fn bench_validation_error_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation_error_handling");

    // Create schema that will generate validation errors
    let strict_schema = Schema::with_type(ValueType::Object)
        .rule(SchemaRule::Properties({
            let mut props = HashMap::new();
            props.insert(
                "name".to_string(),
                Schema::with_type(ValueType::String).rule(SchemaRule::Length {
                    min: Some(10),
                    max: Some(20),
                }),
            );
            props.insert(
                "email".to_string(),
                Schema::with_type(ValueType::String).rule(SchemaRule::Pattern(
                    Regex::new(r"^[a-z]+@[a-z]+\.com$").unwrap(),
                )),
            );
            props.insert(
                "age".to_string(),
                Schema::with_type(ValueType::Integer).rule(SchemaRule::Range {
                    min: Some(25.0),
                    max: Some(35.0),
                }),
            );
            props
        }))
        .rule(SchemaRule::Required(vec![
            "name".to_string(),
            "email".to_string(),
            "age".to_string(),
        ]));

    let validator = SchemaValidator::new(strict_schema);

    // Create invalid data
    let mut invalid_data = IndexMap::new();
    invalid_data.insert(
        Value::String("name".to_string()),
        Value::String("Al".to_string()),
    ); // Too short
    invalid_data.insert(
        Value::String("email".to_string()),
        Value::String("Not-An-Email".to_string()),
    ); // Invalid format
    invalid_data.insert(Value::String("age".to_string()), Value::Int(15)); // Out of range
    let invalid_value = Value::Mapping(invalid_data);

    group.bench_function("multiple_validation_errors", |b| {
        b.iter(|| {
            let _ = validator.validate(std::hint::black_box(&invalid_value));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_basic_type_validation,
    bench_constraint_validation,
    bench_object_validation,
    bench_array_validation,
    bench_complex_logical_validation,
    bench_yaml_integration_with_validation,
    bench_validation_error_handling
);

criterion_main!(benches);
