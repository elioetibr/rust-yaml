# GitHub Actions Caching Strategy

This document outlines the caching strategy implemented for optimal CI/CD performance.

## Overview

Our caching strategy focuses on three main areas:

1. **Rust toolchain caching** - Cache rustup installations
2. **Cargo dependency caching** - Cache downloaded crates and git repositories  
3. **Build artifact caching** - Cache compiled target directories with smart fallbacks
4. **Tool caching** - Cache installed cargo tools like cargo-audit, cargo-llvm-cov

## Cache Keys

### Registry and Git Cache

- **Key**: `{OS}-cargo-registry-{Cargo.lock hash}`
- **Paths**: `~/.cargo/registry/index`, `~/.cargo/registry/cache`, `~/.cargo/git/db`
- **Fallback**: `{OS}-cargo-registry-`

### Build Cache  

- **Key**: `{OS}-cargo-{suffix}-{toolchain}-{Cargo.lock hash}-{source hash}`
- **Paths**: `target/`
- **Fallbacks**: Multiple levels of specificity for maximum cache hits

### Toolchain Cache

- **Key**: `{OS}-rustup-{toolchain}-{toolchain files hash}`
- **Paths**: `~/.rustup/toolchains`, `~/.rustup/update-hashes`, `~/.rustup/settings.toml`
- **Fallback**: `{OS}-rustup-{toolchain}-`, `{OS}-rustup-`

### Tools Cache

- **Key**: `{OS}-cargo-tools-{tools list}-{binary hash}`
- **Paths**: `~/.cargo/bin`
- **Fallback**: `{OS}-cargo-tools-{tools list}-`, `{OS}-cargo-tools-`

## Composite Actions

### setup-rust-cache

Reusable action for Rust project caching with inputs:

- `cache-key-suffix`: Additional cache specificity
- `toolchain`: Rust version (default: stable)
- `cache-target`: Whether to cache target directory (default: true)

### setup-cargo-tools  

Reusable action for cargo tools with inputs:

- `tools`: Comma-separated list of tools to install/cache

## Performance Optimizations

1. **Incremental Compilation**: `CARGO_INCREMENTAL=1` for faster builds
2. **Concurrency Control**: Prevent redundant workflow runs
3. **Restore Keys**: Multi-level fallback for partial cache hits
4. **Separate Caches**: Different caches for different build types (test, clippy, bench, etc.)
5. **Source Hash Inclusion**: Invalidate cache when source code changes

## Cache Strategy by Job

### Test Suite

- Toolchain cache (for non-stable versions)
- Registry cache (shared across all jobs)
- Build cache with toolchain-specific keys

### Clippy/Format

- Registry cache (shared)
- Build cache with specific keys, falls back to test cache

### Coverage

- Registry cache (shared)
- Build cache with coverage-specific keys
- Tool cache for cargo-llvm-cov

### Benchmarks

- Registry cache (shared)  
- Build cache including benchmark source files
- Separate from other caches due to different optimization profile

### Security Audit

- Tool cache for cargo-audit
- No build cache needed (just runs audit)

### Documentation

- Registry cache (shared)
- Build cache for docs generation

### Cross-platform

- Platform-specific registry and build caches
- Falls back to stable toolchain caches

## Cache Efficiency

- **Registry cache**: Shared across all jobs for maximum efficiency
- **Build cache**: Job-specific with intelligent fallbacks
- **Tool cache**: Persists tools between runs, major time saver
- **Restore keys**: Ensure partial cache hits when exact match fails

## Monitoring

Monitor cache hit rates in Actions tab. Expected hit rates:

- Registry cache: >90% after first run
- Build cache: 70-85% depending on source changes
- Tool cache: >95% (tools rarely change)

## Troubleshooting

If builds are slower than expected:

1. Check cache hit rates in workflow logs
2. Verify cache keys are appropriate for your changes
3. Consider if source file hashing is too broad/narrow
4. Check if restore-keys are providing useful fallbacks
