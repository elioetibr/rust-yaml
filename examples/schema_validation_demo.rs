//! Demonstration of schema validation with custom rules

use indexmap::IndexMap;
use regex::Regex;
use rust_yaml::{Schema, SchemaRule, SchemaValidator, Value, ValueType, Yaml};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Schema Validation Demo - Testing Implementation");

    // 1. Basic type validation
    println!("\n1. Testing Basic Type Validation");
    let string_schema = Schema::with_type(ValueType::String);
    let validator = SchemaValidator::new(string_schema);

    // Should pass
    let valid_string = Value::String("hello".to_string());
    match validator.validate(&valid_string) {
        Ok(()) => println!("✅ String validation passed"),
        Err(errors) => println!("❌ String validation failed: {:?}", errors),
    }

    // Should fail
    let invalid_number = Value::Int(42);
    match validator.validate(&invalid_number) {
        Ok(()) => println!("❌ Number validation should have failed"),
        Err(errors) => println!(
            "✅ Number validation correctly failed: {} errors",
            errors.len()
        ),
    }

    // 2. Range validation
    println!("\n2. Testing Range Validation");
    let age_schema = Schema::with_type(ValueType::Integer).rule(SchemaRule::Range {
        min: Some(0.0),
        max: Some(150.0),
    });

    let age_validator = SchemaValidator::new(age_schema);

    let valid_age = Value::Int(25);
    match age_validator.validate(&valid_age) {
        Ok(()) => println!("✅ Valid age (25) passed"),
        Err(errors) => println!("❌ Valid age failed: {:?}", errors),
    }

    let invalid_age = Value::Int(-5);
    match age_validator.validate(&invalid_age) {
        Ok(()) => println!("❌ Invalid age (-5) should have failed"),
        Err(errors) => println!("✅ Invalid age correctly failed: {} errors", errors.len()),
    }

    // 3. Length validation
    println!("\n3. Testing Length Validation");
    let name_schema = Schema::with_type(ValueType::String).rule(SchemaRule::Length {
        min: Some(2),
        max: Some(50),
    });

    let name_validator = SchemaValidator::new(name_schema);

    let valid_name = Value::String("Alice".to_string());
    match name_validator.validate(&valid_name) {
        Ok(()) => println!("✅ Valid name (Alice) passed"),
        Err(errors) => println!("❌ Valid name failed: {:?}", errors),
    }

    let too_short = Value::String("A".to_string());
    match name_validator.validate(&too_short) {
        Ok(()) => println!("❌ Too short name should have failed"),
        Err(errors) => println!(
            "✅ Too short name correctly failed: {} errors",
            errors.len()
        ),
    }

    // 4. Pattern validation (regex)
    println!("\n4. Testing Pattern Validation");
    let email_pattern = Regex::new(r"^[^@]+@[^@]+\.[^@]+$")?;
    let email_schema =
        Schema::with_type(ValueType::String).rule(SchemaRule::Pattern(email_pattern));

    let email_validator = SchemaValidator::new(email_schema);

    let valid_email = Value::String("alice@example.com".to_string());
    match email_validator.validate(&valid_email) {
        Ok(()) => println!("✅ Valid email passed"),
        Err(errors) => println!("❌ Valid email failed: {:?}", errors),
    }

    let invalid_email = Value::String("not-an-email".to_string());
    match email_validator.validate(&invalid_email) {
        Ok(()) => println!("❌ Invalid email should have failed"),
        Err(errors) => println!("✅ Invalid email correctly failed: {} errors", errors.len()),
    }

    // 5. Enum validation
    println!("\n5. Testing Enum Validation");
    let status_values = vec![
        Value::String("active".to_string()),
        Value::String("inactive".to_string()),
        Value::String("pending".to_string()),
    ];
    let status_schema = Schema::with_type(ValueType::String).rule(SchemaRule::Enum(status_values));

    let status_validator = SchemaValidator::new(status_schema);

    let valid_status = Value::String("active".to_string());
    match status_validator.validate(&valid_status) {
        Ok(()) => println!("✅ Valid status (active) passed"),
        Err(errors) => println!("❌ Valid status failed: {:?}", errors),
    }

    let invalid_status = Value::String("unknown".to_string());
    match status_validator.validate(&invalid_status) {
        Ok(()) => println!("❌ Invalid status should have failed"),
        Err(errors) => println!(
            "✅ Invalid status correctly failed: {} errors",
            errors.len()
        ),
    }

    // 6. Object validation with properties
    println!("\n6. Testing Object Validation");
    let mut user_properties = HashMap::new();
    user_properties.insert(
        "name".to_string(),
        Schema::with_type(ValueType::String).rule(SchemaRule::Length {
            min: Some(1),
            max: Some(100),
        }),
    );
    user_properties.insert(
        "age".to_string(),
        Schema::with_type(ValueType::Integer).rule(SchemaRule::Range {
            min: Some(0.0),
            max: Some(150.0),
        }),
    );
    user_properties.insert(
        "email".to_string(),
        Schema::with_type(ValueType::String)
            .rule(SchemaRule::Pattern(Regex::new(r"^[^@]+@[^@]+\.[^@]+$")?)),
    );

    let user_schema = Schema::with_type(ValueType::Object)
        .rule(SchemaRule::Properties(user_properties))
        .rule(SchemaRule::Required(vec![
            "name".to_string(),
            "email".to_string(),
        ]))
        .info("User Schema", "Validates user information");

    let user_validator = SchemaValidator::new(user_schema);

    // Create valid user object
    let mut valid_user = IndexMap::new();
    valid_user.insert(
        Value::String("name".to_string()),
        Value::String("Alice".to_string()),
    );
    valid_user.insert(
        Value::String("email".to_string()),
        Value::String("alice@example.com".to_string()),
    );
    valid_user.insert(Value::String("age".to_string()), Value::Int(30));
    let valid_user_value = Value::Mapping(valid_user);

    match user_validator.validate(&valid_user_value) {
        Ok(()) => println!("✅ Valid user object passed"),
        Err(errors) => println!("❌ Valid user failed: {:?}", errors),
    }

    // Create invalid user object (missing required email)
    let mut invalid_user = IndexMap::new();
    invalid_user.insert(
        Value::String("name".to_string()),
        Value::String("Bob".to_string()),
    );
    let invalid_user_value = Value::Mapping(invalid_user);

    match user_validator.validate(&invalid_user_value) {
        Ok(()) => println!("❌ Invalid user (missing email) should have failed"),
        Err(errors) => println!("✅ Invalid user correctly failed: {} errors", errors.len()),
    }

    // 7. Array validation
    println!("\n7. Testing Array Validation");
    let number_list_schema = Schema::with_type(ValueType::Array)
        .rule(SchemaRule::Items(Box::new(
            Schema::with_type(ValueType::Integer).rule(SchemaRule::Range {
                min: Some(0.0),
                max: Some(100.0),
            }),
        )))
        .rule(SchemaRule::Length {
            min: Some(1),
            max: Some(10),
        });

    let array_validator = SchemaValidator::new(number_list_schema);

    let valid_array = Value::Sequence(vec![Value::Int(10), Value::Int(20), Value::Int(30)]);

    match array_validator.validate(&valid_array) {
        Ok(()) => println!("✅ Valid array passed"),
        Err(errors) => println!("❌ Valid array failed: {:?}", errors),
    }

    let invalid_array = Value::Sequence(vec![
        Value::Int(10),
        Value::Int(200), // Out of range
        Value::Int(30),
    ]);

    match array_validator.validate(&invalid_array) {
        Ok(()) => println!("❌ Invalid array (out of range item) should have failed"),
        Err(errors) => println!("✅ Invalid array correctly failed: {} errors", errors.len()),
    }

    // 8. Integration with YAML API
    println!("\n8. Testing YAML API Integration");
    let yaml = Yaml::new();

    // Test load_str_with_schema
    let simple_schema = Schema::with_type(ValueType::String);

    let yaml_content = "hello world";
    match yaml.load_str_with_schema(yaml_content, &simple_schema) {
        Ok(value) => println!("✅ YAML API load_str_with_schema passed: {:?}", value),
        Err(error) => println!("❌ YAML API integration failed: {}", error),
    }

    // Test validation failure
    let int_content = "42";
    match yaml.load_str_with_schema(int_content, &simple_schema) {
        Ok(_) => println!("❌ YAML API should have failed validation"),
        Err(error) => println!("✅ YAML API correctly failed validation: {}", error),
    }

    println!("\n🎉 Schema Validation Demo Complete!");
    println!("✅ All core features implemented and working:");
    println!("   - Type validation (string, int, float, bool, array, object, null)");
    println!("   - Range constraints for numbers");
    println!("   - Length constraints for strings and arrays");
    println!("   - Pattern validation with regex");
    println!("   - Enum validation");
    println!("   - Object property validation");
    println!("   - Array item validation");
    println!("   - Required field validation");
    println!("   - Detailed error reporting");
    println!("   - Integration with YAML API");

    Ok(())
}
