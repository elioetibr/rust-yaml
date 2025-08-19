//! Scanner state management

use crate::Position;

/// Scanner state for different contexts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum ScannerState {
    StreamStart,
    StreamEnd,
    DocumentStart,
    DocumentContent,
    BlockMapping,
    BlockSequence,
    FlowMapping,
    FlowSequence,
    Scalar,
    Tag,
    Anchor,
    Alias,
}

/// Scanner context manager
#[derive(Debug)]
pub struct ScannerContext {
    /// Current scanner state
    pub state: ScannerState,
    /// Flow level (0 = block context, >0 = flow context)
    pub flow_level: usize,
    /// Whether we're in a key position
    pub in_key: bool,
    /// Whether we're in a value position
    pub in_value: bool,
    /// Whether simple keys are allowed
    pub allow_simple_key: bool,
    /// Stack of simple key positions
    pub simple_keys: Vec<Option<Position>>,
}

impl ScannerContext {
    /// Create a new scanner context
    pub fn new() -> Self {
        Self {
            state: ScannerState::StreamStart,
            flow_level: 0,
            in_key: false,
            in_value: false,
            allow_simple_key: true,
            simple_keys: vec![None],
        }
    }

    /// Reset the scanner context
    pub fn reset(&mut self) {
        self.state = ScannerState::StreamStart;
        self.flow_level = 0;
        self.in_key = false;
        self.in_value = false;
        self.allow_simple_key = true;
        self.simple_keys.clear();
        self.simple_keys.push(None);
    }

    /// Check if we're in flow context
    pub fn in_flow(&self) -> bool {
        self.flow_level > 0
    }

    /// Check if we're in block context
    pub fn in_block(&self) -> bool {
        self.flow_level == 0
    }

    /// Enter flow context
    pub fn enter_flow(&mut self) {
        self.flow_level += 1;
        self.simple_keys.push(None);
    }

    /// Exit flow context
    pub fn exit_flow(&mut self) {
        if self.flow_level > 0 {
            self.flow_level -= 1;
            self.simple_keys.pop();
        }
    }

    /// Save a simple key position
    pub fn save_simple_key(&mut self, position: Position) {
        if let Some(key_slot) = self.simple_keys.last_mut() {
            *key_slot = Some(position);
        }
    }

    /// Clear the current simple key
    pub fn clear_simple_key(&mut self) {
        if let Some(key_slot) = self.simple_keys.last_mut() {
            *key_slot = None;
        }
    }

    /// Check if a simple key is possible
    pub fn simple_key_allowed(&self) -> bool {
        self.allow_simple_key && self.simple_keys.last().map_or(false, |k| k.is_none())
    }
}

impl Default for ScannerContext {
    fn default() -> Self {
        Self::new()
    }
}
