//! Resource limits for secure YAML processing

use crate::{Error, Result};
use std::time::Duration;

/// Resource limits configuration for YAML processing
#[derive(Debug, Clone)]
pub struct Limits {
    /// Maximum nesting depth for collections
    pub max_depth: usize,
    /// Maximum number of anchors in a document
    pub max_anchors: usize,
    /// Maximum document size in bytes
    pub max_document_size: usize,
    /// Maximum string length in characters
    pub max_string_length: usize,
    /// Maximum alias expansion depth
    pub max_alias_depth: usize,
    /// Maximum number of items in a collection
    pub max_collection_size: usize,
    /// Maximum complexity score (calculated based on structure)
    pub max_complexity_score: usize,
    /// Timeout for parsing operations
    pub timeout: Option<Duration>,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_depth: 1000,
            max_anchors: 10_000,
            max_document_size: 100 * 1024 * 1024, // 100MB
            max_string_length: 10 * 1024 * 1024,  // 10MB
            max_alias_depth: 100,
            max_collection_size: 1_000_000,
            max_complexity_score: 1_000_000,
            timeout: None,
        }
    }
}

impl Limits {
    /// Creates strict limits for untrusted input
    pub fn strict() -> Self {
        Self {
            max_depth: 50,
            max_anchors: 100,
            max_document_size: 1024 * 1024, // 1MB
            max_string_length: 64 * 1024,   // 64KB
            max_alias_depth: 5,
            max_collection_size: 10_000,
            max_complexity_score: 10_000,
            timeout: Some(Duration::from_secs(5)),
        }
    }

    /// Creates permissive limits for trusted input
    pub fn permissive() -> Self {
        Self {
            max_depth: 10_000,
            max_anchors: 100_000,
            max_document_size: 1024 * 1024 * 1024, // 1GB
            max_string_length: 100 * 1024 * 1024,  // 100MB
            max_alias_depth: 1000,
            max_collection_size: 10_000_000,
            max_complexity_score: 100_000_000,
            timeout: None,
        }
    }

    /// Creates unlimited configuration (use with caution)
    pub fn unlimited() -> Self {
        Self {
            max_depth: usize::MAX,
            max_anchors: usize::MAX,
            max_document_size: usize::MAX,
            max_string_length: usize::MAX,
            max_alias_depth: usize::MAX,
            max_collection_size: usize::MAX,
            max_complexity_score: usize::MAX,
            timeout: None,
        }
    }
}

/// Tracks resource usage during parsing
#[derive(Debug, Clone, Default)]
pub struct ResourceTracker {
    current_depth: usize,
    max_depth_seen: usize,
    anchor_count: usize,
    bytes_processed: usize,
    alias_depth: usize,
    complexity_score: usize,
    collection_items: usize,
}

impl ResourceTracker {
    /// Creates a new resource tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks if depth limit is exceeded
    pub fn check_depth(&mut self, limits: &Limits, depth: usize) -> Result<()> {
        self.current_depth = depth;
        self.max_depth_seen = self.max_depth_seen.max(depth);

        if depth > limits.max_depth {
            return Err(Error::limit_exceeded(format!(
                "Maximum depth {} exceeded",
                limits.max_depth
            )));
        }
        Ok(())
    }

    /// Increments and checks anchor count
    pub fn add_anchor(&mut self, limits: &Limits) -> Result<()> {
        self.anchor_count += 1;
        if self.anchor_count > limits.max_anchors {
            return Err(Error::limit_exceeded(format!(
                "Maximum anchor count {} exceeded",
                limits.max_anchors
            )));
        }
        Ok(())
    }

    /// Tracks bytes processed
    pub fn add_bytes(&mut self, limits: &Limits, bytes: usize) -> Result<()> {
        self.bytes_processed += bytes;
        if self.bytes_processed > limits.max_document_size {
            return Err(Error::limit_exceeded(format!(
                "Maximum document size {} exceeded",
                limits.max_document_size
            )));
        }
        Ok(())
    }

    /// Checks string length
    pub fn check_string_length(&self, limits: &Limits, length: usize) -> Result<()> {
        if length > limits.max_string_length {
            return Err(Error::limit_exceeded(format!(
                "Maximum string length {} exceeded",
                limits.max_string_length
            )));
        }
        Ok(())
    }

    /// Tracks alias expansion depth
    pub fn enter_alias(&mut self, limits: &Limits) -> Result<()> {
        if self.alias_depth + 1 > limits.max_alias_depth {
            return Err(Error::limit_exceeded(format!(
                "Maximum alias depth {} exceeded",
                limits.max_alias_depth
            )));
        }
        self.alias_depth += 1;
        Ok(())
    }

    /// Exits alias expansion
    pub fn exit_alias(&mut self) {
        if self.alias_depth > 0 {
            self.alias_depth -= 1;
        }
    }

    /// Tracks collection items
    pub fn add_collection_item(&mut self, limits: &Limits) -> Result<()> {
        self.collection_items += 1;
        if self.collection_items > limits.max_collection_size {
            return Err(Error::limit_exceeded(format!(
                "Maximum collection size {} exceeded",
                limits.max_collection_size
            )));
        }
        Ok(())
    }

    /// Adds to complexity score
    pub fn add_complexity(&mut self, limits: &Limits, score: usize) -> Result<()> {
        self.complexity_score += score;
        if self.complexity_score > limits.max_complexity_score {
            return Err(Error::limit_exceeded(format!(
                "Maximum complexity score {} exceeded",
                limits.max_complexity_score
            )));
        }
        Ok(())
    }

    /// Resets the tracker for a new document
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Gets current statistics
    pub fn stats(&self) -> ResourceStats {
        ResourceStats {
            max_depth: self.max_depth_seen,
            anchor_count: self.anchor_count,
            bytes_processed: self.bytes_processed,
            complexity_score: self.complexity_score,
            collection_items: self.collection_items,
        }
    }
}

/// Resource usage statistics
#[derive(Debug, Clone)]
pub struct ResourceStats {
    /// Maximum depth reached during processing
    pub max_depth: usize,
    /// Total number of anchors encountered
    pub anchor_count: usize,
    /// Total bytes processed
    pub bytes_processed: usize,
    /// Total complexity score
    pub complexity_score: usize,
    /// Total collection items processed
    pub collection_items: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limits() {
        let limits = Limits::default();
        assert_eq!(limits.max_depth, 1000);
        assert_eq!(limits.max_anchors, 10_000);
    }

    #[test]
    fn test_strict_limits() {
        let limits = Limits::strict();
        assert_eq!(limits.max_depth, 50);
        assert_eq!(limits.max_anchors, 100);
        assert!(limits.timeout.is_some());
    }

    #[test]
    fn test_resource_tracker() {
        let limits = Limits::strict();
        let mut tracker = ResourceTracker::new();

        // Test depth checking
        assert!(tracker.check_depth(&limits, 10).is_ok());
        assert!(tracker.check_depth(&limits, 51).is_err());

        // Test anchor counting
        for _ in 0..100 {
            assert!(tracker.add_anchor(&limits).is_ok());
        }
        assert!(tracker.add_anchor(&limits).is_err());
    }

    #[test]
    fn test_alias_depth_tracking() {
        let limits = Limits::strict();
        let mut tracker = ResourceTracker::new();

        // Test entering aliases
        for _ in 0..5 {
            assert!(tracker.enter_alias(&limits).is_ok());
        }
        assert!(tracker.enter_alias(&limits).is_err());

        // Test exiting aliases
        tracker.exit_alias();
        assert!(tracker.enter_alias(&limits).is_ok());
    }
}
