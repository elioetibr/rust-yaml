# Profiling Report — `rust_yaml::from_str` (serde load)

**Target:** `serde_load` bench, case `rust_yaml/large` (the `fixtures/config_large.yaml`
document deserialized into a derived `Config` struct via `rust_yaml::from_str`).
**Profile:** `profiling` profile (release opt-level + debuginfo), macOS `dtrace`
@ 997 Hz, ~12 s, **11,935 stack samples**, 884 unique stacks.
**Artifact:** `flamegraph.svg` (repo root — git-ignored; regenerate with
`cargo flame-serde-load`).

> Context: [PR #67](https://github.com/elioetibr/rust-yaml/pull/67) reported
> `from_str` ~1.5–1.8× slower than `serde_yaml` 0.9. This profile explains where
> that time goes.

## Headline

**The `from_str` path is allocation-bound, not compute-bound.** Over half of all
CPU self-time is spent in memory management, and the serde layer itself is nearly
free — the cost is building the intermediate `Value` tree.

| Subsystem | Self-time (leaf samples) |
| --- | ---: |
| **Allocator** (`malloc` / `free` / `nanov2_*`) | **45.2%** |
| Scanner (tokenizer) | 19.2% |
| Composer (Value tree) | 10.5% |
| Parser (events) | 8.0% |
| `memcpy` / `memmove` | 7.2% |
| serde visitor bridge | 4.6% |
| Constructor (safety rules) | 3.3% |
| Scalar resolution (`to_lowercase`, resolve) | 1.1% |
| clone / drop, runtime | 0.8% |

**Allocator + memcpy ≈ 52% of self-time.** The YAML pipeline proper (scan →
parse → compose → construct) is ≈ 42%. The serde `Deserializer` bridge that walks
the finished `Value` is only **4.6%** — so the divergence from `serde_yaml` is
*not* in the serde integration, it's in how many allocations the
scan/compose/construct pipeline makes to produce the `Value`.

## Top self-time (leaf) functions

| % | Function |
| ---: | --- |
| 18.4% | `_nanov2_free` |
| 16.6% | `nanov2_malloc_type` |
| 8.1% | `scanner::BasicScanner::process_line` |
| 6.3% | `_platform_memmove` |
| 4.8% | `composer::BasicComposer::new_eager_with_limits` |
| 4.5% | `scanner::BasicScanner::scan_plain_scalar` |
| 3.6% | `parser::BasicParser::process_token` |
| 3.4% | `composer::BasicComposer::compose_node` |
| 2.7% | `parser::BasicParser::get_event` |
| 2.2% | `scanner::BasicScanner::get_token` |
| 1.7% | `constructor::SafeConstructor::apply_safety_rules` |
| 1.6% | `constructor::SafeConstructor::validate_value` |
| 1.5% | `composer::BasicComposer::compose_scalar` |
| 1.1% | `alloc::str::<impl str>::to_lowercase` |

## Top inclusive (function + callees) frames

| % | Function |
| ---: | --- |
| 52.8% | `composer::BasicComposer::new_eager_with_limits` |
| 30.8% | `constructor::SafeConstructor::construct` |
| 25.7% | `scanner::BasicScanner::scan_all_tokens` |
| 24.6% | `scanner::BasicScanner::process_line` |
| 21.6% | `composer::BasicComposer::compose_mapping` |
| 18.0% | `composer::BasicComposer::compose_sequence` |
| 13.0% | `parser::BasicParser::process_token` |
| 12.2% | `scanner::BasicScanner::scan_plain_scalar` |
| 8.5% | `alloc::raw_vec::RawVecInner::finish_grow` (Vec growth) |

## Interpretation — where the allocations come from

The malloc/free dominance is not a single call site; it is spread across the
pipeline. The likely sources, each of which already has (or suggests) an issue:

1. **Per-scalar `String` allocation in the scanner.** `scan_plain_scalar` (4.5%
   self, 12.2% inclusive) and `process_line` (8.1% self) build owned strings for
   every plain scalar. Trimming/copying instead of borrowing drives both the
   allocator and `memmove` (7.2%). → directly addressed by **#31
   (`perf(scanner)`: trim plain-scalar in place instead of re-allocating)**.

2. **Scanner side tables.** `RawVecInner::finish_grow` at 8.5% inclusive points at
   `Vec` reallocations during tokenizing. → **#26 (`perf(scanner)`: drop the dead
   `char_indices` parallel `Vec`, ~50% memory cut)** targets exactly this.

3. **`to_lowercase` in scalar resolution (1.1% self).** Bool/null/`~` detection
   lowercases via an allocating `to_lowercase()`. This can be a zero-alloc
   `eq_ignore_ascii_case` comparison against the small known token set — a cheap
   self-contained win not yet ticketed.

4. **`SafeConstructor` (30.8% inclusive, 3.3% self).** `apply_safety_rules` +
   `validate_value` walk and in places re-own the constructed values. Worth
   checking for borrow-instead-of-clone opportunities on the validation path.

## Recommended next steps (in impact order)

1. **Land #31 and #26.** They attack the two biggest allocation sources
   (per-scalar strings + scanner side `Vec`s). Re-run `cargo flame-serde-load`
   after each and compare with `inferno-diff-folded`.
2. **Replace `to_lowercase()` with `eq_ignore_ascii_case`** in plain-scalar
   bool/null resolution — small, isolated, no API change.
3. **Audit `SafeConstructor` for clones** on the `validate_value` path.
4. Only after the above, revisit the serde bridge (4.6%) — it is not currently
   worth optimizing.

## Cross-platform comparison — aarch64 Linux (Raspberry Pi 5)

Re-profiled the identical bench on a Raspberry Pi 5 (Debian 13 trixie, kernel
6.18, aarch64, 4 cores) using `perf record -F 997 --call-graph dwarf` + `inferno`,
with `libc6-dbg` installed so glibc frames symbolize. 11,446 samples.
Artifact: `flamegraph.aarch64.svg`.

| Subsystem (self-time) | macOS arm64 (dtrace) | aarch64 Linux (perf) |
| --- | ---: | ---: |
| Allocator (malloc/free) | 45.2% | 32.1% |
| Scanner | 19.2% | 17.4% |
| Parser | 8.0% | 9.2% |
| Composer | 10.5% | 4.5% |
| Constructor (safety) | 3.3% | 3.1% |
| memcpy/memmove | 7.2% | 1.8% |
| **serde visitor bridge** | **4.6%** | **0.3%** |
| Generic glue (`Result::branch`, `ptr::write`, …) | *(inlined into callers)* | 29.2% |

**What's robust across both platforms (trust these):**

1. **`from_str` is allocation-bound.** The allocator is the single largest cost
   on both targets by a wide margin. The optimization thesis (cut allocations via
   #31 / #26) is platform-independent.
2. **The serde bridge is negligible** — 4.6% on macOS, 0.3% on Linux. Do not
   optimize it.
3. **Subsystem ordering is consistent:** scanner is the top non-allocator cost on
   both; constructor safety is small-but-real (~3%) on both.

**What differs (methodology, don't over-read the exact %):**

- **Unwinding granularity.** macOS used frame-pointer/dtrace unwinding, which
  folds inlined generic frames into their callers; Linux used DWARF call-graphs,
  which expose them as separate leaves — hence the 29.2% "generic glue" bucket
  (`Result::branch` from `?`, `core::ptr::write` moves) that has no macOS
  counterpart. Redistributing it would push the Linux scanner/parser/composer
  numbers up toward the macOS split. The composer gap (10.5% vs 4.5%) is mostly
  this effect.
- **Allocator implementation.** macOS `libmalloc` (`nanov2_*`) vs glibc
  `ptmalloc` with tcache (`tcache_free`, `_int_malloc`, `malloc_consolidate`).
  glibc's tcache fast-path is cheaper per call, which plausibly explains why the
  same allocation *count* costs relatively less wall-time on Linux (32% vs 45%).
  This is a real allocator-cost difference, not just noise.

**Takeaway:** the Pi 5 run corroborates the macOS conclusion rather than
contradicting it — allocation reduction is the right lever, and it will pay off
on both your dev machine and aarch64 deployment targets. It also gives a clean,
reproducible Linux/`perf` path that side-steps the macOS `xctrace` converter bug.

## Reproduce

```bash
cargo flame-serde-load          # writes flamegraph.svg (see docs/PROFILING.md)
# manual dtrace + inferno path used for this report:
#   sudo dtrace -x ustackframes=100 \
#     -n 'profile-997 /pid == $target/ { @[ustack()] = count(); }' \
#     -c 'target/profiling/deps/serde_load-<hash> --bench --profile-time 12 rust_yaml/large' \
#     -o serde_load.dtrace.stacks
#   inferno-collapse-dtrace serde_load.dtrace.stacks > serde_load.folded
#   inferno-flamegraph serde_load.folded > flamegraph.svg
```

> Note: on macOS, `cargo flamegraph`'s built-in `xctrace` → SVG converter
> (v0.6.12) fails to parse the XML emitted by the installed Xcode `xctrace`
> (`MismatchedEndTag`). The dtrace + `inferno` path above is the reliable
> fallback and is what produced the macOS numbers.

**aarch64 Linux (Raspberry Pi 5) — `perf` path (no xctrace bug):**

```bash
# one-time: rustup + perf + flamegraph + inferno + glibc symbols
sudo apt-get install -y linux-perf libc6-dbg
cargo install flamegraph inferno

cargo build --profile profiling --features serde --bench serde_load
BIN=$(ls target/profiling/deps/serde_load-* | grep -v '\.d$' | head -1)
sudo perf record -F 997 --call-graph dwarf -o serde.perf -- \
     "$BIN" --bench --profile-time 12 rust_yaml/large
sudo chown "$USER" serde.perf
perf script -i serde.perf | inferno-collapse-perf > serde.folded
inferno-flamegraph serde.folded > flamegraph.aarch64.svg
```
