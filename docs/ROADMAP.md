# rust-yaml Roadmap

## Overview

This roadmap outlines the planned improvements and enhancements for rust-yaml, focusing on YAML 1.2 specification compliance, performance, security, and code quality.

## Current Status (2026-05-22)

> 🎯 **100% upstream yaml-test-suite conformance achieved** —
> 735 / 735 spec-conformance tests pass against the
> [`yaml/yaml-test-suite`](https://github.com/yaml/yaml-test-suite)
> `data-2022-01-17` pin. See
> [`YAML_CONFORMANCE_ROADMAP.md`](YAML_CONFORMANCE_ROADMAP.md).

### ✅ Fully Implemented & Production Ready

**Core YAML 1.2 Features**

- Complete YAML parsing and emission
- All scalar types (null, bool, int, float, string)
- Collections (sequences, mappings) with complex key support
- Block and flow styles with auto-detection
- Anchors and aliases with security depth limits
- Block scalars (literal `|` and folded `>`)
- Full type tag system with custom tag support
- Multi-document support
- YAML directives (`%YAML`, `%TAG`)
- Merge keys (`<<`) with inheritance and override behavior
- Round-trip preservation with formatting retention
- Memory-safe implementation (zero unsafe code)

**Advanced Features**

- Streaming parser with async/await support
- Zero-copy parsing optimizations (40% memory reduction)
- Memory-mapped file support for large documents
- Comprehensive security limits and attack prevention
- Error reporting with precise position and context
- Indentation style detection and preservation

**Development Infrastructure**

- 210+ lib unit tests passing (223 with `--all-features`)
- 200+ integration tests passing
- **735 / 735 yaml-test-suite conformance tests passing (100.0%)**
- Comprehensive security test suite
- Performance benchmarks
- 60+ mise tasks for development
- CI/CD pipeline with multi-platform testing

### ✅ Recently Completed (2026-05-18)

- **100% yaml-test-suite conformance** (735 / 735) achieved across
  68 TDD-driven fixes covering line-start property handling,
  state-machine completeness, indent rules, explicit-key wrapping,
  flow-collection-as-key, and inline single-pair mapping wraps.
- **YAML 1.1 `=` (`!!value`) tag auto-detection** (§10.3.4):
  bare `=` under `%YAML 1.1` is now recognized as
  `tag:yaml.org,2002:value` and rejected with a construction error,
  matching `ruamel.yaml typ="safe"` / `typ="unsafe"`. Closes the
  parity gap from [#1](https://github.com/elioetibr/rust-yaml/issues/1).
  Default 1.2 keeps `=` as a plain string. See
  [`YAML_1.2.2_COMPLIANCE.md`](YAML_1.2.2_COMPLIANCE.md).
- Strict clippy gate (`-D warnings -D pedantic` with curated allow-list)
  now blocking — every fix must pass `mise run ci` before merge.

### ✅ Earlier (2025-08-16)

- Security hardening with resource limits
- Streaming support with configurable buffers
- Zero-copy value types with Rc optimization
- Alias depth enforcement and cyclic reference detection
- Performance benchmarks and optimization
- Comprehensive development tooling (mise)

### ❌ Future Enhancements

- Full Unicode normalization
- Comment preservation during editing (round-trip already done; in-place edits still open)
- Language server protocol support

## v1.1.0 Milestone — In Progress

Granular delivery is tracked in the GitHub
[v1.1.0 milestone](https://github.com/elioetibr/rust-yaml/milestone/2).
Status as of 2026-05-22 — **9 of 15 issues implemented**.

### ✅ Implemented — PR #67 (open, awaiting merge)

| Issue | Change |
|-------|--------|
| #20 | `fix(scanner)` — `peek_char` `isize::MIN` offset overflow |
| #24 | `fix(scanner)` — cap anchor/alias name length while scanning |
| #25 | `fix(streaming)` — `MmapYamlReader` UTF-8 errors + char-boundary splits |
| #26 | `perf(scanner)` — drop the `char_indices` side table + O(n) directive resets |
| #27 | `perf(error)` — build error context from a line window, not all lines |
| #28 | `test(fuzz)` — cargo-fuzz harness (`load_str`, `load_str_strict`, `roundtrip`) |
| #66 | `fix(scanner)` — numeric mapping key parsed as a bare scalar |
| #22 | `fix(resolver)` — implicit hex/octal/binary ints + dotted inf/nan |
| #21 | `feat(serde)` — full `Serialize`/`Deserialize` data format + `Value` impls |

`#66` was discovered by the new `roundtrip` fuzz target (#28). `#21` ships
the full serde data format (`from_str`/`from_slice`/`from_reader` /
`to_string`/`to_writer`) plus `Serialize` / `Deserialize` for `Value`,
behind the existing `serde` feature. See
[`SERDE_INTEGRATION_DESIGN.md`](SERDE_INTEGRATION_DESIGN.md) for the
spec; non-unit enum variants use the single-entry-mapping form rather
than `serde_yaml`'s tagged form (documented divergence).

### 🚧 Remaining — P1

| Issue | Change | Effort |
|-------|--------|--------|
| #23 | `feat(errors)` — recoverable parsing + multi-error collection | Large (~2 wk) |

`#23` is non-breaking via an additive constructor (see the issue), and
needs a dedicated multi-session effort.

### 🚧 Remaining — P2 / P3

- #29 `feat(emit)` — full quote-style preservation on round-trip
- #30 `feat(types)` — construct `!!omap` / `!!pairs` / `!!set` as distinct types
- #31 `perf(scanner)` — trim plain scalars in place instead of re-allocating
- #32 `perf(scanner)` — fixed-size array in space-indentation detection
- #33 `reliability(composer)` — short-circuit complexity calc when limit is `usize::MAX`

Issue #34 (`refactor(resolver)` — delete dead `implicit_resolvers`) was moved
to the **v2.0.0** milestone: removing it necessarily breaks the public
`add_implicit_resolver` API.

### Release checklist

- [ ] All milestone issues closed or moved to the v1.2.0 milestone
- [ ] Version bump `1.0.x` → `1.1.0` (`Cargo.toml` **and** `src/version.rs`)
- [ ] `CHANGELOG.md` entry for 1.1.0
- [ ] Tag + `release.yml` publish (`GitVersion.yml` drives semver)

## Priority 1: YAML 1.2 Specification Compliance 🎯

### 1.1 Directives Support ✅

- [x] Implement `%YAML` directive for version specification
- [x] Implement `%TAG` directive for tag shorthand definitions
- [x] Add directive preservation for round-trip

### 1.2 Merge Keys ✅

- [x] Implement merge key (`<<`) functionality
- [x] Support multiple merge keys
- [x] Handle merge key conflicts properly
- [x] Add comprehensive tests

### 1.3 Advanced Tag Support ✅

- [x] Implement full tag resolution mechanism
- [x] Support custom tag handlers
- [x] Add schema validation framework
- [x] Implement binary data tags

### 1.4 Character Encoding

- [ ] Full UTF-16 and UTF-32 support
- [ ] Byte Order Mark (BOM) handling
- [ ] Unicode normalization
- [ ] Non-printable character handling

## Priority 2: Performance & Scalability 🚀

### 2.1 Zero-Copy Parsing

- [x] Implement borrowing-based API to reduce allocations
- [x] Use `Cow<'a, str>` for strings where possible
- [x] Reduce unnecessary cloning (49 clone calls reduced to ~30)
- [x] Implement Rc-based OptimizedValue for cheap cloning
- [ ] Implement string interning for repeated values

### 2.2 Streaming Support

- [x] Implement true streaming parser for large documents
- [x] Add async/await support for I/O operations
- [x] Implement incremental parsing
- [x] Add memory usage limits and controls
- [x] Memory-mapped file support for efficient large file processing
- [x] Configurable buffer sizes and chunk processing

### 2.3 Re-enable Optimizations

- [ ] Review and re-enable `scanner::optimizations`
- [ ] Review and re-enable `parser::optimizations`
- [ ] Implement SIMD optimizations for scanning
- [ ] Add compile-time feature flags for optimizations

### 2.4 Benchmarking & Profiling

- [ ] Expand benchmark suite coverage
- [ ] Add memory usage benchmarks
- [ ] Implement continuous performance monitoring
- [ ] Create performance regression tests

## Priority 3: Security & Reliability 🔒

### 3.1 Security Hardening ✅

- [x] Implement resource limits (max depth, max size, max anchors)
- [x] Add protection against billion laughs attack
- [x] Implement alias depth tracking and enforcement
- [x] Add cyclic reference detection
- [x] Implement complexity scoring for nested structures
- [x] Add comprehensive security test suite
- [x] Document security best practices in SECURITY.md

### 3.2 Error Handling

- [ ] Implement recoverable error handling
- [ ] Add partial document recovery
- [ ] Improve error messages with suggestions
- [ ] Add error recovery strategies
- [ ] Implement strict vs lenient parsing modes

### 3.3 Validation

- [x] Add schema validation support (`src/schema.rs`: AllOf/AnyOf/OneOf/Not, conditional, custom predicates)
- [x] Support JSON Schema validation (`tests/schema_validation.rs`, `benches/schema_validation.rs`)
- [ ] Implement custom validation rules (extensible registry beyond the current `Custom` predicate)
- [ ] Add type checking at parse time (validate while events are emitted, not post-construction)

## Priority 4: Code Quality & Architecture 🏗️

### 4.1 DRY Violations to Fix

- [ ] Consolidate duplicate `new` and `new_eager` patterns across modules
- [ ] Unify error creation patterns
- [ ] Extract common token processing logic
- [ ] Centralize position tracking logic

### 4.2 KISS Violations to Fix

- [ ] Simplify complex nested match statements in parser
- [ ] Reduce state machine complexity in scanner
- [ ] Simplify event to value conversion logic
- [ ] Streamline the module hierarchy

### 4.3 SOLID Violations to Fix

#### Single Responsibility Principle (SRP)

- [ ] Split `BasicScanner` (now 4000+ lines in `src/scanner/mod.rs`) into smaller components
- [ ] Separate parsing logic from state management
- [ ] Extract validation logic from core components

#### Open/Closed Principle (OCP)

- [ ] Make tag handlers extensible without modifying core
- [ ] Allow custom scalar resolvers
- [ ] Support pluggable validation rules

#### Dependency Inversion Principle (DIP)

- [ ] Reduce direct dependencies between modules
- [ ] Introduce abstraction layers for I/O
- [ ] Use dependency injection for configuration

### 4.4 API Improvements

- [ ] Design cleaner public API surface
- [ ] Add builder patterns for configuration
- [ ] Implement `From`/`Into` traits consistently
- [ ] Add more ergonomic error types

## Priority 5: Features & Ecosystem 🌟

### 5.1 Serde Integration

- [ ] Complete serde serialization support
- [ ] Add serde deserialization support
- [ ] Support custom derive macros
- [ ] Add serde benchmarks

### 5.2 Format Preservation

- [ ] Full comment preservation
- [ ] Whitespace preservation
- [ ] Quote style preservation
- [ ] Line ending preservation
- [ ] Complete round-trip fidelity

### 5.3 Developer Experience

- [ ] Improve documentation with more examples
- [ ] Add comprehensive usage guide
- [ ] Create migration guide from other YAML libraries
- [ ] Add debugging utilities
- [ ] Implement pretty-printing options

### 5.4 Testing

- [ ] Achieve >90% code coverage
- [x] Add fuzzing tests (cargo-fuzz harness in `fuzz/`: `load_str`, `load_str_strict`, `roundtrip` — #28)
- [ ] Implement property-based testing
- [x] Add YAML test suite compliance tests (735/735 against `yaml/yaml-test-suite` `data-2022-01-17`; harness in `yaml-test-suite/`, run via `mise run test-yaml-suite`)
- [ ] Create integration tests with popular frameworks

## Priority 6: Advanced Features 🔧

### 6.1 Language Server Protocol

- [ ] Implement YAML language server
- [ ] Add syntax highlighting support
- [ ] Implement auto-completion
- [ ] Add real-time validation

### 6.2 Tooling

- [ ] Create YAML formatter CLI
- [ ] Add YAML linter
- [ ] Implement YAML diff tool
- [ ] Create YAML merge tool

### 6.3 Ecosystem Integration

- [ ] Add support for configuration frameworks
- [ ] Integrate with popular web frameworks
- [ ] Support for Kubernetes manifests
- [ ] Docker Compose file support

## Timeline Estimates

### ✅ Completed (Q1-Q3 2025)

- YAML 1.2 specification compliance ✅
- Security hardening with comprehensive protection ✅
- Streaming support with async/await ✅
- Zero-copy performance optimizations ✅
- Development infrastructure and tooling ✅

### Phase 1: Q4 2025

- Schema validation framework ✅ (`src/schema.rs`)
- Full serde integration (still a 28-line stub in `src/serde_integration.rs`)
- Enhanced error recovery
- Developer experience improvements

### Phase 2: Q1 2026

- Language server protocol support
- Advanced formatting and linting tools
- Comment preservation during editing
- Plugin architecture for extensibility

### Phase 3: Q2 2026

- Advanced features and ecosystem integration
- Performance monitoring and regression testing
- Tooling development (CLI tools, formatters)
- Kubernetes/Docker Compose specific optimizations

## Success Metrics

- **Specification Compliance**: Pass 100% of YAML test suite
- **Performance**: Match or exceed performance of leading YAML libraries
- **Security**: Zero security vulnerabilities in audits
- **Code Quality**: Maintain A+ rating on code quality tools
- **Adoption**: Achieve 10K+ downloads per month
- **Documentation**: 100% public API documentation coverage

## Contributing

We welcome contributions! Priority areas for contributors:

1. **Immediate needs**:
   - Full serde integration (current `serde_integration.rs` is a stub)
   - Enhanced error recovery and partial-document recovery
   - Performance regression testing
   - Re-enable `scanner::optimizations` / `parser::optimizations` (currently commented out)

2. **Good first issues**:
   - Adding more comprehensive tests
   - Improving documentation with examples
   - Implementing missing Unicode normalization
   - Adding benchmarks for specific use cases

3. **Advanced contributions**:
   - Language server protocol implementation
   - Comment preservation during editing
   - Advanced formatting tools
   - Plugin architecture design

Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Breaking Changes Policy

- Version 0.x: Breaking changes allowed in minor versions
- Version 1.0: Semantic versioning strictly followed
- Deprecation period: Minimum 2 minor versions
- Migration guides provided for all breaking changes

## References

- [YAML 1.2 Specification](https://yaml.org/spec/1.2.2/)
- [YAML Test Suite](https://github.com/yaml/yaml-test-suite)
- [ruamel.yaml Documentation](https://yaml.readthedocs.io/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

---

_Last updated: 2026-05-22_
_Next review: 2026-07-22_
