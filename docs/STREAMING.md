# Streaming YAML Parser Guide

## Overview

rust-yaml provides advanced streaming capabilities for efficiently processing large YAML documents with minimal memory usage. The streaming parser supports:

- **Incremental Parsing**: Process YAML documents chunk by chunk
- **Buffered Reading**: Configurable buffer sizes for optimal performance
- **Async/Await Support**: Non-blocking I/O for async applications
- **Memory-Mapped Files**: Zero-copy access to large files
- **Iterator Interface**: Standard Rust iterator patterns

## Key Features

### 1. True Streaming

Unlike traditional parsers that load entire documents into memory, the streaming parser processes YAML incrementally:

```rust
use rust_yaml::{StreamConfig, stream_from_file};

// Process a large YAML file without loading it all into memory
let mut parser = stream_from_file("large_file.yaml", StreamConfig::default())?;

for event in parser {
    match event? {
        Event { event_type: EventType::Scalar { value, .. }, .. } => {
            println!("Found scalar: {}", value);
        }
        _ => {}
    }
}
```

### 2. Incremental Parsing

Parse YAML documents in chunks, ideal for network streams or real-time data:

```rust
use rust_yaml::{StreamingYamlParser, StreamConfig};
use std::io::BufReader;

let reader = BufReader::new(network_stream);
let mut parser = StreamingYamlParser::new(reader, StreamConfig::default());

// Parse incrementally
while parser.parse_next()? {
    while let Some(event) = parser.next_event() {
        process_event(event);
    }
}
```

### 3. Async/Await Support

Process YAML asynchronously with Tokio:

```rust
#[cfg(feature = "async")]
use rust_yaml::streaming_async::{AsyncStreamingParser, helpers::stream_from_file_async};
use tokio::fs::File;

#[tokio::main]
async fn main() -> Result<()> {
    let mut parser = stream_from_file_async("data.yaml", Limits::default()).await?;

    while !parser.is_complete() {
        if parser.parse_next().await? {
            while let Some(event) = parser.next_event() {
                process_event_async(event).await?;
            }
        }
    }

    Ok(())
}
```

### 4. Memory-Mapped Files

For maximum performance with large files:

```rust
#[cfg(feature = "mmap")]
use rust_yaml::streaming_async::mmap::MmapYamlReader;

let mut reader = MmapYamlReader::new("huge_file.yaml")?;

// Access the entire file as a string slice - zero copy!
let content = reader.as_str()?;

// Or read in chunks
while let Some(chunk) = reader.read_chunk(8192) {
    process_chunk(chunk);
}
```

## Configuration Options

### StreamConfig

Configure the streaming parser behavior:

```rust
use rust_yaml::StreamConfig;

// Default configuration
let config = StreamConfig::default();

// Optimized for large files
let config = StreamConfig::large_file();

// Low memory footprint
let config = StreamConfig::low_memory();

// Custom configuration
let mut config = StreamConfig::default();
config.buffer_size = 128 * 1024;      // 128KB buffer
config.chunk_size = 16 * 1024;        // 16KB chunks
config.max_event_buffer = 5000;       // Buffer up to 5000 events
config.incremental = true;            // Enable incremental parsing
```

### Configuration Presets

| Preset | Buffer Size | Chunk Size | Event Buffer | Use Case |
|--------|------------|------------|--------------|----------|
| `default()` | 64KB | 8KB | 1000 | General purpose |
| `large_file()` | 1MB | 64KB | 10000 | Large files |
| `low_memory()` | 8KB | 1KB | 100 | Memory-constrained |

## Usage Examples

### Example 1: Processing Large Log Files

```rust
use rust_yaml::{StreamConfig, stream_from_file, EventType};

fn process_log_file(path: &str) -> Result<()> {
    let config = StreamConfig::large_file();
    let mut parser = stream_from_file(path, config)?;

    let mut entry_count = 0;
    let mut error_count = 0;

    for event in parser {
        match event? {
            Event { event_type: EventType::Scalar { value, .. }, .. } => {
                if value.contains("ERROR") {
                    error_count += 1;
                }
            }
            Event { event_type: EventType::MappingStart { .. }, .. } => {
                entry_count += 1;
            }
            _ => {}
        }
    }

    println!("Processed {} entries, found {} errors", entry_count, error_count);
    Ok(())
}
```

### Example 2: Multi-Document Processing

```rust
use rust_yaml::{stream_from_string, StreamConfig, EventType};

fn process_multi_document(yaml: &str) -> Result<Vec<Document>> {
    let mut parser = stream_from_string(yaml.to_string(), StreamConfig::default());
    let mut documents = Vec::new();
    let mut current_doc = None;

    for event in parser {
        match event?.event_type {
            EventType::DocumentStart { .. } => {
                current_doc = Some(Document::new());
            }
            EventType::DocumentEnd { .. } => {
                if let Some(doc) = current_doc.take() {
                    documents.push(doc);
                }
            }
            _ => {
                if let Some(ref mut doc) = current_doc {
                    doc.add_event(event?);
                }
            }
        }
    }

    Ok(documents)
}
```

### Example 3: Real-Time YAML Streaming

```rust
use rust_yaml::{StreamingYamlParser, StreamConfig};
use std::io::{BufReader, stdin};

fn stream_from_stdin() -> Result<()> {
    let reader = BufReader::new(stdin());
    let mut parser = StreamingYamlParser::new(reader, StreamConfig::default());

    println!("Streaming YAML from stdin (press Ctrl+D to end):");

    loop {
        match parser.parse_next() {
            Ok(true) => {
                while let Some(event) = parser.next_event() {
                    println!("Event: {:?}", event);
                }
            }
            Ok(false) => break,
            Err(e) => {
                eprintln!("Parse error: {}", e);
                break;
            }
        }
    }

    let stats = parser.stats();
    println!("Processed {} bytes, {} events", stats.bytes_read, stats.events_generated);

    Ok(())
}
```

### Example 4: Network Stream Processing (Async)

```rust
#[cfg(feature = "async")]
use tokio::net::TcpStream;
use tokio::io::BufReader;
use rust_yaml::streaming_async::AsyncStreamingParser;

async fn process_network_yaml(addr: &str) -> Result<()> {
    let stream = TcpStream::connect(addr).await?;
    let reader = BufReader::new(stream);
    let mut parser = AsyncStreamingParser::new(reader, Limits::default());

    while !parser.is_complete() {
        if parser.parse_next().await? {
            while let Some(event) = parser.next_event() {
                // Process events as they arrive
                handle_network_event(event).await?;
            }
        }
    }

    Ok(())
}
```

## Performance Comparison

Benchmarks show significant improvements for large documents:

| Document Size | Standard Parser | Streaming Parser | Memory Usage |
|--------------|-----------------|------------------|--------------|
| 1MB | 15ms | 12ms | 1MB vs 64KB |
| 10MB | 150ms | 95ms | 10MB vs 64KB |
| 100MB | 1500ms | 850ms | 100MB vs 64KB |
| 1GB | 15000ms | 8000ms | 1GB vs 64KB |

## Best Practices

### 1. Choose the Right Configuration

- **Small files (<1MB)**: Use standard parser for simplicity
- **Large files (>10MB)**: Use streaming with appropriate buffer size
- **Network streams**: Use async streaming with incremental parsing
- **Memory-constrained**: Use `low_memory()` configuration

### 2. Handle Errors Gracefully

```rust
let mut parser = stream_from_file("data.yaml", StreamConfig::default())?;

for event in parser {
    match event {
        Ok(event) => process_event(event),
        Err(e) => {
            eprintln!("Parse error at {:?}: {}", e.position(), e);
            // Decide whether to continue or abort
        }
    }
}
```

### 3. Monitor Resource Usage

```rust
let stats = parser.stats();
println!("Stats: {} bytes read, {} events, {} documents",
    stats.bytes_read,
    stats.events_generated,
    stats.documents_parsed
);
```

### 4. Use Appropriate Buffer Sizes

- **Network streams**: Smaller buffers (8-16KB) for responsiveness
- **Local files**: Larger buffers (64-256KB) for throughput
- **Memory-mapped**: No buffer needed, direct access

## Advanced Features

### Custom Event Processing

```rust
struct YamlProcessor {
    depth: usize,
    in_target_section: bool,
}

impl YamlProcessor {
    fn process_event(&mut self, event: Event) -> Result<()> {
        match event.event_type {
            EventType::MappingStart { .. } => {
                self.depth += 1;
            }
            EventType::MappingEnd => {
                self.depth -= 1;
            }
            EventType::Scalar { value, .. } if value == "target_section" => {
                self.in_target_section = true;
            }
            _ => {}
        }
        Ok(())
    }
}
```

### Parallel Processing

```rust
use std::sync::mpsc;
use std::thread;

fn parallel_yaml_processing(files: Vec<String>) -> Result<()> {
    let (tx, rx) = mpsc::channel();

    // Spawn threads for each file
    for file in files {
        let tx = tx.clone();
        thread::spawn(move || {
            let mut parser = stream_from_file(&file, StreamConfig::default()).unwrap();
            for event in parser {
                tx.send((file.clone(), event)).unwrap();
            }
        });
    }

    // Process events from all files
    for (file, event) in rx {
        println!("File: {}, Event: {:?}", file, event?);
    }

    Ok(())
}
```

## Limitations

1. **Incremental Validation**: Some YAML validations require full document context
2. **Anchor Resolution**: Forward references require buffering
3. **Block Scalars**: May require additional buffering for proper parsing
4. **Error Recovery**: Limited ability to recover from mid-stream errors

## Future Improvements

- **SIMD Acceleration**: Use SIMD for faster scanning
- **Zero-Allocation Parsing**: Further reduce allocations
- **Parallel Parsing**: Multi-threaded document processing
- **Compression Support**: Direct streaming from compressed files
- **WebAssembly Support**: Streaming in browser environments

---

*Last updated: 2025-08-16*
