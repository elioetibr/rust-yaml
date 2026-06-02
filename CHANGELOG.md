# Changelog

All notable changes to this project will be documented in this file.

## [1.1.0] - 2026-06-02

### 🚀 Features

- **serde:** Impl Serialize for Value
- **serde:** Impl Deserialize for Value via Visitor
- **serde:** ValueSerializer with all serde::Serializer methods
- **serde:** To_string and to_writer entry points
- **serde:** ValueDeserializer primitives + from_str/from_slice/from_reader
- **serde:** EnumAccess for externally tagged enum variants
- **serde:** Re-export from_str/to_string family at crate root

### ⚡ Performance

- **serde:** From_str microbench vs serde_yaml
- **serde:** To_string microbench vs serde_yaml

### 📚 Documentation

- **serde:** Note enum-format divergence + mark #21 done
- Add flamegraph profiling guide and from_str report

### 🧪 Testing

- **serde:** Proptest round-trip for arbitrary Value
- **serde:** Parity corpus against serde_yaml (>=30 cases)

### 📦 Build

- Adopt workspace.package for shared crate metadata
- Add profiling profile and flamegraph aliases

### 👷 CI/CD

- **build:** Update the build logic

## [1.0.6] - 2026-05-22

### 🚀 Features

- **serde:** Scaffold serde_integration module structure
- **serde:** Map serde Error traits to crate::Error::Construction

## [1.0.5] - 2026-05-22

### 📚 Documentation

- **serde:** Add design doc for #21 serde integration

## [1.0.4] - 2026-05-22

### 🐛 Bug Fixes

- **scanner:** Emit BlockMappingStart for numeric mapping keys
- **resolver:** Resolve hex/octal/binary ints and dotted inf/nan

### 📚 Documentation

- **roadmap:** Mark #66 and #22 done, note #34 moved to v2.0.0

## [1.0.3] - 2026-05-22

### 📚 Documentation

- **roadmap:** Sync ROADMAP with v1.1.0 milestone status

## [1.0.2] - 2026-05-22

### 🐛 Bug Fixes

- **scanner:** Prevent peek_char isize offset overflow on isize::MIN
- **scanner:** Cap anchor and alias name length during scanning
- **streaming:** Propagate UTF-8 errors and stop splitting chars in MmapYamlReader

### ⚡ Performance

- **scanner:** Drop char_indices side table and O(n) directive resets
- **error:** Build error context from a line window instead of all lines

### 🧪 Testing

- **fuzz:** Add cargo-fuzz harness for load_str, strict limits, round-trip

### 📦 Build

- **deps:** Refresh Cargo.lock to latest 1.85-compatible versions

### 👷 CI/CD

- **fuzz:** Install cargo-fuzz from a prebuilt binary
- **fuzz:** Pin the host GNU triple for cargo-fuzz builds

## [1.0.1] - 2026-05-20

### 🐛 Bug Fixes

- **composer:** Cap cumulative alias materialization and bound traversal by @elioseverojunior
- **parser:** Convert three state-stack underflows from panic to error by @elioseverojunior
- **scanner:** Replace indent_stack panics with safe fallback (#18) by @elioseverojunior
- **scanner:** Propagate scan errors from new_eager_with_comments (#19) by @elioseverojunior
- **test:** Silence clippy::result-large-err in panic_resistance closure by @elioseverojunior

### 🧪 Testing

- **composer:** Cover iterative helpers and alias materialization cap by @elioseverojunior in [#64](https://github.com/elioetibr/rust-yaml/pull/64)

## [1.0.0] - 2026-05-19

### 🚀 Features

- **resolver:** Honor %YAML 1.1 directive in plain-scalar resolution by @elioseverojunior
- **scanner:** Implement \x## / \u#### / \U######## double-quoted escapes by @elioseverojunior
- **parser:** Reject aliases pointing at undefined anchors by @elioseverojunior
- **scanner:** Implement YAML 1.2 §8.1.1.2 block-scalar chomping by @elioseverojunior
- **parser:** Reject unclosed flow collections at end of stream by @elioseverojunior
- **parser:** Implicit single-pair flow mapping inside flow sequence by @elioseverojunior
- **1.1:** Detect bare `=` as the !!value tag under %YAML 1.1 by @elioseverojunior

### 🐛 Bug Fixes

- Handle empty arrays/structs and block-sequence parsing by @sebt3 in [#5](https://github.com/elioetibr/rust-yaml/pull/5)
- **scanner:** Stop infinite loop when `---` precedes non-space content by @elioseverojunior
- **scanner:** Ignore unknown directives instead of erroring by @elioseverojunior
- **tag:** Allow percent-encoded chars in suffix + URI-decode on resolve by @elioseverojunior
- **scanner:** Treat `?`, `:`, `-` as plain-scalar starts when not indicators by @elioseverojunior
- **scanner:** Accept full unicode in anchor / alias names by @elioseverojunior
- **parser:** Accept complex-key marker after explicit document start by @elioseverojunior
- **parser:** Always emit DocumentEnd before StreamEnd when a doc is open by @elioseverojunior
- **scanner:** Fold multi-line plain scalars per YAML 1.2 §6.5 / §7.3.3 by @elioseverojunior
- **scanner:** Enforce YAML 1.2 double-quoted escape allowlist by @elioseverojunior
- **scanner:** Accept literal `\<tab>` as a tab character in double-quoted by @elioseverojunior
- **parser:** Reject a second anchor on the same node by @elioseverojunior
- **parser:** Reject a second tag on the same node by @elioseverojunior
- **scanner:** Error on content after `...` document-end marker by @elioseverojunior
- **scanner:** Fold line breaks in quoted scalars per YAML 1.2 §7.3.2 / §7.3.3 by @elioseverojunior
- **scanner:** Single-quote `''` escape + double-quote `\<NL>` whitespace strip by @elioseverojunior
- **scanner:** Accept `:` adjacent to value in flow context by @elioseverojunior
- **scanner:** Reject document markers inside flow collections by @elioseverojunior
- **scanner:** Reject trailing content after a quoted scalar's closing quote by @elioseverojunior
- **parser:** Surface eager-parse errors + reject duplicate %YAML directive by @elioseverojunior
- **parser:** Reject anchor / tag attached to an alias node by @elioseverojunior
- **parser:** Reject %YAML / %TAG directives outside the directive context by @elioseverojunior
- **parser:** Transition to ImplicitDocumentStart after DocumentEnd by @elioseverojunior
- **scanner:** Reject `#` adjacent to quoted scalar without preceding space by @elioseverojunior
- **scanner:** Stop plain scalar at `:` followed by flow indicator in flow by @elioseverojunior
- **scanner:** Error on unclosed quoted strings by @elioseverojunior
- **scanner:** Comment `#` must be preceded by whitespace by @elioseverojunior
- **parser:** Reject directive without a document body at end-of-stream by @elioseverojunior
- **scanner:** Reject extra content after %YAML directive by @elioseverojunior
- **parser:** Emit implicit empty scalar for empty `---` document by @elioseverojunior
- **parser:** Emit implicit empty scalar between back-to-back `---` markers by @elioseverojunior
- **parser:** Emit implicit empty scalar for `---\n...` empty document by @elioseverojunior
- **parser:** Relax over-strict double-tag check (same as anchor case) by @elioseverojunior
- **scanner:** Reject multi-line quoted scalars used as implicit keys by @elioseverojunior
- **scanner:** Reject multi-line plain scalars used as implicit keys by @elioseverojunior
- **parser:** Close open collections before `---` starts a new document by @elioseverojunior
- **parser:** Close open collections before final DocumentEnd at EOS by @elioseverojunior
- **parser:** Close open collections before explicit `...` DocumentEnd by @elioseverojunior
- **parser:** Emit implicit empty value when next scalar is a new key by @elioseverojunior
- **parser:** Synthesize implicit empty value when closing odd-child mapping by @elioseverojunior
- **scanner:** Preserve breaks adjacent to more-indented folded lines by @elioseverojunior
- **scanner:** Scan past inner `:` when detecting implicit mapping keys by @elioseverojunior
- **parser:** Synthesize empty value for unmatched key before next key by @elioseverojunior
- **parser:** Synthesize implicit empty key for leading-colon mapping by @elioseverojunior
- **parser:** Keep anchor on key when BlockMappingStart wraps implicit key by @elioseverojunior
- **scanner:** Preserve literal whitespace on blank lines in block scalars by @elioseverojunior
- **parser:** Line-aware heuristic for "scalar is value vs new key" by @elioseverojunior
- **scanner:** Reject `]` and `}` outside their flow context by @elioseverojunior
- **scanner:** Reject block-scalar indent indicator `0` by @elioseverojunior
- **parser:** Reject leading or double comma in flow collections by @elioseverojunior
- **scanner:** Block scalar content_indent is leading-space count by @elioseverojunior
- **scanner:** Drop strict multiple-of-N indentation rule by @elioseverojunior
- **parser:** Pass pending anchor/tag to synthesised empty mapping keys by @elioseverojunior
- **scanner:** Treat quoted scalars at line head as potential keys by @elioseverojunior
- **scanner:** Trim trailing whitespace before plain-scalar fold by @elioseverojunior
- **parser:** Reject implicit mapping key without `:` at end of stream by @elioseverojunior
- **scanner:** Re-run indent handling after a block scalar by @elioseverojunior
- **parser:** Synth empty value at BlockEnd when innermost map is odd by @elioseverojunior
- **parser:** Synth empty value for key-only flow mapping entries by @elioseverojunior
- **scanner:** Fold multi-line plain scalars in flow context by @elioseverojunior
- **scanner:** Block-scalar base_indent reads from indent_stack by @elioseverojunior
- **scanner:** Parent-aware multi-line plain scalar continuation by @elioseverojunior
- **parser:** Reject implicit-key-without-colon at BlockEnd too by @elioseverojunior
- **parser:** Reject second root-level node in same document by @elioseverojunior
- **parser:** Scope second-root check to truly-closed root contexts by @elioseverojunior
- **parser:** Also catch second root via BlockMappingStart/BlockSequenceStart by @elioseverojunior
- **parser:** Leading `:` in flow sequence opens implicit empty-key pair by @elioseverojunior
- **parser:** Keep anchor on key of implicit single-pair flow mapping by @elioseverojunior
- **scanner:** Don't strip escape-produced whitespace at quoted fold by @elioseverojunior
- **scanner:** Support zero-indented block scalars by @elioseverojunior
- **parser:** `...` is a no-op when no document is open by @elioseverojunior
- **scanner:** Trailing whitespace-only line counts as block-scalar content by @elioseverojunior
- **scanner:** Detect sibling-outside-scope when auto-detecting block-scalar indent by @elioseverojunior
- **scanner:** Clip-mode chomping leaves empty scalar empty by @elioseverojunior
- **scanner:** Stricter is_pure_number — `20:03:20` isn't a number by @elioseverojunior
- **parser:** Flow-mapping `:` after empty key + comma synth in FlowMappingValue by @elioseverojunior
- **parser:** Flush pending anchor/tag as empty scalar at BlockSequence close by @elioseverojunior
- **parser:** Flush pending anchor/tag at BlockMapping close too by @elioseverojunior
- **parser:** Explicit `?` with no key at flow-mapping close by @elioseverojunior
- **scanner:** Folded block scalar — tab-indented content is "more indented" by @elioseverojunior
- **scanner:** Block-scalar header doesn't swallow next line's leading whitespace by @elioseverojunior
- **scanner:** Is_pure_number must see rest-of-line as whitespace/comment by @elioseverojunior
- **parser:** "new key" skip needs same-line co-occurrence, not just empty by @elioseverojunior
- **parser:** Preserve FlowMappingKey/Value state across flow opens by @elioseverojunior
- **scanner:** Line-start anchor opens block mapping at its column by @elioseverojunior
- **scanner:** Line-start alias/tag also opens block mapping by @elioseverojunior
- **scanner:** Drop strict multiple-of-N indent rule by @elioseverojunior
- **parser:** Synth empty scalar for empty block entries by @elioseverojunior
- **parser:** Reject double-comma + surface eager parse errors by @elioseverojunior
- **parser:** Freestanding anchor attaches to collection by @elioseverojunior
- **parser:** Explicit ? in block sequence opens nested mapping by @elioseverojunior
- **scanner+parser:** Nested block sequence across lines by @elioseverojunior
- **parser:** Bare anchor/tag at EOS synth tagged empty scalar doc by @elioseverojunior
- **scanner:** Flow collection as implicit block-mapping key by @elioseverojunior
- **scanner:** Reject doc marker inside quoted scalar by @elioseverojunior
- **parser:** Value after BlockEntry opens empty-key item mapping by @elioseverojunior
- **parser:** Freestanding anchor on block-seq item synth empty scalar by @elioseverojunior
- **scanner+parser:** Tag terminator in flow context + empty-value tag by @elioseverojunior
- **parser:** ? in BlockMappingValue opens nested value mapping by @elioseverojunior
- **scanner:** ? at line-start opens implicit block mapping by @elioseverojunior
- **scanner:** Compact sequence at dash column, continuation-aware by @elioseverojunior
- **parser:** Reject missing comma between flow collection entries by @elioseverojunior
- **scanner:** Block scalar header comment requires whitespace by @elioseverojunior
- **parser:** Implicit flow key must be on single line by @elioseverojunior
- **scanner:** Reject block-entry immediately after flow close on same line by @elioseverojunior
- **scanner:** Reject plain scalar immediately after flow close on same line by @elioseverojunior
- **scanner:** Reject indent landing between known levels after dedent by @elioseverojunior
- **parser:** Reject scalar in BlockSequence without preceding dash by @elioseverojunior
- **parser+tag:** %TAG directive scoped per document; reject unknown handles by @elioseverojunior
- **scanner:** Reject pure-tab indentation (with flow-opener carve-out) by @elioseverojunior
- **scanner:** Enforce flow-content indent inside enclosing block by @elioseverojunior
- **scanner:** Reject `,` outside flow context + reject in tag suffix by @elioseverojunior
- **scanner:** Flow-indent check exempts trailing closer line by @elioseverojunior
- **scanner:** Reject tab after `?` block-key marker by @elioseverojunior
- **parser:** Missing-value synth at BlockEnd consumes pending property by @elioseverojunior
- **scanner:** Multi-line flow as implicit key (with explicit-key carve-out) by @elioseverojunior
- **scanner:** Tab as block-scalar indent (carve-out for content tabs) by @elioseverojunior
- **parser:** Track pending_tag_line for freestanding-tag handling by @elioseverojunior
- **scanner:** Reject block-entry after property on same line by @elioseverojunior
- **scanner:** Reject tab between two block-entries on same line by @elioseverojunior
- **scanner:** Flow-indent check also considers compact sequences by @elioseverojunior
- **scanner:** Reject tab after line-start `:` separator by @elioseverojunior
- **scanner:** Multi-line quoted scalar in block context requires deeper indent by @elioseverojunior
- **scanner:** Tag at parent indent in value-expectation context by @elioseverojunior
- **scanner:** Anchor at parent indent in value-expectation context by @elioseverojunior
- **scanner:** Reject deeper indent when parent has complete key-value by @elioseverojunior
- **scanner:** Leading blank-line indent in block scalar must not exceed content indent by @elioseverojunior
- **parser:** Reject multiple `:` on same line in block mapping by @elioseverojunior
- **parser:** Multi-colon check carve-out for explicit-value-as-inline-mapping by @elioseverojunior
- **scanner:** Tab as quoted-scalar continuation indent in nested block by @elioseverojunior
- **parser:** Synth implicit empty value after pending-property key flush by @elioseverojunior
- **parser:** Reject anchor on `---` line before implicit mapping by @elioseverojunior
- **scanner:** Reject block-seq value on same line as implicit key by @elioseverojunior
- **scanner:** Allow tab as content-area whitespace in value continuation by @elioseverojunior
- **scanner:** Alias name can contain colon (skip implicit-key check) by @elioseverojunior
- **parser:** Closed flow collection as key in flow sequence by @elioseverojunior
- **parser:** Inline-wrapped explicit key with empty value (M2N8 cluster) by @elioseverojunior
- **parser:** BlockEnd carve-out for collection-typed explicit key by @elioseverojunior
- **parser:** Close inline-wrapped key on explicit-value separator by @elioseverojunior
- **parser:** V9D5 — value-side inline mapping wrap via inline_wrap_column by @elioseverojunior
- **tests:** Drop `ref` in implicit-borrow patterns (flow_indent_bug) by @elioseverojunior
- **quality:** Treat warnings as errors — full make check / check-all / ci pass by @elioseverojunior
- **deps:** Patch Dependabot alerts in npm + cargo lockfiles by @elioseverojunior
- **scanner:** Terminate tag suffix on `:` followed by whitespace by @elioseverojunior
- **emitter:** Place anchor on its own line for shared block sequences by @elioseverojunior

### 🔧 Refactoring

- **tests:** Extract yaml-test-suite as workspace crate by @elioseverojunior

### 📚 Documentation

- Add YAML 1.2.2 compliance reference by @elioseverojunior
- Add YAML conformance roadmap by @elioseverojunior
- Refresh YAML conformance roadmap after session 2 fixes by @elioseverojunior
- Refresh YAML conformance roadmap after session 3 fixes by @elioseverojunior
- **roadmap:** Add 0dbee5e to session 3 commit list by @elioseverojunior
- **roadmap:** Refresh after session 4 — past the 50% mile-marker (373/735) by @elioseverojunior
- Update roadmap to 424/735 after chomping fix by @elioseverojunior
- Roadmap to 441/735 (60.0%) by @elioseverojunior
- Roadmap to 471/735 (64.1%) by @elioseverojunior
- Roadmap to 483/735 (65.7%) by @elioseverojunior
- Roadmap to 534/735 (72.7%) by @elioseverojunior
- Roadmap to 582/735 (79.2%) by @elioseverojunior
- Roadmap to 596/735 (81.1%) by @elioseverojunior
- Roadmap to 648/735 (88.2%) by @elioseverojunior
- Roadmap to 660/735 (89.8%) by @elioseverojunior
- Roadmap to 685/735 (93.2%) by @elioseverojunior
- Roadmap to 687/735 (93.5%) by @elioseverojunior
- Roadmap to 694/735 (94.4%) by @elioseverojunior
- Roadmap to 699/735 (95.1%) by @elioseverojunior
- Roadmap to 707/735 (96.2%) by @elioseverojunior
- Roadmap to 717/735 (97.6%) by @elioseverojunior
- Roadmap to 720/735 (98.0%) by @elioseverojunior
- Roadmap milestone — 735/735 (100.0%) ✅ by @elioseverojunior
- **1.0.0:** Gitversion + docd + reflect 100% yaml-test-suite conformance by @elioseverojunior
- Reflect 190 lib tests + !!value tag closure in roadmaps by @elioseverojunior

### 🎨 Styling

- **benches:** Rustfmt sweep — normalize import ordering by @elioseverojunior
- **ci:** Normalize YAML quote style via Prettier by @elioseverojunior
- **docs:** Prettier-normalize all markdown — emphasis style + trailing-WS cleanup by @elioseverojunior
- **parser:** Collapse two `match`-with-`if` arms into match guards by @elioseverojunior
- Clean trailing whitespace and apply rustfmt across tree by @elioseverojunior

### 🧪 Testing

- **directives:** Strengthen YAML 1.1 directive coverage by @elioseverojunior
- **yaml-test-suite:** Split into SOLID modules with 100% TDD coverage by @elioseverojunior
- **yaml-test-suite:** Add per-test timeout + categorized failure report by @elioseverojunior
- **directives:** Align with new `=` rejection + widen make ci scope by @elioseverojunior

### 📦 Build

- Treat markdown and rustdoc warnings as errors by @elioseverojunior

### 👷 CI/CD

- Fix lint, format, and security audit failures on main by @elioseverojunior in [#7](https://github.com/elioetibr/rust-yaml/pull/7)
- Bump GitHub Actions to latest major version tags by @elioseverojunior
- Pin branch-only actions to immutable patch tags by @elioseverojunior in [#9](https://github.com/elioetibr/rust-yaml/pull/9)
- **commitlint:** Migrate to Rust-native `committed` by @elioseverojunior
- **workflows:** Standardize app-token id, harden quoting, cache apt pkgs by @elioseverojunior

### ⬆️ Dependencies

- **deps-dev:** Bump js-yaml from 4.1.0 to 4.1.1 by @dependabot[bot] in [#2](https://github.com/elioetibr/rust-yaml/pull/2)

### 🔨 Miscellaneous

- Bump Node to 24 by @elioseverojunior
- Bump Node to 24 and refresh docs.rs edition note by @elioseverojunior
- **tooling:** Add Prettier config + ignore list by @elioseverojunior
- **makefile:** Raise cold-cache timeouts for clippy-strict and test-security by @elioseverojunior

## [0.0.5] - 2025-08-20

### 🚀 Features

- Add comment preservation support in YAML processing by @elioseverojunior

## [0.0.4] - 2025-08-19

### 👷 CI/CD

- Update the release workflow by @elioseverojunior

### 🔨 Miscellaneous

- Set rust version as 1.85.0 by @elioseverojunior

## [0.0.3] - 2025-08-19

### 🐛 Bug Fixes

- Complex keys benches issue by @elioseverojunior

### 📚 Documentation

- Ducuments updates by @elioseverojunior

### 🔨 Miscellaneous

- Update Cargo.toml version and sorting file keys by @elioseverojunior

## [0.0.2] - 2025-08-19

### 🔨 Miscellaneous

- Repository url updates by @elioseverojunior

## [0.0.1] - 2025-08-19

### 🎉 Initialization

- Inital commit by @elioseverojunior

[1.1.0]: https://github.com/elioetibr/rust-yaml/compare/v1.0.6...v1.1.0
[1.0.6]: https://github.com/elioetibr/rust-yaml/compare/v1.0.5...v1.0.6
[1.0.5]: https://github.com/elioetibr/rust-yaml/compare/v1.0.4...v1.0.5
[1.0.4]: https://github.com/elioetibr/rust-yaml/compare/v1.0.3...v1.0.4
[1.0.3]: https://github.com/elioetibr/rust-yaml/compare/v1.0.2...v1.0.3
[1.0.2]: https://github.com/elioetibr/rust-yaml/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/elioetibr/rust-yaml/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/elioetibr/rust-yaml/compare/v0.0.5...v1.0.0
[0.0.5]: https://github.com/elioetibr/rust-yaml/compare/v0.0.4...v0.0.5
[0.0.4]: https://github.com/elioetibr/rust-yaml/compare/v0.0.3...v0.0.4
[0.0.3]: https://github.com/elioetibr/rust-yaml/compare/v0.0.2...v0.0.3
[0.0.2]: https://github.com/elioetibr/rust-yaml/compare/v0.0.1...v0.0.2
