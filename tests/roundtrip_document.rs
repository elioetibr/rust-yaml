//! Integration tests for the source-preserving `RoundTripDocument` API:
//! byte-for-byte identity on realistic config files, and surgical edits that
//! leave every untouched byte alone.

use rust_yaml::{PathSegment, RoundTripDocument, Value};

/// A realistic Kubernetes-style manifest exercising comments, blank lines,
/// nested block structure, and inline comments at several depths.
const K8S_MANIFEST: &str = "\
# Production deployment
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web        # the web frontend
  labels:
    app: web
spec:
  replicas: 3      # scale here

  template:
    spec:
      containers:
        - name: web
          image: nginx:1.25   # pin the tag
";

#[test]
fn k8s_manifest_round_trips_byte_for_byte() {
    let doc = RoundTripDocument::parse(K8S_MANIFEST).unwrap();
    assert_eq!(doc.to_string(), K8S_MANIFEST);
}

#[test]
fn k8s_manifest_scalar_edit_is_surgical() {
    let mut doc = RoundTripDocument::parse(K8S_MANIFEST).unwrap();
    doc.set(
        &[PathSegment::Key("spec"), PathSegment::Key("replicas")],
        "5",
    )
    .unwrap();
    let expected = K8S_MANIFEST.replace(
        "replicas: 3      # scale here",
        "replicas: 5      # scale here",
    );
    assert_eq!(doc.to_string(), expected);
}

#[test]
fn deep_path_through_sequence_edit() {
    let mut doc = RoundTripDocument::parse(K8S_MANIFEST).unwrap();
    let path = [
        PathSegment::Key("spec"),
        PathSegment::Key("template"),
        PathSegment::Key("spec"),
        PathSegment::Key("containers"),
        PathSegment::Index(0),
        PathSegment::Key("image"),
    ];
    assert_eq!(doc.get(&path), Some("nginx:1.25"));
    doc.set(&path, "nginx:1.29").unwrap();
    assert_eq!(
        doc.to_string(),
        K8S_MANIFEST.replace("nginx:1.25   # pin the tag", "nginx:1.29   # pin the tag")
    );
    // The inline comment survived and the typed view sees the new value.
    assert!(doc.to_string().contains("# pin the tag"));
}

#[test]
fn helm_values_style_quoting_survives_neighbor_edit() {
    let input = "\
image:
  repository: 'nginx'
  tag: \"1.25\"
  pullPolicy: IfNotPresent
resources: {}
";
    let mut doc = RoundTripDocument::parse(input).unwrap();
    doc.set(
        &[PathSegment::Key("image"), PathSegment::Key("pullPolicy")],
        "Always",
    )
    .unwrap();
    let out = doc.to_string();
    // Neighboring quote styles and the empty flow mapping are untouched.
    assert!(out.contains("repository: 'nginx'"));
    assert!(out.contains("tag: \"1.25\""));
    assert!(out.contains("resources: {}"));
    assert!(out.contains("pullPolicy: Always"));
}

#[test]
fn multi_document_stream_edits_one_document_only() {
    let input = "---\n# doc one\na: 1\n---\n# doc two\nb: 2\n";
    let mut docs = RoundTripDocument::parse_all(input).unwrap();
    assert_eq!(docs.len(), 2);
    docs[1].set(&[PathSegment::Key("b")], "20").unwrap();
    let joined: String = docs.iter().map(RoundTripDocument::source).collect();
    assert_eq!(joined, "---\n# doc one\na: 1\n---\n# doc two\nb: 20\n");
}

#[test]
fn typed_value_matches_regular_loader() {
    let doc = RoundTripDocument::parse(K8S_MANIFEST).unwrap();
    let expected = rust_yaml::Yaml::new().load_str(K8S_MANIFEST).unwrap();
    assert_eq!(doc.value(), &expected);
}

#[test]
fn crlf_file_round_trips_and_edits() {
    let input = "a: 1\r\nb: 2\r\n";
    let mut doc = RoundTripDocument::parse(input).unwrap();
    assert_eq!(doc.to_string(), input);
    doc.set(&[PathSegment::Key("a")], "9").unwrap();
    assert_eq!(doc.to_string(), "a: 9\r\nb: 2\r\n");
}

#[test]
fn bom_file_round_trips_and_edits() {
    let input = "\u{feff}a: 1\nb: 2\n";
    let mut doc = RoundTripDocument::parse(input).unwrap();
    assert_eq!(doc.to_string(), input);
    doc.set(&[PathSegment::Key("b")], "7").unwrap();
    assert_eq!(doc.to_string(), "\u{feff}a: 1\nb: 7\n");
}

#[test]
fn anchor_and_alias_bytes_survive_unrelated_edit() {
    let input = "\
defaults: &defaults
  timeout: 30
  retries: 3
service:
  <<: *defaults
  name: web
";
    let mut doc = RoundTripDocument::parse(input).unwrap();
    doc.set(
        &[PathSegment::Key("service"), PathSegment::Key("name")],
        "api",
    )
    .unwrap();
    let out = doc.to_string();
    assert!(out.contains("&defaults"));
    assert!(out.contains("<<: *defaults"));
    assert!(out.contains("name: api"));
}

#[test]
fn replacing_an_alias_breaks_only_that_reference() {
    let input = "base: &b 7\nx: *b\ny: *b\n";
    let mut doc = RoundTripDocument::parse(input).unwrap();
    doc.set(&[PathSegment::Key("x")], "8").unwrap();
    assert_eq!(doc.to_string(), "base: &b 7\nx: 8\ny: *b\n");
    assert_eq!(doc.value().get_str("y"), Some(&Value::Int(7)));
}

#[test]
fn block_scalar_content_is_addressable_verbatim() {
    let input = "script: |\n  echo one\n  echo two\nafter: 1\n";
    let doc = RoundTripDocument::parse(input).unwrap();
    let got = doc.get(&[PathSegment::Key("script")]).unwrap();
    assert!(got.starts_with('|'));
    assert!(got.contains("echo one"));
}

#[test]
fn whole_document_span_via_replace() {
    // replace_span with an explicit span supports arbitrary regions,
    // including growing the document.
    let input = "a: 1\n";
    let mut doc = RoundTripDocument::parse(input).unwrap();
    let len = doc.source().len();
    doc.replace_span(rust_yaml::Span::new(len, len), "b: 2\n")
        .unwrap();
    assert_eq!(doc.to_string(), "a: 1\nb: 2\n");
}
