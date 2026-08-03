<!--
SPDX-FileCopyrightText: Rust Yaml contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Contributing to rust-yaml

Thank you for your interest in contributing to rust-yaml! We welcome contributions from the community and are pleased to have you aboard.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Code Style](#code-style)
- [Submitting Changes](#submitting-changes)
- [Security](#security)
- [Community](#community)

## Code of Conduct

This project and everyone participating in it is governed by our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers.

## Getting Started

### Prerequisites

- **Rust**: Latest stable version (1.74.0 or higher)
- **Git**: For version control
- **Cargo**: Comes with Rust installation

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork locally:

   ```bash
   git clone https://github.com/elioetibr/rust-yaml.git
   cd rust-yaml
   ```

3. Add the upstream repository:

   ```bash
   git remote add upstream https://github.com/elioetibr/rust-yaml.git
   ```

## Development Setup

### Initial Setup

```bash

# Install dependencies and run initial tests
cargo check
cargo test
cargo fmt -- --check
cargo clippy -- -D warnings
```

### Development Environment Setup

Run the setup script to configure git hooks and development tools:

```bash
./scripts/setup-hooks.sh
```

This will:

- Configure git hooks for commit message validation
- Set up conventional commit template
- Install `committed` (Rust-native Conventional Commits linter) if missing
- Verify Rust toolchain and components
- Run initial project checks
- Configure helpful git aliases

### Commit Message Format

We use [conventional commits](https://www.conventionalcommits.org/) for consistent commit messages:

```text
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:**

- `feat`: New features
- `fix`: Bug fixes
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Build tasks, dependency updates, etc.
- `perf`: Performance improvements
- `ci`: CI/CD changes
- `build`: Build system changes
- `revert`: Reverting previous commits

**Scopes (optional):**
`parser`, `scanner`, `emitter`, `composer`, `constructor`, `lib`, `cli`, `docs`, `tests`, `benches`, `examples`, `ci`, `deps`, `release`

**Examples:**

```bash
feat(parser): add support for complex mapping keys
fix(emitter): resolve string quoting for version numbers
docs: update installation instructions
test(scanner): add edge case tests for deeply nested structures
chore: bump version to 1.1.0 [skip ci]
```

**Special markers:**

- `+semver: major|minor|patch|none` - Control version bumping
- `[skip ci]` or `[ci skip]` - Skip CI builds (for version bumps, docs-only changes)
- `BREAKING CHANGE: description` - Indicate breaking changes

### Development Tools

We recommend installing these additional tools:

```bash

# For code coverage
cargo install cargo-tarpaulin

# For benchmarking
cargo install cargo-criterion

# For security audits
cargo install cargo-audit
```

### Project Structure

```text
rust-yaml/
├── src/                    # Source code
│   ├── scanner/           # Token scanning
│   ├── parser/            # Event parsing
│   ├── composer.rs        # Node composition
│   ├── emitter.rs         # YAML output
│   └── lib.rs             # Main library
├── tests/                 # Integration tests
├── benches/              # Performance benchmarks
├── examples/             # Usage examples
└── docs/                 # Documentation
```

## Making Changes

### Branch Naming

Use descriptive branch names:

- `feature/add-streaming-support`
- `fix/merge-keys-bug`
- `docs/update-readme`
- `perf/optimize-scanner`

### Commit Messages

Follow conventional commits format:

```text
type(scope): description

[optional body]

[optional footer]
```

Examples:

- `feat(parser): add support for complex keys`
- `fix(scanner): resolve quote style detection bug`
- `docs(readme): update installation instructions`
- `perf(emitter): optimize string allocation`

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`

### Code Guidelines

#### General Principles

1. **Safety First**: No `unsafe` code without exceptional justification
2. **Performance**: Zero-copy parsing where possible
3. **Error Handling**: Comprehensive error context with positions
4. **Documentation**: All public APIs must be documented
5. **Testing**: All features must have tests

#### Engineering Principles

- **DRY** — one representation per piece of knowledge. Where two places must
  agree, generate one from the other (`commit-types.toml` → `committed.toml`,
  `cliff.toml`, `.gitmessage`) or add a drift check (`mise run version:check`).
- **KISS** — prefer the straightforward solution over the clever one.
- **YAGNI** — build what is needed now, not what might be needed later.
- **TDD** — write the failing test first, then the smallest change that passes
  it, then refactor.
- **SOLID** — single responsibility; open for extension, closed for
  modification; subtypes substitutable for their base; no client forced to
  depend on methods it does not use; depend on abstractions, not concretions.
- **Separation of concerns** — the load pipeline (`scanner` → `parser` →
  `composer` → `constructor`) and the dump pipeline (`representer` →
  `serializer` → `emitter`) are the architectural seams. Keep changes inside
  the stage that owns the concern.
- **Boy Scout rule** — leave touched code cleaner than you found it. This is
  scoped to what you touch; unrelated cleanup belongs in its own PR.
- **Dependency injection** — pass collaborators in rather than constructing
  them inside, so they can be substituted in tests.

#### Code Quality

Thresholds are enforced by `.clippy.toml`, not by review — treat that file as
the source of truth and this list as a summary:

- Cognitive complexity ≤ 30 per function; ≤ 10 function arguments; type
  complexity ≤ 300.
- New modules start small and focused. `src/scanner/mod.rs` and
  `src/parser/mod.rs` are known outliers (4,100 and 2,600 lines); they are not
  a licence to grow others, and splitting them is welcome when you are already
  working there.
- Meaningful names, no abbreviations.
- Validate all external input. Never hardcode secrets.
- Profile before optimizing — `mise run bench` and `docs/PROFILING.md`.

#### Rust Style

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` with default settings — `mise run cargo:fmt`
- MSRV is **1.85**, edition **2024**. Do not use APIs newer than the MSRV;
  both are declared once in `[workspace.package]`.
- `cargo:clippy:strict` is the gate that matters. It denies `clippy::all` and
  `clippy::pedantic` on top of `-D warnings`, so code passing plain
  `cargo clippy` can still be rejected by CI.
- **No `unwrap()` / `expect()` / panics on scanner or parser input paths.**
  `[profile.release]` sets `panic = "abort"`, so a panic on untrusted input
  kills the host process — it is a denial-of-service bug, not a crash.
- The crate is `#![deny(unsafe_code)]` (`src/lib.rs`). The single exception is
  the memory map in `src/streaming_async.rs`, which carries a local
  `#[allow(unsafe_code)]` and a `// SAFETY:` comment stating the caller
  contract. Any new `unsafe` needs both, plus a reason it cannot be safe.
- Errors are `Result` plus the crate's own `Error` enum (`src/error.rs`). This
  crate deliberately depends on neither `thiserror` nor `anyhow`; keep it that
  way and extend `Error` instead.
- `#[must_use]` on builders and on returns that are meaningless to discard.
- Prefer borrowing over cloning on hot paths; take `&str` over `String`.
- Use `IndexMap` for preserving key order in mappings.
- `mise run cargo:deny` and `mise run cargo:audit` gate licences and advisories.

#### Before You Commit

The `hk` pre-commit hook runs most of this automatically (see
[Git hooks](docs/PRE_COMMIT.md)), but the full local gate is:

```bash
mise run cargo:fmt            # 1. format
mise run cargo:clippy:strict  # 2. lint at the strictest setting — zero warnings
mise run test                 # 3. tests
mise run ci                   # or all of the above, as CI runs it
```

Do not leave throwaway test scripts or test-output directories in the tree;
generate them, use them, remove them.

#### YAML Implementation

- Full YAML 1.2 specification compliance
- Round-trip preservation (parse → serialize → parse)
- Secure by default (no code execution)
- Clear error messages with position information

## Testing

### Running Tests

```bash

# Run all tests
cargo test

# Run specific test suite
cargo test test_merge_keys
cargo test integration_tests

# Run with coverage
cargo tarpaulin --out html

# Run benchmarks
cargo bench
```

### Writing Tests

#### Unit Tests

Place unit tests in the same file as the code being tested:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_name() {
        // Test implementation
    }
}
```

#### Integration Tests

Place integration tests in the `tests/` directory:

```rust
use rust_yaml::Yaml;

#[test]
fn test_real_world_scenario() {
    let yaml = Yaml::new();
    // Test real-world usage
}
```

#### Test Categories

- **Basic Functionality**: Core parsing and generation
- **YAML 1.2 Compliance**: Specification conformance
- **Advanced Features**: Anchors, merge keys, complex structures
- **Error Handling**: Invalid input and edge cases
- **Performance**: Benchmarks and stress tests
- **Round-trip**: Parse → serialize → parse consistency

### Test Data

For YAML test files, use the `tests/data/` directory:

- Valid YAML files: `tests/data/valid/`
- Invalid YAML files: `tests/data/invalid/`
- Edge cases: `tests/data/edge_cases/`

## Code Style

### Formatting

```bash

# Format code
cargo fmt

# Check formatting
cargo fmt -- --check
```

### Linting

```bash

# Run clippy
cargo clippy -- -D warnings

# Fix clippy suggestions (carefully)
cargo clippy --fix
```

### Documentation

```bash

# Build documentation
cargo doc --open

# Check documentation links
cargo doc --no-deps
```

#### Documentation Guidelines

- Document all public APIs with examples
- Include error conditions in documentation
- Provide usage examples for complex features
- Link to relevant YAML specification sections

## Submitting Changes

### Before Submitting

1. **Rebase** your branch on the latest `main`:

   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Run the full test suite**:

   ```bash
   cargo test
   cargo clippy -- -D warnings
   cargo fmt -- --check
   ```

3. **Update documentation** if needed
4. **Add tests** for new features
5. **Update CHANGELOG.md** for significant changes

### Pull Request Process

1. **Create a descriptive title**:
   - `Add support for streaming YAML parsing`
   - `Fix memory leak in large document processing`

2. **Write a detailed description**:

   ```markdown
   ## Summary

   Brief description of changes

   ## Changes

   - Specific change 1
   - Specific change 2

   ## Testing

   - [ ] All existing tests pass
   - [ ] Added new tests for feature X
   - [ ] Manual testing completed

   ## Performance Impact

   Description of any performance changes

   ## Breaking Changes

   List any breaking changes
   ```

3. **Link related issues**: Use `Fixes #123` or `Closes #123`

### Review Process

- At least one maintainer review required
- All CI checks must pass
- Address all review feedback
- Keep the PR focused and atomic

## Security

For security vulnerabilities, please see our [Security Policy](SECURITY.md). Do not report security issues through public GitHub issues.

## Community

### Getting Help

- **GitHub Issues**: For bugs and feature requests
- **Discussions**: For questions and general discussion
- **Documentation**: Check the docs first

### Ways to Contribute

#### Code Contributions

- Bug fixes
- New features
- Performance improvements
- Code refactoring

#### Documentation

- API documentation improvements
- Usage examples
- Tutorials and guides
- README enhancements

#### Testing

- Test case improvements
- Performance benchmarks
- YAML specification compliance tests
- Edge case testing

#### Community

- Help answer questions
- Review pull requests
- Participate in discussions
- Share usage examples

## Release Process

(For maintainers)

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create release PR
4. After merge, tag release: `git tag v1.2.3`
5. Push tags: `git push --tags`
6. GitHub Actions will handle publishing

## Development Tips

### Performance Testing

```bash

# Run benchmarks
cargo bench

# Profile specific benchmark
cargo bench --bench parsing -- --profile-time=10
```

### Memory Testing

```bash

# Check for memory leaks
cargo test --features large-documents
valgrind --tool=memcheck cargo test
```

### Fuzzing

```bash

# Install cargo-fuzz
cargo install cargo-fuzz

# Run fuzzer
cargo fuzz run parse_yaml
```

## Architecture Notes

### Key Components

1. **Scanner** (`src/scanner/`): Tokenizes YAML input
2. **Parser** (`src/parser/`): Generates parsing events
3. **Composer** (`src/composer.rs`): Builds node tree from events
4. **Emitter** (`src/emitter.rs`): Serializes values back to YAML

### Design Principles

- **Streaming**: Process large documents efficiently
- **Zero-copy**: Minimize memory allocations
- **Error Context**: Always provide position information
- **Round-trip**: Preserve formatting and structure

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

---

Thank you for contributing to rust-yaml! 🎉
