<!--
SPDX-FileCopyrightText: Rust Yaml contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# YAML 1.2.2 Compliance

This document tracks `rust-yaml`'s conformance to the [YAML 1.2.2 specification](https://yaml.org/spec/1.2.2/) and notes deviations from / compatibility with [YAML 1.1](https://yaml.org/spec/1.1/).

It is **descriptive**, not aspirational: every entry below reflects the current behavior of the codebase on `main`. Code/test references are included so this doc can be audited against the source.

## Legend

- ✅ **Implemented** — feature works and has test coverage.
- 🟡 **Partial** — works for the common case but has documented gaps.
- ❌ **Gap** — required by the spec but not yet implemented.
- 🔵 **Out of scope (1.1 only)** — feature exists in YAML 1.1 only; intentionally not part of 1.2.2.
- ⏸ **Deferred** — recognized requirement, not prioritized for the current milestone.

## YAML 1.2.2 chapter-by-chapter

### Chapter 5 — Character productions

| Feature                                                                          | Status | Notes                                                   |
| -------------------------------------------------------------------------------- | ------ | ------------------------------------------------------- |
| UTF-8 input                                                                      | ✅     | Strings are `&str`, encoding handled by Rust            |
| UTF-16 / UTF-32 BOM detection                                                    | ❌     | Not implemented                                         |
| Indicator characters (`[ ] { } , - ? : ! & \* # ; , > \| ' " % @ \``)            | ✅     | Recognized by the scanner                               |
| Escape sequences in `"..."` scalars (`\n`, `\t`, `\x..`, `\u....`, `\U........`) | 🟡     | Common escapes work; full table not exhaustively tested |
| Line break normalization (`\r\n` → `\n`)                                         | ✅     | Handled in scanner                                      |

### Chapter 6 — Structural productions

| Feature                                 | Status | Notes                                                                                                                                                                                                                                                                                                                                                     |
| --------------------------------------- | ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Indentation (spaces only, no tabs)      | ✅     | `src/scanner/indentation.rs`                                                                                                                                                                                                                                                                                                                              |
| `# comment`                             | ✅     | `tests/comment_preservation_tests.rs`                                                                                                                                                                                                                                                                                                                     |
| `%YAML version` directive parsed        | ✅     | `src/parser/mod.rs:319` populates `yaml_version`                                                                                                                                                                                                                                                                                                          |
| `%YAML 1.1` honored for bool resolution | ✅     | Composers read `Event::DocumentStart`'s version and call `resolver::resolve_plain_scalar(value, version)` (`src/composer.rs`, `composer_borrowed.rs`, `composer_comments.rs`, `composer_optimized.rs`). 1.1 enables `yes`/`no`/`on`/`off` as bools; 1.2 (default) does not. See `tests/directives.rs` `test_yaml_version_1_1_compatibility` and siblings. |
| `%YAML 1.1` honored for `=` / `!!value` | ✅     | Bare `=` under `%YAML 1.1` is detected by `resolver::resolve_plain_scalar` as the `tag:yaml.org,2002:value` indicator (§10.3.4) and rejected by every composer via `resolver::value_tag_error` — matches `ruamel.yaml typ="safe"`/`typ="unsafe"`. Quoted `'='` or default 1.2 remain plain strings. See `src/constructor.rs::test_yaml_1_1_value_tag_rejected_with_directive`. |
| `%TAG handle prefix` directive          | ✅     | `tests/directives.rs`, `tests/directive_roundtrip.rs`                                                                                                                                                                                                                                                                                                     |
| Reserved directive handling             | 🟡     | Other directives parse but are not strictly validated                                                                                                                                                                                                                                                                                                     |

### Chapter 7 — Flow style productions

| Feature                                              | Status | Notes                                       |
| ---------------------------------------------------- | ------ | ------------------------------------------- |
| Flow sequences `[a, b, c]`                           | ✅     | `tests/flow_indent_bug.rs`                  |
| Flow mappings `{k: v}`                               | ✅     |                                             |
| Plain scalars in flow context                        | ✅     |                                             |
| Single-quoted scalars `'...'` with `''` escape       | ✅     |                                             |
| Double-quoted scalars `"..."` with backslash escapes | ✅     | See Chapter 5 caveat                        |
| Empty flow collections `[]` / `{}`                   | ✅     | Fixed in [#3](https://github.com/elioetibr/rust-yaml/issues/3) via `448e3f6` |

### Chapter 8 — Block style productions

| Feature                                             | Status | Notes                                                           |
| --------------------------------------------------- | ------ | --------------------------------------------------------------- |
| Block sequences (`- item`)                          | ✅     | Compact and extra-indented forms — fixed in `448e3f6`           |
| Block mappings (`key: value`)                       | ✅     |                                                                 |
| Block sequences nested in mappings                  | ✅     | The case from [#3](https://github.com/elioetibr/rust-yaml/issues/3)                              |
| Literal block scalars (`\|`, chomping `\|-`, `\|+`) | ✅     |                                                                 |
| Folded block scalars (`>`, chomping `>-`, `>+`)     | ✅     |                                                                 |
| Explicit block keys (`? key`)                       | ✅     | `tests/complex_keys.rs`                                         |
| Indentation indicator (`\|2`, `>4`)                 | 🟡     | Parsing works; less-common combinations not exhaustively tested |

### Chapter 9 — Document stream productions

| Feature                       | Status | Notes                                 |
| ----------------------------- | ------ | ------------------------------------- |
| `---` document start marker   | ✅     | `tests/directives.rs`                 |
| `...` document end marker     | ✅     |                                       |
| Multi-document streams        | ✅     | `Yaml::load_all_str` / `dump_all_str` |
| Implicit document (no `---`)  | ✅     |                                       |
| Bare document with directives | ✅     |                                       |

### Chapter 10 — Recommended schemas

| Schema                                             | Status | Constants                                                       |
| -------------------------------------------------- | ------ | --------------------------------------------------------------- |
| Failsafe (`!!map`, `!!seq`, `!!str`)               | ✅     | `Schema::Failsafe` in `src/tag.rs:342`                          |
| JSON (adds `!!null`, `!!bool`, `!!int`, `!!float`) | ✅     | `Schema::Json`                                                  |
| **Core** (default)                                 | ✅     | `Schema::Core` in `src/tag.rs:338`, default per `src/tag.rs:91` |

#### Core Schema resolution

Implicit (untagged) resolution happens in the **composer**, not the `Resolver` trait. Every composer variant (`src/composer.rs:300-318`, `composer_borrowed.rs:240-251`, `composer_comments.rs:195-206`, `composer_optimized.rs:240-251`) duplicates the same hardcoded sequence: try `i64::parse`, try `f64::parse`, try a YAML-1.1-style bool/null match, fall back to `String`. The `TagResolver` / `BasicResolver` is only consulted for **explicitly tagged** scalars like `!!int 0xFF`.

The table below was captured by parsing each input with `Yaml::new().load_str(...)` on `main` and recording the resulting `Value` variant:

| Input                                      | Implicit result | Spec-correct under 1.2 Core | Notes                                                                                                  |
| ------------------------------------------ | --------------- | --------------------------- | ------------------------------------------------------------------------------------------------------ |
| `null`, `~`                                | `Null` ✅       | `!!null`                    | Composer path                                                                                          |
| `true` / `false` (any case)                | `Bool` ✅       | `!!bool`                    | Composer matches case-insensitively                                                                    |
| `yes` / `no` / `on` / `off` (no directive) | `String` ✅     | `!!str`                     | Fixed — composer reads `%YAML` version; `yes`/`no`/`on`/`off` only resolve as `Bool` under `%YAML 1.1` |
| `42`, `-1` (decimal)                       | `Int` ✅        | `!!int`                     | `i64::parse`                                                                                           |
| `0xFF` (hex)                               | `String` ❌     | `!!int`                     | Only works via `!!int 0xFF` (`src/tag.rs:249`)                                                         |
| `0o17` (octal, 1.2 form)                   | `String` ❌     | `!!int`                     | Only works via `!!int 0o17` (`src/tag.rs:252`)                                                         |
| `0b101` (binary)                           | `String` 🔵     | n/a                         | Non-spec extension; works when tagged (`src/tag.rs:255`)                                               |
| `014` (octal, 1.1 form)                    | `Int(14)` ✅    | `!!int 14`                  | Spec says decimal-14; rust-yaml agrees                                                                 |
| `3.14`, `1e6`                              | `Float` ✅      | `!!float`                   | `f64::parse`                                                                                           |
| `inf`, `nan` (Rust forms)                  | `Float` ❌      | `!!str`                     | Composer's `f64::parse` accepts these — spec uses `.inf`/`.nan`                                        |
| `.inf`, `-.inf`, `.nan` (spec forms)       | `String` ❌     | `!!float`                   | Tagged construction works (`src/tag.rs:274-276`); implicit doesn't                                     |
| anything else                              | `String` ✅     | `!!str`                     | Default fallback                                                                                       |

**Remaining Core Schema compliance gaps**:

1. Implicit hex / 1.2-octal integers (`0xFF`, `0o17`) resolve as strings — tagged forms (`!!int 0xFF`) work.
2. Spec float forms `.inf` / `.nan` resolve as strings while Rust forms `inf` / `nan` are wrongly resolved as floats — tagged forms work.
3. The `BasicResolver` / `TagResolver` `implicit_resolvers` HashMap (`src/resolver.rs:36-48`) is still unused; the active resolution path is now `resolver::resolve_plain_scalar`, which is shared by all composers. The `BasicResolver` lookup map could be removed in a follow-up.

**Resolved during [#10](https://github.com/elioetibr/rust-yaml/issues/10)**:

- Composer code-path consolidation: all four composers now share `resolver::resolve_plain_scalar`, eliminating four near-identical inline resolution blocks.
- `%YAML 1.1` directive now governs bool resolution (`yes`/`no`/`on`/`off`).
- Scanner no longer rewrites `yes` → the literal string `"true"` — plain-scalar text reaches the composer unchanged.

## YAML 1.1 compatibility

YAML 1.1 includes types and behaviors that the 1.2 spec **explicitly removed** from the Core Schema. The table below shows where `rust-yaml` lands.

| 1.1 feature                                       | 1.2 status                          | rust-yaml behavior                                                                                                                                                                                 |
| ------------------------------------------------- | ----------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Boolean alternatives `yes/no/on/off`              | Dropped (1.2 = `true`/`false` only) | ✅ Directive-aware. Under default 1.2 (or `%YAML 1.2`), `yes`/`no`/`on`/`off` resolve as `String`. Under `%YAML 1.1`, they resolve as `Bool`.                                                      |
| Boolean short forms `y/n`                         | Dropped (1.2 = `true`/`false` only) | 🔵 Not recognized in either version.                                                                                                                                                               |
| Octal leading-zero `014` (= decimal 12 in 1.1)    | Dropped (1.2 = `0o14`)              | ❌ Resolved as decimal `Int(14)`, never as octal 12. Wrong for both 1.1 (should be 12) and arguably right for 1.2 (decimal interpretation matches `!!int`'s "implicit type implies decimal" rule). |
| `!!value` tag and `=` value-key replacement       | Dropped from 1.2 Core               | ✅ Default 1.2: `=` is a plain string (spec-correct). Under `%YAML 1.1`: detected as `tag:yaml.org,2002:value` and rejected with a construction error — closes the `ruamel` parity gap from [#1](https://github.com/elioetibr/rust-yaml/issues/1). |
| `!!merge` (`<<`)                                  | Retained de facto                   | ✅ `tests/merge_keys.rs`, `tests/merge_keys_comprehensive.rs`                                                                                                                                      |
| `!!binary`                                        | Retained, base64                    | ✅ `src/tag.rs:291-318` (decodes to `String` or marker for non-UTF-8)                                                                                                                              |
| `!!timestamp`                                     | Retained, ISO 8601                  | 🟡 Stub — `src/tag.rs:321-325` stores as `String("timestamp:<raw>")`                                                                                                                               |
| `!!omap`, `!!pairs`, `!!set`                      | Retained in 1.2 type repository     | ⏸ Recognized as tag names but mapped to default `Mapping`                                                                                                                                          |
| `%YAML 1.1` directive enables 1.1 bool resolution | —                                   | ✅ Directive flows through `Event::DocumentStart` → composer → `resolver::resolve_plain_scalar`. See `tests/directives.rs::test_yaml_version_1_1_compatibility`.                                   |
| `%YAML 1.1` directive enables `!!value` / `=`     | —                                   | ✅ Detected and rejected — see `src/constructor.rs::test_yaml_1_1_value_tag_rejected_with_directive` and `src/resolver.rs::plain_scalar_value_tag_1_1`. Mirrors `ruamel.yaml typ="safe"`. |

## Engine features beyond pure spec compliance

These are `rust-yaml`-specific and not part of any spec-conformance level:

| Feature                                        | Status | Notes                                                                                   |
| ---------------------------------------------- | ------ | --------------------------------------------------------------------------------------- |
| Comment preservation through round-trip        | ✅     | `tests/comment_preservation_tests.rs`, `LoaderType::RoundTrip`                          |
| Position info (line/column) on errors          | ✅     | `src/position.rs`                                                                       |
| Anchor / alias resolution                      | ✅     | `tests/integration_tests.rs`                                                            |
| Custom tag handlers (`register_handler`)       | ✅     | `src/tag.rs`                                                                            |
| JSON-Schema-style validation engine            | ✅     | `src/schema.rs` (note: this is the _validation_ schema, not the YAML resolution schema) |
| Streaming / event-based API                    | ✅     | `src/streaming_enhanced.rs`, `streaming_async.rs`                                       |
| Borrowed / zero-copy values                    | ✅     | `src/value_borrowed.rs`, `src/zero_copy_value.rs`                                       |
| Resource limits (max depth, max document size) | ✅     | `tests/security_limits.rs`                                                              |

## Known gaps tracked separately

- The Core Schema implicit `int` resolver only recognizes decimals — hex/octal/binary forms only typed when explicitly tagged.
- The Core Schema implicit `float` resolver uses Rust's `f64::parse` (accepts `inf`/`nan`), instead of the spec's `.inf`/`.nan` forms.
- `!!timestamp` construction is a stub.
- UTF-16/UTF-32 BOM detection at stream start.
- The `BasicResolver`'s `implicit_resolvers` HashMap is dead code now that `resolver::resolve_plain_scalar` is the single resolution path — candidate for removal in a follow-up.

## Maintaining this doc

When you add or change a spec-relevant feature:

1. Update the matching row above (move ❌/🟡 to ✅, or split a row).
2. Add a code/test reference so future maintainers can verify.
3. If a behavior diverges from 1.2.2 intentionally (e.g., for 1.1 interop), note it in the _YAML 1.1 compatibility_ table — don't quietly drop spec compliance.
