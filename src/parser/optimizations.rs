//! Parser optimizations for improved performance

use crate::{
    Error, Result,
    parser::{Event, EventType, ParserState},
    profiling::YamlProfiler,
};

/// Optimized event buffer for parser performance
#[derive(Debug)]
pub struct EventBuffer {
    events: Vec<Event>,
    capacity: usize,
    profiler: Option<YamlProfiler>,
}

impl EventBuffer {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            capacity: 128,
            profiler: std::env::var("RUST_YAML_PROFILE")
                .ok()
                .map(|_| YamlProfiler::new()),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: Vec::with_capacity(capacity),
            capacity,
            profiler: std::env::var("RUST_YAML_PROFILE")
                .ok()
                .map(|_| YamlProfiler::new()),
        }
    }

    pub fn push(&mut self, event: Event) {
        if let Some(ref mut profiler) = self.profiler {
            profiler.time_operation("event_buffer_push", || {
                self.events.push(event);
            });
        } else {
            self.events.push(event);
        }
    }

    pub fn get(&self, index: usize) -> Option<&Event> {
        self.events.get(index)
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Pre-allocate space for expected number of events
    pub fn reserve(&mut self, additional: usize) {
        self.events.reserve(additional);
    }

    /// Shrink buffer if using too much memory
    pub fn shrink_if_needed(&mut self) {
        if self.events.capacity() > self.capacity * 2 && self.events.is_empty() {
            self.events.shrink_to(self.capacity);
        }
    }
}

impl Default for EventBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// State machine optimization for parser
#[derive(Debug, Clone)]
pub struct ParserStateMachine {
    states: Vec<ParserState>,
    current_state: ParserState,
    profiler: Option<YamlProfiler>,
}

impl ParserStateMachine {
    pub fn new() -> Self {
        Self {
            states: Vec::with_capacity(16), // Pre-allocate state stack
            current_state: ParserState::StreamStart,
            profiler: std::env::var("RUST_YAML_PROFILE")
                .ok()
                .map(|_| YamlProfiler::new()),
        }
    }

    pub fn push_state(&mut self, state: ParserState) {
        if let Some(ref mut profiler) = self.profiler {
            profiler.time_operation("state_push", || {
                self.states.push(self.current_state.clone());
                self.current_state = state;
            });
        } else {
            self.states.push(self.current_state.clone());
            self.current_state = state;
        }
    }

    pub fn pop_state(&mut self) -> Option<ParserState> {
        if let Some(ref mut profiler) = self.profiler {
            profiler.time_operation("state_pop", || {
                if let Some(prev_state) = self.states.pop() {
                    let current = std::mem::replace(&mut self.current_state, prev_state);
                    Some(current)
                } else {
                    None
                }
            })
        } else {
            if let Some(prev_state) = self.states.pop() {
                let current = std::mem::replace(&mut self.current_state, prev_state);
                Some(current)
            } else {
                None
            }
        }
    }

    pub fn current_state(&self) -> &ParserState {
        &self.current_state
    }

    pub fn set_state(&mut self, state: ParserState) {
        self.current_state = state;
    }

    pub fn depth(&self) -> usize {
        self.states.len()
    }

    /// Check if we're in a flow context (optimization for flow parsing)
    pub fn is_flow_context(&self) -> bool {
        matches!(
            self.current_state,
            ParserState::FlowSequence { .. } | ParserState::FlowMapping { .. }
        )
    }

    /// Check if we're in a block context
    pub fn is_block_context(&self) -> bool {
        matches!(
            self.current_state,
            ParserState::BlockSequence
                | ParserState::BlockMappingKey
                | ParserState::BlockMappingValue
        )
    }
}

impl Default for ParserStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

/// Fast event creation utilities
pub struct EventFactory {
    profiler: Option<YamlProfiler>,
}

impl EventFactory {
    pub fn new() -> Self {
        Self {
            profiler: std::env::var("RUST_YAML_PROFILE")
                .ok()
                .map(|_| YamlProfiler::new()),
        }
    }

    /// Create a scalar event with minimal allocations
    pub fn create_scalar(
        &mut self,
        value: String,
        tag: Option<String>,
        anchor: Option<String>,
        style: crate::parser::ScalarStyle,
    ) -> Event {
        let event_type = EventType::Scalar {
            value,
            tag,
            anchor,
            plain_implicit: true,
            quoted_implicit: true,
            style,
        };

        if let Some(ref mut profiler) = self.profiler {
            profiler.time_operation("create_scalar_event", || {
                Event::new(event_type.clone(), crate::Position::new())
            })
        } else {
            Event::new(event_type, crate::Position::new())
        }
    }

    /// Create a mapping start event
    pub fn create_mapping_start(
        &mut self,
        tag: Option<String>,
        anchor: Option<String>,
        flow_style: bool,
    ) -> Event {
        Event::new(
            EventType::MappingStart {
                tag,
                anchor,
                flow_style,
            },
            crate::Position::new(),
        )
    }

    /// Create a sequence start event
    pub fn create_sequence_start(
        &mut self,
        tag: Option<String>,
        anchor: Option<String>,
        flow_style: bool,
    ) -> Event {
        Event::new(
            EventType::SequenceStart {
                tag,
                anchor,
                flow_style,
            },
            crate::Position::new(),
        )
    }

    /// Create an alias event
    pub fn create_alias(&mut self, anchor: String) -> Event {
        Event::new(EventType::Alias { anchor }, crate::Position::new())
    }
}

impl Default for EventFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Lazy parsing optimization - parse events on demand
pub struct LazyEventIterator {
    parser: Box<dyn Iterator<Item = Result<Event>>>,
    buffer: Vec<Event>,
    buffer_index: usize,
    done: bool,
    profiler: Option<YamlProfiler>,
}

impl LazyEventIterator {
    pub fn new<P>(parser: P) -> Self
    where
        P: Iterator<Item = Result<Event>> + 'static,
    {
        Self {
            parser: Box::new(parser),
            buffer: Vec::new(),
            buffer_index: 0,
            done: false,
            profiler: std::env::var("RUST_YAML_PROFILE")
                .ok()
                .map(|_| YamlProfiler::new()),
        }
    }

    /// Get next event, buffering for efficiency
    pub fn next_event(&mut self) -> Result<Option<Event>> {
        let use_profiler = self.profiler.is_some();
        if use_profiler {
            let result = self.next_event_impl();
            if let Some(ref mut profiler) = self.profiler {
                profiler.record_memory(
                    "lazy_next_event",
                    self.buffer.len() * std::mem::size_of::<Event>(),
                );
            }
            result
        } else {
            self.next_event_impl()
        }
    }

    fn next_event_impl(&mut self) -> Result<Option<Event>> {
        if self.done {
            return Ok(None);
        }

        if self.buffer_index < self.buffer.len() {
            let event = self.buffer[self.buffer_index].clone();
            self.buffer_index += 1;
            return Ok(Some(event));
        }

        // Fill buffer with next batch of events
        self.buffer.clear();
        self.buffer_index = 0;

        const BUFFER_SIZE: usize = 16; // Process events in batches
        for _ in 0..BUFFER_SIZE {
            match self.parser.next() {
                Some(Ok(event)) => {
                    let is_stream_end = matches!(event.event_type, EventType::StreamEnd);
                    self.buffer.push(event);
                    if is_stream_end {
                        self.done = true;
                        break;
                    }
                }
                Some(Err(e)) => return Err(e),
                None => {
                    self.done = true;
                    break;
                }
            }
        }

        if self.buffer.is_empty() {
            Ok(None)
        } else {
            let event = self.buffer[0].clone();
            self.buffer_index = 1;
            Ok(Some(event))
        }
    }

    /// Peek at next event without consuming it
    pub fn peek_event(&mut self) -> Result<Option<&Event>> {
        if self.buffer_index < self.buffer.len() {
            return Ok(Some(&self.buffer[self.buffer_index]));
        }

        // Try to get next event into buffer
        self.next_event()?;
        if self.buffer_index > 0 {
            self.buffer_index -= 1; // Move back to make it available for peek
            Ok(Some(&self.buffer[self.buffer_index]))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ScalarStyle;

    #[test]
    fn test_event_buffer() {
        let mut buffer = EventBuffer::new();
        assert!(buffer.is_empty());

        let event = Event::new(EventType::StreamStart);
        buffer.push(event);
        assert_eq!(buffer.len(), 1);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_parser_state_machine() {
        let mut state_machine = ParserStateMachine::new();
        assert!(matches!(
            state_machine.current_state(),
            ParserState::StreamStart
        ));

        state_machine.push_state(ParserState::BlockSequence);
        assert!(matches!(
            state_machine.current_state(),
            ParserState::BlockSequence
        ));
        assert_eq!(state_machine.depth(), 1);

        let prev_state = state_machine.pop_state();
        assert!(matches!(prev_state, Some(ParserState::BlockSequence)));
        assert!(matches!(
            state_machine.current_state(),
            ParserState::StreamStart
        ));
    }

    #[test]
    fn test_event_factory() {
        let mut factory = EventFactory::new();

        let scalar_event =
            factory.create_scalar("test value".to_string(), None, None, ScalarStyle::Plain);

        if let EventType::Scalar { value, .. } = scalar_event.event_type {
            assert_eq!(value, "test value");
        } else {
            panic!("Expected scalar event");
        }
    }

    #[test]
    fn test_state_machine_context_detection() {
        let mut state_machine = ParserStateMachine::new();

        state_machine.set_state(ParserState::BlockSequence);
        assert!(state_machine.is_block_context());
        assert!(!state_machine.is_flow_context());

        state_machine.set_state(ParserState::FlowSequence { first: true });
        assert!(state_machine.is_flow_context());
        assert!(!state_machine.is_block_context());
    }
}
