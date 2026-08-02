// SPDX-FileCopyrightText: YAML test suite contributors
//
// SPDX-License-Identifier: MIT

//! Convert rust-yaml parser events into the yaml-test-suite tree DSL.

use rust_yaml::parser::{BasicParser, EventType, Parser as ParserTrait, ScalarStyle};

use crate::escape::escape_value;

pub fn events_to_tree(input: &str) -> Result<Vec<String>, String> {
    let mut parser = BasicParser::new_eager(input.to_string());
    if let Some(err) = parser.take_scanning_error() {
        return Err(err.to_string());
    }
    drain_events(&mut parser)
}

/// Consume events from `parser`, converting each to a tree-DSL line.
///
/// Uses a `dyn` trait object rather than a generic so there is exactly one
/// concrete `drain_events` in the binary — that way the union of real-parser
/// and stand-in-parser tests covers every arm of the inner match (see
/// `ErrParser` in this module).
fn drain_events(parser: &mut dyn ParserTrait) -> Result<Vec<String>, String> {
    let mut lines = Vec::new();
    loop {
        match parser.get_event() {
            Ok(Some(event)) => lines.push(event_to_tree_line(&event.event_type)),
            Ok(None) => break,
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(lines)
}

pub fn event_to_tree_line(kind: &EventType) -> String {
    match kind {
        EventType::StreamStart => "+STR".to_string(),
        EventType::StreamEnd => "-STR".to_string(),
        EventType::DocumentStart { implicit, .. } => {
            if *implicit {
                "+DOC".to_string()
            } else {
                "+DOC ---".to_string()
            }
        }
        EventType::DocumentEnd { implicit } => {
            if *implicit {
                "-DOC".to_string()
            } else {
                "-DOC ...".to_string()
            }
        }
        EventType::MappingStart {
            anchor,
            tag,
            flow_style,
        } => collection_start("+MAP", "{}", *flow_style, anchor, tag),
        EventType::MappingEnd => "-MAP".to_string(),
        EventType::SequenceStart {
            anchor,
            tag,
            flow_style,
        } => collection_start("+SEQ", "[]", *flow_style, anchor, tag),
        EventType::SequenceEnd => "-SEQ".to_string(),
        EventType::Scalar {
            anchor,
            tag,
            value,
            style,
            ..
        } => {
            let mut s = "=VAL".to_string();
            append_anchor_and_tag(&mut s, anchor, tag);
            s.push(' ');
            s.push(style_char(*style));
            s.push_str(&escape_value(value));
            s
        }
        EventType::Alias { anchor } => format!("=ALI *{anchor}"),
    }
}

fn collection_start(
    head: &str,
    flow_marker: &str,
    flow_style: bool,
    anchor: &Option<String>,
    tag: &Option<String>,
) -> String {
    let mut s = head.to_string();
    if flow_style {
        s.push(' ');
        s.push_str(flow_marker);
    }
    append_anchor_and_tag(&mut s, anchor, tag);
    s
}

fn append_anchor_and_tag(out: &mut String, anchor: &Option<String>, tag: &Option<String>) {
    if let Some(a) = anchor {
        out.push_str(&format!(" &{a}"));
    }
    if let Some(t) = tag {
        out.push_str(&format!(" <{t}>"));
    }
}

fn style_char(style: ScalarStyle) -> char {
    match style {
        ScalarStyle::Plain => ':',
        ScalarStyle::SingleQuoted => '\'',
        ScalarStyle::DoubleQuoted => '"',
        ScalarStyle::Literal => '|',
        ScalarStyle::Folded => '>',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_start_renders_plus_str() {
        assert_eq!(event_to_tree_line(&EventType::StreamStart), "+STR");
    }

    #[test]
    fn stream_end_renders_minus_str() {
        assert_eq!(event_to_tree_line(&EventType::StreamEnd), "-STR");
    }

    fn doc_start(implicit: bool) -> EventType {
        EventType::DocumentStart {
            version: None,
            tags: Vec::new(),
            implicit,
        }
    }

    #[test]
    fn document_start_implicit_renders_plus_doc() {
        assert_eq!(event_to_tree_line(&doc_start(true)), "+DOC");
    }

    #[test]
    fn document_start_explicit_renders_plus_doc_dashes() {
        assert_eq!(event_to_tree_line(&doc_start(false)), "+DOC ---");
    }

    #[test]
    fn document_end_implicit_renders_minus_doc() {
        assert_eq!(
            event_to_tree_line(&EventType::DocumentEnd { implicit: true }),
            "-DOC"
        );
    }

    #[test]
    fn document_end_explicit_renders_minus_doc_dots() {
        assert_eq!(
            event_to_tree_line(&EventType::DocumentEnd { implicit: false }),
            "-DOC ..."
        );
    }

    fn map_start(anchor: Option<&str>, tag: Option<&str>, flow_style: bool) -> EventType {
        EventType::MappingStart {
            anchor: anchor.map(str::to_string),
            tag: tag.map(str::to_string),
            flow_style,
        }
    }

    #[test]
    fn mapping_start_block_renders_plus_map() {
        assert_eq!(event_to_tree_line(&map_start(None, None, false)), "+MAP");
    }

    #[test]
    fn mapping_start_flow_renders_plus_map_braces() {
        assert_eq!(event_to_tree_line(&map_start(None, None, true)), "+MAP {}");
    }

    #[test]
    fn mapping_start_with_anchor_appends_ampersand_name() {
        assert_eq!(
            event_to_tree_line(&map_start(Some("foo"), None, false)),
            "+MAP &foo"
        );
    }

    #[test]
    fn mapping_start_with_tag_appends_angle_tag() {
        assert_eq!(
            event_to_tree_line(&map_start(None, Some("tag:yaml.org,2002:map"), false)),
            "+MAP <tag:yaml.org,2002:map>"
        );
    }

    #[test]
    fn mapping_start_flow_anchor_tag_orders_brace_anchor_tag() {
        assert_eq!(
            event_to_tree_line(&map_start(Some("a"), Some("!t"), true)),
            "+MAP {} &a <!t>"
        );
    }

    #[test]
    fn mapping_end_renders_minus_map() {
        assert_eq!(event_to_tree_line(&EventType::MappingEnd), "-MAP");
    }

    fn seq_start(anchor: Option<&str>, tag: Option<&str>, flow_style: bool) -> EventType {
        EventType::SequenceStart {
            anchor: anchor.map(str::to_string),
            tag: tag.map(str::to_string),
            flow_style,
        }
    }

    #[test]
    fn sequence_start_block_renders_plus_seq() {
        assert_eq!(event_to_tree_line(&seq_start(None, None, false)), "+SEQ");
    }

    #[test]
    fn sequence_start_flow_renders_plus_seq_brackets() {
        assert_eq!(event_to_tree_line(&seq_start(None, None, true)), "+SEQ []");
    }

    #[test]
    fn sequence_start_with_anchor_appends_ampersand_name() {
        assert_eq!(
            event_to_tree_line(&seq_start(Some("a"), None, false)),
            "+SEQ &a"
        );
    }

    #[test]
    fn sequence_start_with_tag_appends_angle_tag() {
        assert_eq!(
            event_to_tree_line(&seq_start(None, Some("!t"), false)),
            "+SEQ <!t>"
        );
    }

    #[test]
    fn sequence_start_flow_anchor_tag_orders_bracket_anchor_tag() {
        assert_eq!(
            event_to_tree_line(&seq_start(Some("a"), Some("!t"), true)),
            "+SEQ [] &a <!t>"
        );
    }

    #[test]
    fn sequence_end_renders_minus_seq() {
        assert_eq!(event_to_tree_line(&EventType::SequenceEnd), "-SEQ");
    }

    use rust_yaml::parser::ScalarStyle;

    fn scalar(
        value: &str,
        style: ScalarStyle,
        anchor: Option<&str>,
        tag: Option<&str>,
    ) -> EventType {
        EventType::Scalar {
            anchor: anchor.map(str::to_string),
            tag: tag.map(str::to_string),
            value: value.to_string(),
            plain_implicit: matches!(style, ScalarStyle::Plain),
            quoted_implicit: !matches!(style, ScalarStyle::Plain),
            style,
        }
    }

    #[test]
    fn scalar_plain_uses_colon_style_char() {
        assert_eq!(
            event_to_tree_line(&scalar("42", ScalarStyle::Plain, None, None)),
            "=VAL :42"
        );
    }

    #[test]
    fn scalar_single_quoted_uses_apostrophe_style_char() {
        assert_eq!(
            event_to_tree_line(&scalar("x", ScalarStyle::SingleQuoted, None, None)),
            "=VAL 'x"
        );
    }

    #[test]
    fn scalar_double_quoted_uses_doublequote_style_char() {
        assert_eq!(
            event_to_tree_line(&scalar("x", ScalarStyle::DoubleQuoted, None, None)),
            "=VAL \"x"
        );
    }

    #[test]
    fn scalar_literal_uses_pipe_style_char() {
        assert_eq!(
            event_to_tree_line(&scalar("x", ScalarStyle::Literal, None, None)),
            "=VAL |x"
        );
    }

    #[test]
    fn scalar_folded_uses_gt_style_char() {
        assert_eq!(
            event_to_tree_line(&scalar("x", ScalarStyle::Folded, None, None)),
            "=VAL >x"
        );
    }

    #[test]
    fn scalar_with_anchor_inserts_anchor_before_style_char() {
        assert_eq!(
            event_to_tree_line(&scalar("v", ScalarStyle::Plain, Some("a"), None)),
            "=VAL &a :v"
        );
    }

    #[test]
    fn scalar_with_tag_inserts_tag_before_style_char() {
        assert_eq!(
            event_to_tree_line(&scalar("v", ScalarStyle::Plain, None, Some("!t"))),
            "=VAL <!t> :v"
        );
    }

    #[test]
    fn scalar_with_anchor_and_tag_orders_anchor_then_tag() {
        assert_eq!(
            event_to_tree_line(&scalar("v", ScalarStyle::Plain, Some("a"), Some("!t"))),
            "=VAL &a <!t> :v"
        );
    }

    #[test]
    fn scalar_value_is_escape_integrated() {
        assert_eq!(
            event_to_tree_line(&scalar("a\nb", ScalarStyle::Plain, None, None)),
            "=VAL :a\\nb"
        );
    }

    #[test]
    fn alias_renders_equal_ali_star_anchor() {
        assert_eq!(
            event_to_tree_line(&EventType::Alias {
                anchor: "ref".into()
            }),
            "=ALI *ref"
        );
    }

    #[test]
    fn events_to_tree_emits_full_event_stream_for_simple_scalar() {
        let lines = events_to_tree("42").expect("parse should succeed");
        assert_eq!(
            lines,
            vec![
                "+STR".to_string(),
                "+DOC".to_string(),
                "=VAL :42".to_string(),
                "-DOC".to_string(),
                "-STR".to_string(),
            ]
        );
    }

    #[test]
    fn events_to_tree_returns_err_when_scanner_rejects_reserved_indicator() {
        // `@` is a reserved indicator in YAML 1.2; the scanner rejects it
        // and the error surfaces via take_scanning_error.
        assert!(events_to_tree("@invalid").is_err());
    }

    #[test]
    fn events_to_tree_returns_err_when_yaml_directive_has_invalid_version() {
        assert!(events_to_tree("%YAML 99.99\n---\nfoo").is_err());
    }

    #[test]
    fn events_to_tree_on_empty_input_returns_minimal_stream() {
        let lines = events_to_tree("").expect("empty input should be Ok");
        assert!(!lines.is_empty(), "expected at least StreamStart/StreamEnd");
    }

    /// Stand-in parser that returns a single `Err` from `get_event`, used to
    /// exercise the parser-error branch of [`drain_events`] without depending
    /// on a pathological YAML input. We can't trivially craft such an input
    /// because rust-yaml currently surfaces most errors at scanner level.
    struct ErrParser {
        called: bool,
    }
    impl ParserTrait for ErrParser {
        fn check_event(&self) -> bool {
            !self.called
        }
        fn peek_event(&self) -> rust_yaml::Result<Option<&rust_yaml::parser::Event>> {
            Ok(None)
        }
        fn get_event(&mut self) -> rust_yaml::Result<Option<rust_yaml::parser::Event>> {
            self.called = true;
            Err(rust_yaml::Error::parse(
                rust_yaml::Position::start(),
                "synthetic",
            ))
        }
        fn reset(&mut self) {
            self.called = false;
        }
        fn position(&self) -> rust_yaml::Position {
            rust_yaml::Position::start()
        }
    }

    #[test]
    fn drain_events_returns_err_when_parser_emits_error() {
        let mut p = ErrParser { called: false };
        let result = drain_events(&mut p);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("synthetic"));
    }

    #[test]
    fn err_parser_stub_methods_satisfy_parser_trait() {
        // Cover the trait-fulfilment stubs so they aren't dead lines in
        // coverage reports.
        let mut p = ErrParser { called: false };
        assert!(p.check_event());
        assert!(p.peek_event().expect("no error").is_none());
        p.reset();
        assert!(p.check_event(), "reset should re-enable check_event");
        let _pos = p.position();
    }
}
