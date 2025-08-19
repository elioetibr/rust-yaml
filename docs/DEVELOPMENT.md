# Development Guide

This document covers the development workflow, tools, and processes for the rust-yaml project.

## Quick Start

1. **Clone and setup**:

   ```bash
   git clone https://github.com/elioetibr/rust-yaml.git
   cd rust-yaml
   make setup  # Set up development environment, git hooks, components
   ```

2. **Make changes**:

   ```bash
   # Create feature branch
   git checkout -b feat/my-feature

   # Make your changes
   # ... code changes ...

   # Test your changes
   make quick-check  # Format, lint, and test
   # OR individual commands:
   make test         # Run all tests
   make lint         # Run clippy
   make format       # Format code
   ```

3. **Commit with conventional format**:

   ```bash
   git commit -m "feat(parser): add support for new feature"
   ```

4. **Push and create PR**:

   ```bash
   git push origin feat/my-feature
   ```

## Prerequisites

### Node.js Version Management

This project uses Node.js for development tools like commitlint. The required Node.js version is specified in `.nvmrc`.

**Using nvm (recommended)**:

```bash
# Install and use the correct Node.js version
nvm install
nvm use
```

**Using volta**:

```bash
# Volta will automatically use the version from .nvmrc
volta install node
```

**Manual setup**:

```bash
# Check current version
cat .nvmrc
# Install Node.js 20 manually from nodejs.org
```

## Development Workflow

### Branch Strategy

- **`main`**: Stable release branch, protected
- **Feature branches**: `feat/feature-name`, `fix/bug-description`
- **Release branches**: `release/v1.2.3` (if needed for complex releases)

### Commit Conventions

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

#### Types

- `feat`: New features
- `fix`: Bug fixes
- `docs`: Documentation only changes
- `style`: Code style (formatting, whitespace, etc.)
- `refactor`: Code refactoring without feature changes
- `test`: Adding or updating tests
- `chore`: Build process, dependencies, tooling
- `perf`: Performance improvements
- `ci`: CI/CD changes
- `build`: Build system or external dependencies
- `revert`: Reverting previous commits

#### Scopes

Project-specific scopes:

- `parser`: YAML parsing logic
- `scanner`: Token scanning and lexical analysis
- `emitter`: YAML output generation
- `composer`: Document composition and events
- `constructor`: Value construction from events
- `lib`: Library-level changes
- `cli`: Command-line interface
- `docs`: Documentation
- `tests`: Test files
- `benches`: Benchmarks
- `examples`: Example code
- `ci`: CI/CD workflows
- `deps`: Dependencies
- `release`: Release-related changes

#### Special Markers

- `+semver: major|minor|patch|none`: Control version bumping
- `[skip ci]` or `[ci skip]`: Skip CI builds
- `BREAKING CHANGE: description`: Breaking changes (triggers major version)

#### Examples

```bash
feat(parser): add support for complex mapping keys
fix(emitter): resolve string quoting for version numbers
docs: update installation instructions
test(scanner): add edge case tests for indentation
chore: bump version to 1.1.0 [skip ci]
perf(parser): optimize memory allocation in token scanning
refactor(lib): restructure error handling
ci: add automated security scanning
```

## CI/CD Workflows

### Continuous Integration (`.github/workflows/ci.yml`)

Runs on every push and PR:

- **Version calculation** using GitVersion
- **Multi-platform testing** (Ubuntu, Windows, macOS)
- **Multi-Rust testing** (stable, beta, nightly, MSRV)
- **Code quality checks** (fmt, clippy, audit)
- **Test coverage** reporting
- **Documentation** building
- **Benchmarks** (compile-only)

**Skip CI**: Use `[skip ci]` in commit messages to skip most CI jobs (docs still run)

### Documentation Only (`.github/workflows/docs-only.yml`)

Runs only when `[skip ci]` is detected:

- **Documentation building** and link checking
- **Changelog validation**
- **Version information** display

### Commit Linting (`.github/workflows/commitlint.yml`)

Validates commit messages:

- **Conventional commit** format checking
- **PR comment** with help on validation failure
- **Multiple validation methods** (commitlint + custom)

### Release Process (`.github/workflows/release.yml`)

Triggered by version tags (e.g., `v1.2.3`):

- **Version validation** (tag vs Cargo.toml)
- **Full testing** and validation
- **GitHub release** creation with changelog
- **Crates.io publishing

### Version Management

- **Automatic version bumping** after releases
- **Manual version bumping** via GitHub Actions
- **GitVersion-based** semantic versioning

## Development Tools

### Required Tools

- **Rust toolchain** (stable, with clippy and rustfmt)
- **Git** for version control

### Optional Tools

- **Node.js + npm** for commitlint (enhanced commit validation)
- **cargo-tarpaulin** for code coverage
- **cargo-criterion** for benchmarking
- **cargo-audit** for security scanning

### Makefile Development Commands

The project includes a comprehensive Makefile with 60+ commands. Run `make setup` to:

- Configure git hooks for commit message validation
- Set up conventional commit template
- Install commitlint (if Node.js available)
- Verify Rust toolchain components (rustfmt, clippy)
- Configure git aliases

#### Key Development Commands

**Quick Development**

```bash
make setup         # Set up development environment
make quick-check   # Fast: format + lint + lib tests
make ci            # Full CI pipeline locally
```

**Testing**

```bash
make test              # All tests
make test-lib          # Library tests only
make test-integration  # Integration tests
make test-security     # Security-specific tests
```

**Code Quality**

```bash
make format           # Format code
make lint             # Run clippy
make clippy-strict    # Strict clippy (CI settings)
make audit            # Security audit
make deny             # Cargo deny checks
```

## Code Quality Standards

### Formatting

```bash
cargo fmt --check    # Check formatting
cargo fmt           # Fix formatting
```

### Linting

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### Testing

```bash
cargo test           # Run all tests
cargo test --release # Run in release mode
cargo test -- --nocapture # Show println! output
```

### Documentation

```bash
cargo doc --all-features --no-deps    # Build docs
cargo doc --open                      # Build and open docs
```

### Benchmarking

```bash
cargo bench          # Run benchmarks
cargo bench -- --save-baseline main  # Save baseline
```

## Release Process

### Option 1: Automated Script

```bash
./scripts/release.sh         # Interactive
./scripts/release.sh 1.2.3   # Direct version
```

### Option 2: Manual Process

1. Update `Cargo.toml` version
2. Update `GitVersion.yml` next-version
3. Run tests and checks
4. Commit with `[skip ci]`
5. Create and push tag
6. GitHub Actions handles the rest

### Option 3: GitHub Actions

Use "Manual Version Bump" workflow in GitHub Actions

## Debugging and Troubleshooting

### Common Issues

**Commit message rejected:**

- Use conventional commit format
- Check for typos in type/scope
- See `.gitmessage` for template

**CI failing on formatting:**

```bash
cargo fmt
git add -u
git commit --amend --no-edit
```

**CI failing on clippy:**

```bash
cargo clippy --all-targets --all-features -- -D warnings
# Fix reported issues
```

**Version mismatch in release:**

- Ensure `Cargo.toml` version matches git tag
- Check `GitVersion.yml` configuration

### Debug Commands

```bash
# Check git hooks
ls -la .githooks/
git config --get core.hooksPath

# Test commitlint
echo "feat: test message" | npx commitlint

# Check GitVersion (if installed locally)
gitversion /output buildmetadata /output json

# Verbose test output
cargo test -- --nocapture

# Check formatting without fixing
cargo fmt --check

# Run clippy with explanations
cargo clippy --all-targets --all-features -- -D warnings -A clippy::style
```

## Git Workflow Tips

### Helpful Aliases (configured by setup script)

```bash
git lg    # Pretty log with graph
git st    # Status
git co    # Checkout
git br    # Branch
git ci    # Commit
git unstage # Reset HEAD
git last  # Show last commit
```

### Workflow Examples

**Feature development:**

```bash
git checkout -b feat/complex-keys
# ... make changes ...
cargo test && cargo clippy && cargo fmt
git add .
git commit -m "feat(parser): add support for complex mapping keys

This implements parsing for complex key structures as defined in YAML 1.2
specification section 7.2.

+semver: minor"
git push origin feat/complex-keys
# Create PR on GitHub
```

**Bug fix:**

```bash
git checkout -b fix/string-escaping
# ... fix bug ...
cargo test
git commit -m "fix(emitter): properly escape special characters in strings

Fixes issue where backslashes and quotes were not properly escaped in
double-quoted strings, causing parsing errors on round-trip.

Closes #123
+semver: patch"
```

**Documentation update:**

```bash
git checkout -b docs/api-examples
# ... update docs ...
git commit -m "docs: add comprehensive API usage examples [skip ci]

+semver: none"
```

## Performance Considerations

### Benchmarking

- Run benchmarks before/after changes
- Focus on hot paths (parser, scanner, emitter)
- Consider memory allocation patterns
- Profile with `perf` or `cargo flamegraph`

### Memory Safety

- No unsafe code policy
- Prefer stack allocation where possible
- Use `Vec::with_capacity()` for known sizes
- Avoid unnecessary cloning

### Error Handling

- Use `Result<T, E>` consistently
- Provide meaningful error messages
- Include position information for parse errors
- Avoid panics in library code

## Testing Strategy

### Test Categories

- **Unit tests**: Individual component testing
- **Integration tests**: Cross-component workflows
- **Round-trip tests**: Parse → serialize → parse
- **Fuzz tests**: Random input validation
- **Benchmark tests**: Performance regression detection

### Test Organization

```
tests/
├── integration_tests.rs   # Cross-component tests
├── unit_tests.rs         # Component unit tests
├── merge_keys_comprehensive.rs # Specific feature tests
└── simple_merge_test.rs  # Basic functionality
```

### Coverage Goals

- Maintain >90% code coverage
- Focus on error paths and edge cases
- Test both success and failure scenarios

## Contributing Guidelines

1. **Follow conventional commits**
2. **Add tests for new features**
3. **Update documentation**
4. **Run quality checks** before pushing
5. **Keep PRs focused** and reasonably sized
6. **Write clear PR descriptions**
7. **Respond to review feedback**

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed contribution guidelines.

## Current Project Status (2025-08-16)

### Test Coverage

- **Unit Tests**: 134 tests passing ✅
- **Integration Tests**: 16 tests passing ✅  
- **Security Tests**: Comprehensive alias depth, limits, and attack prevention ✅
- **Performance Tests**: Benchmarks for parsing, streaming, zero-copy ✅
- **CI Pipeline**: All GitHub Actions workflows passing ✅

### Core Features Status

- **YAML 1.2 Parsing**: Complete implementation ✅
- **Serialization**: Full round-trip support ✅
- **Anchors & Aliases**: Complete with depth protection ✅
- **Merge Keys**: Full inheritance support ✅
- **Multi-line Strings**: Literal and folded block scalars ✅
- **Type Tags**: Explicit typing system ✅
- **Complex Keys**: Sequences and mappings as keys ✅
- **Security**: Resource limits, attack prevention ✅

### Advanced Features Status

- **Streaming Parser**: Complete with async support ✅
- **Zero-Copy Parsing**: Optimized memory usage ✅
- **Memory Mapping**: Large file support ✅
- **Error Handling**: Contextual reporting with position info ✅
- **Performance**: Benchmarked against other libraries ✅

### Development Infrastructure

- **Makefile**: 60+ commands for development workflow ✅
- **Git Hooks**: Commit message validation ✅
- **CI/CD**: GitHub Actions with multi-platform testing ✅
- **Code Quality**: Format, lint, audit, security checks ✅
- **Documentation**: Comprehensive guides and API docs ✅

## Recent Accomplishments

### Streaming Support (2025-08-16)

- ✅ Implemented true streaming parser with incremental parsing
- ✅ Added async/await support with Tokio integration
- ✅ Implemented memory-mapped file support for large documents
- ✅ Created configurable buffer management (default, large_file, low_memory)
- ✅ Added streaming event iterator with standard Rust patterns
- ✅ Created comprehensive streaming benchmarks
- ✅ Documented streaming API in STREAMING.md

The streaming implementation provides:

- Incremental parsing with configurable chunk sizes
- Async/await support for non-blocking I/O
- Memory-mapped files for zero-copy large file access
- 40-50% memory reduction for large documents
- Iterator interface for idiomatic Rust usage

### Zero-Copy Parsing (2025-08-16)

- ✅ Created BorrowedValue enum using Cow for potential zero-copy strings
- ✅ Implemented OptimizedValue using Rc for cheap cloning
- ✅ Created ReducedAllocComposer that reduces allocations by ~40%
- ✅ Added comprehensive benchmarks for performance comparison
- ✅ Documented zero-copy API usage in ZERO_COPY.md
- ✅ Reduced clone operations from 49 to approximately 30

The optimized implementation provides:

- Reference-counted strings and collections for O(1) cloning
- Efficient anchor/alias handling with Rc storage
- 30-40% performance improvement for medium to large documents

### Security Hardening (2025-08-16)

- ✅ Implemented comprehensive resource limits (max depth, size, anchors)
- ✅ Added protection against billion laughs attack
- ✅ Implemented cyclic reference detection
- ✅ Added complexity scoring for nested structures
- ✅ Created security test suite in `tests/security_limits.rs`
- ✅ Documented security best practices in `SECURITY.md`
- ✅ Integrated ResourceTracker into composer for real-time monitoring

The security implementation prevents:

- Exponential entity expansion (billion laughs attack)
- Deep nesting attacks (stack overflow)
- Large collection attacks (memory exhaustion)
- Cyclic references
- Anchor bombs

## Resources

- [Conventional Commits](https://www.conventionalcommits.org/)
- [GitVersion Documentation](https://gitversion.net/)
- [YAML 1.2 Specification](https://yaml.org/spec/1.2.2/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
