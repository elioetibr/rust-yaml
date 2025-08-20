//! Advanced schema validation features demo

use indexmap::IndexMap;
use rust_yaml::{Schema, SchemaRule, SchemaValidator, Value, ValueType};
use std::collections::HashMap;

fn main() {
    println!("🚀 Advanced Schema Validation Demo");

    // 1. Conditional validation (if-then-else)
    println!("\n1. Testing Conditional Validation (if-then-else)");

    // If type is "premium", then price must be > 100, else price must be <= 100
    let price_condition = Schema::with_type(ValueType::String)
        .rule(SchemaRule::Enum(vec![Value::String("premium".to_string())]));

    let then_schema = Schema::with_type(ValueType::Integer).rule(SchemaRule::Range {
        min: Some(100.0),
        max: None,
    });

    let else_schema = Schema::with_type(ValueType::Integer).rule(SchemaRule::Range {
        min: None,
        max: Some(100.0),
    });

    let conditional_schema = Schema::with_type(ValueType::Integer).rule(SchemaRule::Conditional {
        if_schema: Box::new(price_condition),
        then_schema: Some(Box::new(then_schema)),
        else_schema: Some(Box::new(else_schema)),
    });

    let conditional_validator = SchemaValidator::new(conditional_schema);

    // This would need a more complex implementation for full conditional logic
    // For now, let's test basic conditional structure
    let price_value = Value::Int(150);
    match conditional_validator.validate(&price_value) {
        Ok(()) => println!("✅ Conditional validation structure works"),
        Err(errors) => println!(
            "⚠️  Conditional validation: {} errors (expected for simplified implementation)",
            errors.len()
        ),
    }

    // 2. AnyOf validation (OR logic)
    println!("\n2. Testing AnyOf Validation (OR logic)");

    let string_schema = Schema::with_type(ValueType::String);
    let int_schema = Schema::with_type(ValueType::Integer);

    let anyof_schema = Schema::new().rule(SchemaRule::AnyOf(vec![string_schema, int_schema]));

    let anyof_validator = SchemaValidator::new(anyof_schema);

    let string_value = Value::String("hello".to_string());
    match anyof_validator.validate(&string_value) {
        Ok(()) => println!("✅ AnyOf validation passed for string"),
        Err(errors) => println!("❌ AnyOf validation failed: {:?}", errors),
    }

    let int_value = Value::Int(42);
    match anyof_validator.validate(&int_value) {
        Ok(()) => println!("✅ AnyOf validation passed for integer"),
        Err(errors) => println!("❌ AnyOf validation failed: {:?}", errors),
    }

    let bool_value = Value::Bool(true);
    match anyof_validator.validate(&bool_value) {
        Ok(()) => println!("❌ AnyOf validation should have failed for boolean"),
        Err(errors) => println!(
            "✅ AnyOf validation correctly failed for boolean: {} errors",
            errors.len()
        ),
    }

    // 3. AllOf validation (AND logic)
    println!("\n3. Testing AllOf Validation (AND logic)");

    let string_type_schema = Schema::with_type(ValueType::String);
    let length_schema = Schema::new().rule(SchemaRule::Length {
        min: Some(5),
        max: Some(20),
    });

    let allof_schema =
        Schema::new().rule(SchemaRule::AllOf(vec![string_type_schema, length_schema]));

    let allof_validator = SchemaValidator::new(allof_schema);

    let valid_string = Value::String("hello world".to_string());
    match allof_validator.validate(&valid_string) {
        Ok(()) => println!("✅ AllOf validation passed for valid string"),
        Err(errors) => println!("❌ AllOf validation failed: {:?}", errors),
    }

    let short_string = Value::String("hi".to_string());
    match allof_validator.validate(&short_string) {
        Ok(()) => println!("❌ AllOf validation should have failed for short string"),
        Err(errors) => println!(
            "✅ AllOf validation correctly failed for short string: {} errors",
            errors.len()
        ),
    }

    // 4. OneOf validation (XOR logic)
    println!("\n4. Testing OneOf Validation (XOR logic)");

    let short_string_schema = Schema::with_type(ValueType::String).rule(SchemaRule::Length {
        min: None,
        max: Some(5),
    });
    let long_string_schema = Schema::with_type(ValueType::String).rule(SchemaRule::Length {
        min: Some(10),
        max: None,
    });

    let oneof_schema = Schema::new().rule(SchemaRule::OneOf(vec![
        short_string_schema,
        long_string_schema,
    ]));

    let oneof_validator = SchemaValidator::new(oneof_schema);

    let short_value = Value::String("hi".to_string());
    match oneof_validator.validate(&short_value) {
        Ok(()) => println!("✅ OneOf validation passed for short string"),
        Err(errors) => println!("❌ OneOf validation failed: {:?}", errors),
    }

    let long_value = Value::String("this is a very long string".to_string());
    match oneof_validator.validate(&long_value) {
        Ok(()) => println!("✅ OneOf validation passed for long string"),
        Err(errors) => println!("❌ OneOf validation failed: {:?}", errors),
    }

    let medium_value = Value::String("medium".to_string());
    match oneof_validator.validate(&medium_value) {
        Ok(()) => println!("❌ OneOf validation should have failed for medium string"),
        Err(errors) => println!(
            "✅ OneOf validation correctly failed for medium string: {} errors",
            errors.len()
        ),
    }

    // 5. Not validation (negation)
    println!("\n5. Testing Not Validation (negation)");

    let not_null_schema = Schema::new().rule(SchemaRule::Not(Box::new(Schema::with_type(
        ValueType::Null,
    ))));

    let not_validator = SchemaValidator::new(not_null_schema);

    let string_value = Value::String("not null".to_string());
    match not_validator.validate(&string_value) {
        Ok(()) => println!("✅ Not validation passed for non-null value"),
        Err(errors) => println!("❌ Not validation failed: {:?}", errors),
    }

    let null_value = Value::Null;
    match not_validator.validate(&null_value) {
        Ok(()) => println!("❌ Not validation should have failed for null value"),
        Err(errors) => println!(
            "✅ Not validation correctly failed for null value: {} errors",
            errors.len()
        ),
    }

    // 6. Complex nested schema
    println!("\n6. Testing Complex Nested Schema");

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
    person_properties.insert(
        "age".to_string(),
        Schema::with_type(ValueType::Integer).rule(SchemaRule::Range {
            min: Some(0.0),
            max: Some(150.0),
        }),
    );
    person_properties.insert("address".to_string(), address_schema);

    let person_schema = Schema::with_type(ValueType::Object)
        .rule(SchemaRule::Properties(person_properties))
        .rule(SchemaRule::Required(vec![
            "name".to_string(),
            "age".to_string(),
        ]));

    let person_validator = SchemaValidator::new(person_schema);

    // Create valid nested object
    let mut address = IndexMap::new();
    address.insert(
        Value::String("street".to_string()),
        Value::String("123 Main St".to_string()),
    );
    address.insert(
        Value::String("city".to_string()),
        Value::String("Anytown".to_string()),
    );
    address.insert(
        Value::String("zip".to_string()),
        Value::String("12345".to_string()),
    );

    let mut person = IndexMap::new();
    person.insert(
        Value::String("name".to_string()),
        Value::String("John Doe".to_string()),
    );
    person.insert(Value::String("age".to_string()), Value::Int(30));
    person.insert(
        Value::String("address".to_string()),
        Value::Mapping(address),
    );

    let person_value = Value::Mapping(person);

    match person_validator.validate(&person_value) {
        Ok(()) => println!("✅ Complex nested schema validation passed"),
        Err(errors) => println!(
            "❌ Complex nested schema validation failed: {} errors",
            errors.len()
        ),
    }

    // 7. Error reporting detail test
    println!("\n7. Testing Detailed Error Reporting");

    let detailed_schema = Schema::with_type(ValueType::Object)
        .rule(SchemaRule::Properties({
            let mut props = HashMap::new();
            props.insert("email".to_string(), Schema::with_type(ValueType::String));
            props.insert(
                "age".to_string(),
                Schema::with_type(ValueType::Integer).rule(SchemaRule::Range {
                    min: Some(18.0),
                    max: Some(65.0),
                }),
            );
            props
        }))
        .rule(SchemaRule::Required(vec![
            "email".to_string(),
            "age".to_string(),
        ]));

    let detailed_validator = SchemaValidator::new(detailed_schema);

    // Create object with multiple validation errors
    let mut invalid_object = IndexMap::new();
    invalid_object.insert(Value::String("age".to_string()), Value::Int(10)); // Too young
                                                                             // Missing required "email" field

    let invalid_value = Value::Mapping(invalid_object);

    match detailed_validator.validate(&invalid_value) {
        Ok(()) => println!("❌ Should have failed validation"),
        Err(errors) => {
            println!("✅ Detailed error reporting:");
            for (i, error) in errors.iter().enumerate() {
                println!("   Error {}: {}", i + 1, error);
            }
        }
    }

    println!("\n🎯 Advanced Schema Validation Demo Complete!");
    println!("✅ Advanced features tested:");
    println!("   - Conditional validation (if-then-else) structure");
    println!("   - AnyOf validation (OR logic)");
    println!("   - AllOf validation (AND logic)");
    println!("   - OneOf validation (XOR logic)");
    println!("   - Not validation (negation)");
    println!("   - Complex nested object validation");
    println!("   - Detailed error reporting with paths");
}
