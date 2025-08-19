//! YAML event definitions

use crate::Position;
use std::fmt;

/// Represents a YAML parsing event
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// The type of event
    pub event_type: EventType,
    /// Position where the event occurred
    pub position: Position,
}

/// Types of YAML parsing events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventType {
    /// Start of stream
    StreamStart,
    /// End of stream
    StreamEnd,

    /// Document start
    DocumentStart {
        /// YAML version
        version: Option<(u8, u8)>,
        /// Tag directives
        tags: Vec<(String, String)>,
        /// Implicit document start
        implicit: bool,
    },

    /// Document end
    DocumentEnd {
        /// Implicit document end
        implicit: bool,
    },

    /// Scalar value
    Scalar {
        /// Anchor name
        anchor: Option<String>,
        /// Tag
        tag: Option<String>,
        /// Value
        value: String,
        /// Plain style (unquoted)
        plain_implicit: bool,
        /// Quoted style implicit
        quoted_implicit: bool,
        /// Style
        style: ScalarStyle,
    },

    /// Sequence start
    SequenceStart {
        /// Anchor name
        anchor: Option<String>,
        /// Tag
        tag: Option<String>,
        /// Flow style
        flow_style: bool,
    },

    /// Sequence end
    SequenceEnd,

    /// Mapping start
    MappingStart {
        /// Anchor name
        anchor: Option<String>,
        /// Tag
        tag: Option<String>,
        /// Flow style
        flow_style: bool,
    },

    /// Mapping end
    MappingEnd,

    /// Alias reference
    Alias {
        /// Anchor name being referenced
        anchor: String,
    },
}

/// Scalar representation styles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarStyle {
    /// Plain style (no quotes)
    Plain,
    /// Single quoted
    SingleQuoted,
    /// Double quoted
    DoubleQuoted,
    /// Literal style (|)
    Literal,
    /// Folded style (>)
    Folded,
}

impl Event {
    /// Create a new event
    pub const fn new(event_type: EventType, position: Position) -> Self {
        Self {
            event_type,
            position,
        }
    }

    /// Create a stream start event
    pub const fn stream_start(position: Position) -> Self {
        Self::new(EventType::StreamStart, position)
    }

    /// Create a stream end event
    pub const fn stream_end(position: Position) -> Self {
        Self::new(EventType::StreamEnd, position)
    }

    /// Create a document start event
    pub const fn document_start(
        position: Position,
        version: Option<(u8, u8)>,
        tags: Vec<(String, String)>,
        implicit: bool,
    ) -> Self {
        Self::new(
            EventType::DocumentStart {
                version,
                tags,
                implicit,
            },
            position,
        )
    }

    /// Create a document end event
    pub const fn document_end(position: Position, implicit: bool) -> Self {
        Self::new(EventType::DocumentEnd { implicit }, position)
    }

    /// Create a scalar event
    pub const fn scalar(
        position: Position,
        anchor: Option<String>,
        tag: Option<String>,
        value: String,
        plain_implicit: bool,
        quoted_implicit: bool,
        style: ScalarStyle,
    ) -> Self {
        Self::new(
            EventType::Scalar {
                anchor,
                tag,
                value,
                plain_implicit,
                quoted_implicit,
                style,
            },
            position,
        )
    }

    /// Create a sequence start event
    pub const fn sequence_start(
        position: Position,
        anchor: Option<String>,
        tag: Option<String>,
        flow_style: bool,
    ) -> Self {
        Self::new(
            EventType::SequenceStart {
                anchor,
                tag,
                flow_style,
            },
            position,
        )
    }

    /// Create a sequence end event
    pub const fn sequence_end(position: Position) -> Self {
        Self::new(EventType::SequenceEnd, position)
    }

    /// Create a mapping start event
    pub const fn mapping_start(
        position: Position,
        anchor: Option<String>,
        tag: Option<String>,
        flow_style: bool,
    ) -> Self {
        Self::new(
            EventType::MappingStart {
                anchor,
                tag,
                flow_style,
            },
            position,
        )
    }

    /// Create a mapping end event
    pub const fn mapping_end(position: Position) -> Self {
        Self::new(EventType::MappingEnd, position)
    }

    /// Create an alias event
    pub const fn alias(position: Position, anchor: String) -> Self {
        Self::new(EventType::Alias { anchor }, position)
    }

    /// Check if this is a collection start event
    pub const fn is_collection_start(&self) -> bool {
        matches!(
            self.event_type,
            EventType::SequenceStart { .. } | EventType::MappingStart { .. }
        )
    }

    /// Check if this is a collection end event
    pub const fn is_collection_end(&self) -> bool {
        matches!(
            self.event_type,
            EventType::SequenceEnd | EventType::MappingEnd
        )
    }

    /// Check if this is a document boundary event
    pub const fn is_document_boundary(&self) -> bool {
        matches!(
            self.event_type,
            EventType::DocumentStart { .. } | EventType::DocumentEnd { .. }
        )
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.event_type {
            EventType::StreamStart => write!(f, "STREAM-START"),
            EventType::StreamEnd => write!(f, "STREAM-END"),
            EventType::DocumentStart { implicit, .. } => {
                if *implicit {
                    write!(f, "DOCUMENT-START (implicit)")
                } else {
                    write!(f, "DOCUMENT-START")
                }
            }
            EventType::DocumentEnd { implicit } => {
                if *implicit {
                    write!(f, "DOCUMENT-END (implicit)")
                } else {
                    write!(f, "DOCUMENT-END")
                }
            }
            EventType::Scalar { value, style, .. } => {
                write!(f, "SCALAR({}, {:?})", value, style)
            }
            EventType::SequenceStart { flow_style, .. } => {
                if *flow_style {
                    write!(f, "SEQUENCE-START (flow)")
                } else {
                    write!(f, "SEQUENCE-START (block)")
                }
            }
            EventType::SequenceEnd => write!(f, "SEQUENCE-END"),
            EventType::MappingStart { flow_style, .. } => {
                if *flow_style {
                    write!(f, "MAPPING-START (flow)")
                } else {
                    write!(f, "MAPPING-START (block)")
                }
            }
            EventType::MappingEnd => write!(f, "MAPPING-END"),
            EventType::Alias { anchor } => write!(f, "ALIAS({})", anchor),
        }
    }
}

impl fmt::Display for ScalarStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plain => write!(f, "plain"),
            Self::SingleQuoted => write!(f, "single-quoted"),
            Self::DoubleQuoted => write!(f, "double-quoted"),
            Self::Literal => write!(f, "literal"),
            Self::Folded => write!(f, "folded"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let pos = Position::at(1, 1, 0);

        let stream_start = Event::stream_start(pos);
        assert!(matches!(stream_start.event_type, EventType::StreamStart));
        assert_eq!(stream_start.position, pos);

        let scalar = Event::scalar(
            pos,
            None,
            None,
            "hello".to_string(),
            true,
            false,
            ScalarStyle::Plain,
        );

        if let EventType::Scalar { value, style, .. } = &scalar.event_type {
            assert_eq!(value, "hello");
            assert_eq!(*style, ScalarStyle::Plain);
        } else {
            panic!("Expected scalar event");
        }
    }

    #[test]
    fn test_event_type_checks() {
        let pos = Position::start();

        let seq_start = Event::sequence_start(pos, None, None, false);
        let seq_end = Event::sequence_end(pos);
        let doc_start = Event::document_start(pos, None, vec![], true);

        assert!(seq_start.is_collection_start());
        assert!(!seq_start.is_collection_end());

        assert!(!seq_end.is_collection_start());
        assert!(seq_end.is_collection_end());

        assert!(doc_start.is_document_boundary());
        assert!(!doc_start.is_collection_start());
    }

    #[test]
    fn test_event_display() {
        let pos = Position::start();

        let scalar = Event::scalar(
            pos,
            None,
            None,
            "test".to_string(),
            true,
            false,
            ScalarStyle::DoubleQuoted,
        );

        let display = format!("{}", scalar);
        assert!(display.contains("SCALAR"));
        assert!(display.contains("test"));
    }
}
