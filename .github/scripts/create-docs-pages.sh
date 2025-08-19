#!/bin/bash
set -e

# Build docs with all features
cargo doc --all-features --no-deps

# Create index redirect
cat > target/doc/index.html << 'EOF'
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>rust-yaml Documentation</title>
  <meta http-equiv="refresh" content="0; url=rust_yaml/index.html">
  <link rel="canonical" href="rust_yaml/index.html">
</head>
<body>
  <p>Redirecting to <a href="rust_yaml/index.html">documentation</a>...</p>
</body>
</html>
EOF

# Add .nojekyll to prevent GitHub Pages from ignoring files starting with underscore
touch target/doc/.nojekyll

# Create a simple landing page
cat > target/doc/landing.html << 'EOF'
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>rust-yaml - Fast, Safe YAML for Rust</title>
  <style>
    body { font-family: system-ui, -apple-system, sans-serif; max-width: 800px; margin: 0 auto; padding: 2rem; }
    h1 { color: #333; }
    .links { margin: 2rem 0; }
    .links a { display: inline-block; margin: 0.5rem 1rem 0.5rem 0; color: #0969da; text-decoration: none; }
    .links a:hover { text-decoration: underline; }
    .badges { margin: 1rem 0; }
    .badges img { margin-right: 0.5rem; }
  </style>
</head>
<body>
  <h1>rust-yaml</h1>
  <p>A fast, safe YAML 1.2 library for Rust with full specification support.</p>
  
  <div class="badges">
    <img src="https://img.shields.io/crates/v/rust-yaml.svg" alt="Crates.io">
    <img src="https://img.shields.io/docsrs/rust-yaml" alt="docs.rs">
    <img src="https://img.shields.io/crates/l/rust-yaml.svg" alt="License">
  </div>
  
  <div class="links">
    <a href="rust_yaml/index.html">📚 API Documentation</a>
    <a href="https://github.com/elioetibr/rust-yaml">📦 GitHub Repository</a>
    <a href="https://crates.io/crates/rust-yaml">🦀 Crates.io</a>
    <a href="https://docs.rs/rust-yaml">📖 docs.rs</a>
  </div>
  
  <h2>Features</h2>
  <ul>
    <li>Full YAML 1.2 specification support</li>
    <li>Safe by default with configurable limits</li>
    <li>Zero-copy parsing where possible</li>
    <li>Streaming parser for large documents</li>
    <li>Preserve formatting for round-trip operations</li>
    <li>Comprehensive error reporting with positions</li>
  </ul>
  
  <h2>Quick Start</h2>
  <pre><code>use rust_yaml::{Yaml, Value};

let yaml = Yaml::new();
let value = yaml.load_str("key: value").unwrap();
println!("{:?}", value);</code></pre>
</body>
</html>
EOF

echo "Documentation pages created successfully"