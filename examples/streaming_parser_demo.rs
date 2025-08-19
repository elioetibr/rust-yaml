//! Demonstration of the streaming parser with zero-copy optimizations

#![allow(clippy::needless_raw_string_hashes)] // Test YAML strings

use rust_yaml::{EventType, StreamingConfig, StreamingParser};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example YAML with various constructs
    let yaml_input = r#"
name: streaming-demo
version: 1.0.0
features:
  - zero-copy parsing
  - memory efficient
  - batch processing
config:
  buffer_size: 64
  use_zero_copy: true
"#;

    // Configure streaming parser for zero-copy operation
    let config = StreamingConfig {
        max_buffer_size: 32,
        use_zero_copy: true,
        max_depth: 10,
        collect_stats: true,
    };

    // Create streaming parser
    let mut parser = StreamingParser::new_zero_copy(yaml_input, config);

    println!("=== Streaming YAML Parser Demo ===");
    println!("Input YAML:\n{}", yaml_input);
    println!("\n=== Generated Events ===");

    let mut event_count = 0;
    let mut batch_count = 0;

    // Process YAML in batches
    while parser.has_more_events() {
        let batch = parser.next_batch()?;
        if batch.is_empty() {
            break;
        }

        batch_count += 1;
        println!("\n--- Batch {} ({} events) ---", batch_count, batch.len());

        for event in batch {
            event_count += 1;
            match &event.event_type {
                EventType::StreamStart => {
                    println!("{:3}: StreamStart", event_count);
                }
                EventType::StreamEnd => {
                    println!("{:3}: StreamEnd", event_count);
                }
                EventType::DocumentStart { implicit, .. } => {
                    println!("{:3}: DocumentStart (implicit: {})", event_count, implicit);
                }
                EventType::DocumentEnd { implicit } => {
                    println!("{:3}: DocumentEnd (implicit: {})", event_count, implicit);
                }
                EventType::SequenceStart { flow_style, .. } => {
                    println!("{:3}: SequenceStart (flow: {})", event_count, flow_style);
                }
                EventType::SequenceEnd => {
                    println!("{:3}: SequenceEnd", event_count);
                }
                EventType::MappingStart { flow_style, .. } => {
                    println!("{:3}: MappingStart (flow: {})", event_count, flow_style);
                }
                EventType::MappingEnd => {
                    println!("{:3}: MappingEnd", event_count);
                }
                EventType::Scalar { value, style, .. } => {
                    println!("{:3}: Scalar({:?}) = \"{}\"", event_count, style, value);
                }
                EventType::Alias { anchor } => {
                    println!("{:3}: Alias({})", event_count, anchor);
                }
            }
        }
    }

    // Display statistics if available
    if let Some(stats) = parser.get_stats() {
        println!("\n=== Performance Statistics ===");
        println!("Events processed: {}", stats.events_processed);
        println!("Tokens processed: {}", stats.tokens_processed);
        println!("Max buffer size: {}", stats.max_buffer_size);
        println!("Max depth: {}", stats.max_depth);
        println!("Parse time: {}μs", stats.parse_time_ns / 1000);

        if let Some(scanner_stats) = &stats.scanner_stats {
            println!("\n=== Zero-Copy Scanner Stats ===");
            println!("Input length: {} bytes", scanner_stats.input_length);
            println!("Characters processed: {}", scanner_stats.chars_processed);
            println!("Tokens allocated: {}", scanner_stats.tokens_allocated);
            println!("Tokens used: {}", scanner_stats.tokens_used);
            println!(
                "Current position: line {}, col {}",
                scanner_stats.position.line + 1,
                scanner_stats.position.column + 1
            );
        }
    }

    println!("\n=== Summary ===");
    println!(
        "Processed {} events in {} batches",
        event_count, batch_count
    );
    println!("Streaming parser completed successfully!");

    Ok(())
}
