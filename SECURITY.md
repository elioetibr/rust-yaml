# Security Guide for rust-yaml

## Overview

rust-yaml implements comprehensive security measures to protect against common YAML parsing vulnerabilities, including the billion laughs attack, resource exhaustion, and malformed document attacks.

## Built-in Protection

### 1. Resource Limits

rust-yaml enforces configurable resource limits to prevent resource exhaustion attacks:

- **Max Depth**: Limits nesting depth of collections (default: 1000)
- **Max Anchors**: Limits number of anchors per document (default: 10,000)
- **Max Document Size**: Limits total document size in bytes (default: 100MB)
- **Max String Length**: Limits individual string length (default: 10MB)
- **Max Alias Depth**: Limits alias expansion depth (default: 100)
- **Max Collection Size**: Limits items in a single collection (default: 1,000,000)
- **Max Complexity Score**: Limits overall document complexity (default: 1,000,000)

### 2. Protection Against Billion Laughs Attack

The billion laughs attack (exponential entity expansion) is prevented through:

- Alias expansion depth tracking
- Cyclic reference detection
- Complexity scoring for nested structures
- Collection size limits

Example of prevented attack:

```yaml

# This would expand exponentially without protection
a: &a ["lol", "lol", "lol", "lol", "lol", "lol", "lol", "lol", "lol"]
b: &b [*a, *a, *a, *a, *a, *a, *a, *a, *a]
c: &c [*b, *b, *b, *b, *b, *b, *b, *b, *b]

# ... would expand to 9^n items
```

### 3. Cyclic Reference Detection

rust-yaml detects and prevents cyclic alias references:

```yaml

# This cyclic reference is detected and rejected
a: &a
  b: *b
b: &b
  a: *a
```

## Configuration Presets

### Strict Mode (Untrusted Input)

```rust
use rust_yaml::{Yaml, YamlConfig, Limits};

let mut config = YamlConfig::default();
config.limits = Limits::strict();
let yaml = Yaml::with_config(config);
```

Strict limits:

- Max depth: 50
- Max anchors: 100
- Max document size: 1MB
- Max string length: 64KB
- Max alias depth: 5
- Max collection size: 10,000
- Timeout: 5 seconds

### Secure Mode

```rust
let config = YamlConfig::secure();
let yaml = Yaml::with_config(config);
```

Balanced security for production use with reasonable limits.

### Permissive Mode (Trusted Input)

```rust
let mut config = YamlConfig::default();
config.limits = Limits::permissive();
```

Higher limits for trusted sources:

- Max depth: 10,000
- Max anchors: 100,000
- Max document size: 1GB
- Max string length: 100MB

### Custom Limits

```rust
use rust_yaml::{Limits, YamlConfig};
use std::time::Duration;

let mut limits = Limits::default();
limits.max_depth = 100;
limits.max_anchors = 500;
limits.max_document_size = 5 * 1024 * 1024; // 5MB
limits.timeout = Some(Duration::from_secs(10));

let mut config = YamlConfig::default();
config.limits = limits;
```

## Best Practices

### 1. Always Use Limits for Untrusted Input

Never parse untrusted YAML without resource limits:

```rust
// BAD - No protection
let yaml = Yaml::new();
let result = yaml.load_str(untrusted_input); // Vulnerable!

// GOOD - Protected parsing
let config = YamlConfig::secure();
let yaml = Yaml::with_config(config);
let result = yaml.load_str(untrusted_input); // Protected
```

### 2. Choose Appropriate Limits

Select limits based on your use case:

- **Configuration files**: Use `strict()` limits
- **User-generated content**: Use `strict()` with custom timeout
- **Internal data**: Use `default()` or `permissive()`
- **Large datasets**: Use custom limits with appropriate sizes

### 3. Handle Errors Gracefully

Resource limit errors should be handled appropriately:

```rust
match yaml.load_str(input) {
    Ok(value) => process_value(value),
    Err(e) if e.to_string().contains("limit") => {
        // Log security event
        log::warn!("Resource limit exceeded: {}", e);
        // Return safe error to user
        return Err("Document too complex");
    }
    Err(e) => {
        // Handle other parsing errors
        return Err(format!("Invalid YAML: {}", e));
    }
}
```

### 4. Monitor Resource Usage

Use the ResourceTracker to monitor actual usage:

```rust
let tracker = ResourceTracker::new();
// After parsing, check statistics
let stats = tracker.stats();
log::debug!("Max depth: {}, Anchors: {}, Complexity: {}",
    stats.max_depth, stats.anchor_count, stats.complexity_score);
```

### 5. Validate Content After Parsing

Even with security limits, validate parsed content:

```rust
let value = yaml.load_str(input)?;

// Validate expected structure
if !validate_schema(&value) {
    return Err("Invalid document structure");
}

// Sanitize values if needed
let sanitized = sanitize_values(value);
```

## Common Attack Vectors

### 1. Exponential Expansion

- **Attack**: Nested aliases causing exponential growth
- **Protection**: Alias depth limits, complexity scoring

### 2. Deep Nesting

- **Attack**: Deeply nested structures causing stack overflow
- **Protection**: Max depth limits

### 3. Large Collections

- **Attack**: Huge arrays/maps consuming memory
- **Protection**: Collection size limits

### 4. Long Strings

- **Attack**: Multi-gigabyte strings
- **Protection**: String length limits

### 5. Anchor Bombs

- **Attack**: Thousands of anchors slowing parsing
- **Protection**: Anchor count limits

## Testing Security

Run security tests to verify protection:

```bash
cargo test --test security_limits
```

Tests include:

- Billion laughs attack prevention
- Cyclic reference detection
- Resource limit enforcement
- Timeout handling

## Reporting Security Issues

If you discover a security vulnerability:

1. **Do not** create a public issue
2. Email security concerns to the maintainers
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

## Updates and Patches

- Security updates are released as patch versions
- Critical vulnerabilities trigger immediate releases
- Subscribe to security advisories for notifications

## Additional Resources

- [YAML Security Cheatsheet](https://cheatsheetseries.owasp.org/cheatsheets/YAML_Security_Cheat_Sheet.html)
- [CVE Database for YAML](https://cve.mitre.org/cgi-bin/cvekey.cgi?keyword=yaml)
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)

---

*Last updated: 2025-08-16*
