//! Async streaming YAML parser for non-blocking I/O
//!
//! This module provides async/await support for streaming YAML parsing,
//! enabling efficient processing of YAML from async sources like network
//! streams, async file I/O, and more.

#[cfg(feature = "async")]
use futures::stream::Stream;
#[cfg(feature = "async")]
use std::pin::Pin;
#[cfg(feature = "async")]
use std::task::{Context, Poll};
#[cfg(feature = "async")]
use tokio::io::{AsyncBufRead, AsyncBufReadExt, BufReader};

use crate::{
    parser::{Event, EventType},
    Limits, Position, Result,
};
use std::collections::VecDeque;

/// Async streaming YAML parser
#[cfg(feature = "async")]
pub struct AsyncStreamingParser<R: AsyncBufRead + Unpin> {
    /// Async reader
    reader: R,
    /// Buffer for incomplete data
    buffer: String,
    /// Event queue
    events: VecDeque<Event>,
    /// Current position
    position: Position,
    /// Parse state
    state: AsyncParseState,
    /// Resource limits
    limits: Limits,
    /// Statistics
    stats: AsyncStreamStats,
}

#[cfg(feature = "async")]
#[derive(Debug, Clone, PartialEq)]
enum AsyncParseState {
    Initial,
    InDocument,
    BetweenDocuments,
    Complete,
}

#[cfg(feature = "async")]
#[derive(Debug, Clone, Default)]
/// Statistics for async streaming parser
#[allow(missing_docs)]
pub struct AsyncStreamStats {
    pub bytes_read: usize,
    pub events_generated: usize,
    pub documents_parsed: usize,
}

#[cfg(feature = "async")]
impl<R: AsyncBufRead + Unpin> AsyncStreamingParser<R> {
    /// Create a new async streaming parser
    pub fn new(reader: R, limits: Limits) -> Self {
        Self {
            reader,
            buffer: String::with_capacity(4096),
            events: VecDeque::with_capacity(100),
            position: Position::new(),
            state: AsyncParseState::Initial,
            limits,
            stats: AsyncStreamStats::default(),
        }
    }

    /// Parse the next chunk asynchronously
    pub async fn parse_next(&mut self) -> Result<bool> {
        // Read next line or chunk
        let mut line = String::new();
        let bytes_read = self.reader.read_line(&mut line).await?;

        if bytes_read == 0 && self.buffer.is_empty() {
            self.state = AsyncParseState::Complete;
            return Ok(false);
        }

        self.buffer.push_str(&line);
        self.stats.bytes_read += bytes_read;

        // Parse the buffer
        self.parse_buffer()?;

        Ok(!self.events.is_empty())
    }

    /// Parse current buffer content
    fn parse_buffer(&mut self) -> Result<()> {
        match self.state {
            AsyncParseState::Initial => {
                self.emit_event(EventType::StreamStart)?;
                self.state = AsyncParseState::BetweenDocuments;
            }
            AsyncParseState::BetweenDocuments => {
                if self.buffer.contains("---") {
                    self.emit_event(EventType::DocumentStart {
                        version: None,
                        tags: Vec::new(),
                        implicit: true,
                    })?;
                    self.state = AsyncParseState::InDocument;
                    self.stats.documents_parsed += 1;
                }
            }
            AsyncParseState::InDocument => {
                self.parse_document_content()?;
            }
            AsyncParseState::Complete => {}
        }
        Ok(())
    }

    /// Parse document content
    fn parse_document_content(&mut self) -> Result<()> {
        // Simplified parsing logic
        while !self.buffer.is_empty() {
            if self.buffer.starts_with("...") {
                self.emit_event(EventType::DocumentEnd { implicit: false })?;
                self.state = AsyncParseState::BetweenDocuments;
                self.buffer.drain(..3);
                break;
            }

            // Parse line by line (simplified)
            if let Some(newline_pos) = self.buffer.find('\n') {
                let line = self.buffer.drain(..=newline_pos).collect::<String>();
                self.parse_line(line)?;
            } else {
                break; // Need more data
            }
        }
        Ok(())
    }

    /// Parse a single line
    fn parse_line(&mut self, line: String) -> Result<()> {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            return Ok(());
        }

        // Simple key-value parsing
        if let Some(colon_pos) = trimmed.find(':') {
            let key = &trimmed[..colon_pos];
            let value = &trimmed[colon_pos + 1..];

            // Emit scalar events for key and value
            self.emit_event(EventType::Scalar {
                value: key.trim().to_string(),
                anchor: None,
                tag: None,
                style: crate::parser::ScalarStyle::Plain,
                plain_implicit: true,
                quoted_implicit: true,
            })?;

            self.emit_event(EventType::Scalar {
                value: value.trim().to_string(),
                anchor: None,
                tag: None,
                style: crate::parser::ScalarStyle::Plain,
                plain_implicit: true,
                quoted_implicit: true,
            })?;
        }

        Ok(())
    }

    /// Emit an event
    fn emit_event(&mut self, event_type: EventType) -> Result<()> {
        self.events.push_back(Event {
            event_type,
            position: self.position,
        });
        self.stats.events_generated += 1;
        Ok(())
    }

    /// Get the next event
    pub fn next_event(&mut self) -> Option<Event> {
        self.events.pop_front()
    }

    /// Check if parsing is complete
    pub fn is_complete(&self) -> bool {
        self.state == AsyncParseState::Complete && self.events.is_empty()
    }

    /// Get statistics
    pub fn stats(&self) -> &AsyncStreamStats {
        &self.stats
    }
}

#[cfg(feature = "async")]
impl<R: AsyncBufRead + Unpin> Stream for AsyncStreamingParser<R> {
    type Item = Result<Event>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Check if we have buffered events
        if let Some(event) = self.next_event() {
            return Poll::Ready(Some(Ok(event)));
        }

        // If parsing is complete, return None
        if self.is_complete() {
            return Poll::Ready(None);
        }

        // Try to parse more data
        let waker = cx.waker().clone();

        // This is simplified - in production would use proper async runtime integration
        match futures::executor::block_on(self.parse_next()) {
            Ok(true) => {
                if let Some(event) = self.next_event() {
                    Poll::Ready(Some(Ok(event)))
                } else {
                    waker.wake();
                    Poll::Pending
                }
            }
            Ok(false) => Poll::Ready(None),
            Err(e) => Poll::Ready(Some(Err(e))),
        }
    }
}

/// Async helper functions
#[cfg(feature = "async")]
pub mod helpers {
    use super::*;
    use std::path::Path;
    use tokio::fs::File;

    /// Stream YAML from an async file
    pub async fn stream_from_file_async<P: AsRef<Path>>(
        path: P,
        limits: Limits,
    ) -> Result<AsyncStreamingParser<BufReader<File>>> {
        let file = File::open(path).await?;
        let reader = BufReader::new(file);
        Ok(AsyncStreamingParser::new(reader, limits))
    }

    /// Stream YAML from async reader
    pub fn stream_from_async_reader<R: AsyncBufRead + Unpin>(
        reader: R,
        limits: Limits,
    ) -> AsyncStreamingParser<R> {
        AsyncStreamingParser::new(reader, limits)
    }

    /// Process YAML stream with a callback
    pub async fn process_yaml_stream<R, F, Fut>(
        mut parser: AsyncStreamingParser<R>,
        mut callback: F,
    ) -> Result<()>
    where
        R: AsyncBufRead + Unpin,
        F: FnMut(Event) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        while !parser.is_complete() {
            if parser.parse_next().await? {
                while let Some(event) = parser.next_event() {
                    callback(event).await?;
                }
            }
        }
        Ok(())
    }
}

/// Memory-mapped file support for efficient large file processing
#[cfg(not(target_arch = "wasm32"))]
pub mod mmap {
    use crate::Result;
    use memmap2::{Mmap, MmapOptions};
    use std::fs::File;
    use std::path::Path;

    /// Memory-mapped YAML file reader
    pub struct MmapYamlReader {
        mmap: Mmap,
        position: usize,
    }

    impl MmapYamlReader {
        /// Create a new memory-mapped reader
        pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
            let file = File::open(path)?;
            // Note: Using memory mapping which is inherently unsafe but contained
            // This is acceptable for file I/O in controlled environments
            #[allow(unsafe_code)]
            let mmap = unsafe { MmapOptions::new().map(&file)? };

            Ok(Self { mmap, position: 0 })
        }

        /// Get the entire content as a string slice
        pub fn as_str(&self) -> Result<&str> {
            std::str::from_utf8(&self.mmap).map_err(|e| {
                crate::Error::construction(
                    crate::Position::new(),
                    format!("UTF-8 conversion failed: {}", e),
                )
            })
        }

        /// Read a chunk from current position
        pub fn read_chunk(&mut self, size: usize) -> Option<&str> {
            if self.position >= self.mmap.len() {
                return None;
            }

            let end = (self.position + size).min(self.mmap.len());
            let chunk = &self.mmap[self.position..end];
            self.position = end;

            std::str::from_utf8(chunk).ok()
        }

        /// Reset position to beginning
        pub fn reset(&mut self) {
            self.position = 0;
        }

        /// Get remaining bytes
        pub fn remaining(&self) -> usize {
            self.mmap.len().saturating_sub(self.position)
        }
    }
}

#[cfg(all(test, feature = "async"))]
mod async_tests {
    use super::*;
    use futures::StreamExt;
    use std::io::Cursor;

    #[tokio::test]
    async fn test_async_streaming() {
        const MAX_ITERATIONS: usize = 100;

        let yaml = "---\nkey: value\n...\n";
        let cursor = Cursor::new(yaml.as_bytes().to_vec());
        let reader = BufReader::new(cursor);
        let mut parser = AsyncStreamingParser::new(reader, Limits::default());

        let mut events = Vec::new();
        let mut iterations = 0;

        while !parser.is_complete() && iterations < MAX_ITERATIONS {
            iterations += 1;
            match parser.parse_next().await {
                Ok(has_events) => {
                    if has_events {
                        while let Some(event) = parser.next_event() {
                            events.push(event);
                        }
                    } else if parser.state == AsyncParseState::Complete {
                        // Ensure we exit when parsing is done
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        assert!(!events.is_empty());
        assert!(matches!(events[0].event_type, EventType::StreamStart));
    }

    #[tokio::test]
    async fn test_stream_trait() {
        use tokio::time::{timeout, Duration};

        let yaml = "key: value\n";
        let cursor = Cursor::new(yaml.as_bytes().to_vec());
        let reader = BufReader::new(cursor);
        let mut parser = AsyncStreamingParser::new(reader, Limits::default());

        let result = timeout(Duration::from_secs(5), parser.take(5).collect::<Vec<_>>()).await;

        let events = result.expect("Test timed out after 5 seconds");
        assert!(!events.is_empty());
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod mmap_tests {
    use super::mmap::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_mmap_reader() {
        // Create a temporary file
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "key: value").unwrap();
        writeln!(file, "list:").unwrap();
        writeln!(file, "  - item1").unwrap();
        writeln!(file, "  - item2").unwrap();
        file.flush().unwrap();

        // Test memory-mapped reading
        let mut reader = MmapYamlReader::new(file.path()).unwrap();
        let content = reader.as_str().unwrap();
        assert!(content.contains("key: value"));

        // Test chunk reading
        reader.reset();
        let chunk = reader.read_chunk(10).unwrap();
        assert_eq!(chunk, "key: value");
    }
}
