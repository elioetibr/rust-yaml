// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tests for YAML directives (%YAML and %TAG)

use rust_yaml::{Value, Yaml};

#[test]
fn test_yaml_version_directive() {
    let yaml_input = r#"%YAML 1.2
---
foo: bar
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input).unwrap();

    // Should parse the document content correctly
    if let Value::Mapping(map) = result {
        assert_eq!(
            map.get(&Value::String("foo".to_string())),
            Some(&Value::String("bar".to_string()))
        );
    } else {
        panic!("Expected mapping");
    }
}

#[test]
fn test_tag_directive_basic() {
    let yaml_input = r#"%TAG ! tag:example.com,2024:
---
!person
name: John Doe
age: 30
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input);

    // Should handle tag directives (even if not fully resolved yet)
    assert!(result.is_ok(), "Should parse document with tag directive");
}

#[test]
fn test_multiple_tag_directives() {
    let yaml_input = r#"%TAG ! tag:example.com,2024:
%TAG !! tag:yaml.org,2002:
---
!!str "Hello"
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input);

    assert!(
        result.is_ok(),
        "Should parse document with multiple tag directives"
    );
}

#[test]
fn test_directives_with_multiple_documents() {
    let yaml_input = r#"%YAML 1.2
---
doc1: value1
...
%YAML 1.2
---
doc2: value2
"#;

    let yaml = Yaml::new();

    // Load all documents
    let documents = yaml.load_all_str(yaml_input).unwrap();

    assert_eq!(documents.len(), 2, "Should parse two documents");

    // Check first document
    if let Value::Mapping(map) = &documents[0] {
        assert_eq!(
            map.get(&Value::String("doc1".to_string())),
            Some(&Value::String("value1".to_string()))
        );
    } else {
        panic!("Expected mapping for first document");
    }

    // Check second document
    if let Value::Mapping(map) = &documents[1] {
        assert_eq!(
            map.get(&Value::String("doc2".to_string())),
            Some(&Value::String("value2".to_string()))
        );
    } else {
        panic!("Expected mapping for second document");
    }
}

#[test]
fn test_implicit_document_with_directives() {
    // Document without explicit --- should still work with directives
    let yaml_input = r#"%YAML 1.2
%TAG ! tag:example.com,2024:
key: value
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input).unwrap();

    if let Value::Mapping(map) = result {
        assert_eq!(
            map.get(&Value::String("key".to_string())),
            Some(&Value::String("value".to_string()))
        );
    } else {
        panic!("Expected mapping");
    }
}

#[test]
fn test_tag_directive_with_handle() {
    let yaml_input = r#"%TAG !ex! tag:example.com,2024:
---
!ex!widget
id: 123
type: button
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input);

    assert!(
        result.is_ok(),
        "Should parse document with named tag handle"
    );
}

#[test]
fn test_directives_only_apply_to_next_document() {
    let yaml_input = r#"%YAML 1.2
%TAG ! tag:example.com,2024:
---
doc1: with_directives
...
---
doc2: without_directives
"#;

    let yaml = Yaml::new();
    let documents = yaml.load_all_str(yaml_input).unwrap();

    assert_eq!(documents.len(), 2, "Should parse both documents");

    // Both documents should parse correctly
    // The directives only apply to the first document
    for (i, doc) in documents.iter().enumerate() {
        if let Value::Mapping(_) = doc {
            // Good - parsed as mapping
        } else {
            panic!("Document {} should be a mapping", i + 1);
        }
    }
}

#[test]
fn test_yaml_version_1_1_compatibility() {
    // Under %YAML 1.1, `yes`/`no` are booleans, not strings.
    let yaml_input = r#"%YAML 1.1
---
yes: true
no: false
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input).expect("parse YAML 1.1 document");

    let Value::Mapping(map) = result else {
        panic!("expected mapping, got {result:?}");
    };

    // Both keys and values are booleans under 1.1.
    let yes_val = map
        .get(&Value::Bool(true))
        .expect("`yes` key as Bool(true)");
    assert_eq!(yes_val, &Value::Bool(true));
    let no_val = map
        .get(&Value::Bool(false))
        .expect("`no` key as Bool(false)");
    assert_eq!(no_val, &Value::Bool(false));
    assert_eq!(map.len(), 2, "exactly two entries");
}

#[test]
fn test_yaml_1_2_default_treats_legacy_bools_as_strings() {
    // No %YAML directive => 1.2 semantics. yes/no are plain strings.
    let yaml_input = "yes: hello\nno: world\n";

    let yaml = Yaml::new();
    let Value::Mapping(map) = yaml.load_str(yaml_input).expect("parse") else {
        panic!("expected mapping");
    };

    assert_eq!(
        map.get(&Value::String("yes".to_string())),
        Some(&Value::String("hello".to_string())),
        "`yes` is a string key under 1.2 default"
    );
    assert_eq!(
        map.get(&Value::String("no".to_string())),
        Some(&Value::String("world".to_string())),
        "`no` is a string key under 1.2 default"
    );
    assert!(
        map.get(&Value::Bool(true)).is_none(),
        "no Bool(true) key should be created"
    );
}

#[test]
fn test_yaml_1_2_explicit_directive_keeps_1_2_semantics() {
    // %YAML 1.2 is the spec default — yes/no should remain strings.
    let yaml_input = "%YAML 1.2\n---\nyes: hi\n";

    let yaml = Yaml::new();
    let Value::Mapping(map) = yaml.load_str(yaml_input).expect("parse") else {
        panic!("expected mapping");
    };
    assert_eq!(
        map.get(&Value::String("yes".to_string())),
        Some(&Value::String("hi".to_string())),
        "%YAML 1.2 keeps `yes` as string"
    );
}

#[test]
fn test_yaml_version_directive_does_not_carry_across_documents() {
    // Per YAML 1.2.2 spec §9.1.3, directives apply only to the document that
    // follows them. A second document without a directive must use the
    // default version (1.2).
    let yaml_input = r#"%YAML 1.1
---
flag: yes
---
flag: yes
"#;

    let yaml = Yaml::new();
    let docs = yaml
        .load_all_str(yaml_input)
        .expect("parse multi-doc stream");
    assert_eq!(docs.len(), 2, "two documents in stream");

    // Doc 1: %YAML 1.1 in effect, `yes` is bool.
    let Value::Mapping(first_doc) = &docs[0] else {
        panic!("doc1 expected mapping");
    };
    assert_eq!(
        first_doc.get(&Value::String("flag".to_string())),
        Some(&Value::Bool(true)),
        "doc1 under 1.1: yes is Bool(true)"
    );

    // Doc 2: no directive => default 1.2, `yes` is string.
    let Value::Mapping(second_doc) = &docs[1] else {
        panic!("doc2 expected mapping");
    };
    assert_eq!(
        second_doc.get(&Value::String("flag".to_string())),
        Some(&Value::String("yes".to_string())),
        "doc2 reverts to 1.2 default: yes is String"
    );
}

#[test]
fn test_yaml_1_1_value_tag_token_rejected() {
    // Under `%YAML 1.1`, a bare `=` plain scalar is the
    // `tag:yaml.org,2002:value` indicator (§10.3.4) and is rejected
    // by every composer via `resolver::value_tag_error`. This mirrors
    // `ruamel.yaml typ="safe"`/`typ="unsafe"`. See `src/resolver.rs`
    // and the same end-to-end coverage in
    // `src/constructor.rs::test_yaml_1_1_value_tag_rejected_with_directive`.
    let yaml_input = "%YAML 1.1\n---\nitems:\n  - =\n";
    let yaml = Yaml::new();
    let err = yaml
        .load_str(yaml_input)
        .expect_err("`=` under %YAML 1.1 must error");
    let msg = err.to_string();
    assert!(
        msg.contains("tag:yaml.org,2002:value"),
        "error should mention the value tag URI: {msg}"
    );
}

#[test]
fn test_directive_scanner_integration() {
    // Verifies that `%YAML` / `%TAG` directive headers are scanned and
    // passed through to the parser without erroring. Uses only
    // *definitions* of named handles — not their consumption in
    // mapping keys, which exercises a known scanner bug tracked in
    // `test_named_tag_handle_as_mapping_key_known_bug` below.
    let yaml_input = r#"%YAML 1.2
%TAG !foo! tag:example.com,2024/foo:
%TAG !bar! tag:example.com,2024/bar:
---
regular: value
other: another
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input);

    assert!(
        result.is_ok(),
        "should parse a document with multiple unused %TAG directives"
    );
}

/// `scan_tag` must terminate the tag suffix when `:` is followed by
/// whitespace / EOL — that `:` is the YAML mapping-value indicator
/// (§6.8.2), not a URI sub-delimiter. Without this carve-out,
/// `!handle!suffix: value` is mis-scanned as
/// `Tag("!handle!suffix:") Scalar("value")` and the surrounding
/// mapping structure collapses (symptom:
/// `Mapping key not followed by ':'`).
///
/// Mirrors the `,`-in-flow carve-out for the same reason: a URI
/// character that's overloaded as a YAML indicator in some
/// contexts.
#[test]
fn test_named_tag_handle_as_mapping_key() {
    let yaml_input = r#"%YAML 1.2
%TAG !foo! tag:example.com,2024/foo:
---
!foo!widget: component
other: another
"#;
    let yaml = Yaml::new();
    let result = yaml
        .load_str(yaml_input)
        .expect("named tag handle as implicit-key tag must parse");
    let Value::Mapping(map) = result else {
        panic!("expected mapping, got {result:?}");
    };
    // The tag `!foo!widget` applies to the implicit empty scalar
    // between the tag and the `:`. Tagged scalars bypass the
    // implicit-resolution empty→Null rule (the tag overrides
    // implicit typing per §10.1.2) and `compose_tagged_scalar`
    // preserves the raw value as a `String` for any unknown custom
    // tag — `Value` has no `Tagged` variant to carry the tag URI
    // forward. So the first entry's key is `String("")`; the
    // untagged second entry parses normally.
    assert_eq!(
        map.get(&Value::String(String::new())),
        Some(&Value::String("component".to_string())),
        "key from `!foo!widget: component` should be the empty-string scalar the tag was applied to"
    );
    assert_eq!(
        map.get(&Value::String("other".to_string())),
        Some(&Value::String("another".to_string())),
        "untagged sibling key should still parse"
    );
}

/// Companion to the test above: ensure `:` *inside* a URI (no
/// trailing whitespace) is still consumed into the tag suffix,
/// preserving real-world tag forms like `tag:yaml.org,2002:str`.
#[test]
fn test_colon_inside_tag_uri_is_part_of_suffix() {
    // `!foo!bar:baz val` — `:` is followed by `b`, not whitespace,
    // so the tag suffix is `bar:baz` (a valid URI fragment).
    let yaml_input = r#"%YAML 1.2
%TAG !foo! tag:example.com,
---
!foo!bar:baz val
"#;
    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input).expect("parse");
    // `!foo!bar:baz val` is a tag applied to plain scalar `val`,
    // and the document has only one node (no mapping).
    assert_eq!(result, Value::String("val".to_string()));
}
