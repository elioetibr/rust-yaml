// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "serde")]
use indexmap::IndexMap;
use proptest::prelude::*;
use rust_yaml::Value;

fn arb_value() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(Value::Int),
        any::<f64>()
            .prop_filter("finite", |f| f.is_finite())
            .prop_map(Value::Float),
        // Restrict strings to printable ASCII to dodge YAML quoting quirks
        // unrelated to serde correctness.
        "[a-zA-Z0-9_]{0,16}".prop_map(Value::String),
    ];
    leaf.prop_recursive(3, 32, 8, |inner: BoxedStrategy<Value>| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..6).prop_map(Value::Sequence),
            prop::collection::vec(("[a-z]{1,8}", inner), 0..6).prop_map(|kvs| {
                let mut m = IndexMap::new();
                for (k, v) in kvs {
                    m.insert(Value::String(k), v);
                }
                Value::Mapping(m)
            }),
        ]
    })
}

proptest! {
    #[test]
    fn value_round_trips_through_serde(v in arb_value()) {
        let yaml = rust_yaml::to_string(&v).expect("dump");
        let back: Value = rust_yaml::from_str(&yaml).expect("load");
        prop_assert_eq!(v, back);
    }
}

// Parity vs serde_yaml: for each (yaml, expected_struct) pair, both
// rust_yaml::from_str and serde_yaml::from_str must produce the same value.
// This is the corpus referenced in the v1.1.0 acceptance for #21.

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq)]
struct Simple {
    name: String,
    version: u32,
    enabled: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq)]
struct Nested {
    items: Vec<String>,
    config: Simple,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq)]
enum Color {
    Red,
    Green,
    Rgb(u8, u8, u8),
    Named { name: String, hex: String },
}

fn parity<T>(yaml: &str)
where
    T: serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let ours: T =
        rust_yaml::from_str(yaml).unwrap_or_else(|e| panic!("rust_yaml failed on {yaml:?}: {e}"));
    let theirs: T =
        serde_yaml::from_str(yaml).unwrap_or_else(|e| panic!("serde_yaml failed on {yaml:?}: {e}"));
    assert_eq!(ours, theirs, "parity mismatch for {yaml:?}");
}

#[test]
fn parity_primitives() {
    parity::<bool>("true");
    parity::<bool>("false");
    parity::<i64>("0");
    parity::<i64>("42");
    parity::<i64>("-17");
    parity::<f64>("1.5");
    parity::<f64>("3.14");
    parity::<String>("hello");
    parity::<String>("\"with spaces\"");
    parity::<Option<i32>>("null");
    parity::<Option<i32>>("7");
}

#[test]
fn parity_sequences_and_maps() {
    parity::<Vec<i32>>("[1, 2, 3]");
    parity::<Vec<i32>>("- 1\n- 2\n- 3\n");
    parity::<Vec<Vec<i32>>>("- [1, 2]\n- [3, 4]\n");
    parity::<std::collections::BTreeMap<String, i32>>("a: 1\nb: 2\n");
}

#[test]
fn parity_struct() {
    parity::<Simple>("name: rust\nversion: 11\nenabled: true\n");
    parity::<Simple>("name: \"with: colon\"\nversion: 0\nenabled: false\n");
}

#[test]
fn parity_nested() {
    let y = "items:\n  - a\n  - b\nconfig:\n  name: rust\n  version: 11\n  enabled: true\n";
    parity::<Nested>(y);
}

#[test]
fn parity_enums() {
    // Only unit variants are tested: serde_yaml 0.9 uses !Tag notation for
    // tuple/struct variants while rust_yaml uses the single-entry-mapping form;
    // those non-unit formats are incompatible between the two libraries.
    parity::<Color>("Red");
    parity::<Color>("Green");
}

#[test]
fn parity_strings_edge_cases() {
    parity::<String>("\"\"");
    parity::<String>("\"123\"");
    parity::<String>("\"true\"");
    parity::<String>("\"null\"");
    parity::<Vec<String>>("- foo\n- bar\n- baz\n");
}

#[test]
fn parity_float_edge_cases() {
    parity::<f64>("0.0");
    parity::<f64>("-0.5");
    parity::<f64>("1.0e2");
}

#[test]
fn parity_maps_string_keys() {
    parity::<std::collections::BTreeMap<String, String>>("x: hello\ny: world\n");
    parity::<std::collections::BTreeMap<String, bool>>("flag: true\nother: false\n");
}

#[test]
fn parity_more_primitives() {
    parity::<u32>("0");
    parity::<u32>("4294967295");
    parity::<i32>("-2147483648");
    parity::<i32>("2147483647");
    parity::<()>("null");
    parity::<Option<String>>("null");
    parity::<Option<String>>("hello");
}

#[test]
fn parity_nested_options_and_collections() {
    parity::<Vec<Option<i32>>>("- 1\n- null\n- 3\n");
    parity::<Option<Vec<i32>>>("- 1\n- 2\n- 3\n");
    parity::<Option<Vec<i32>>>("null");
}
