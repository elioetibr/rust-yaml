# Serde Integration Design

**Tracks:** issue #21 — `feat(serde): full Serialize + Deserialize integration`.
**Status:** approved 2026-05-22. Implementation pending.

## Overview

Add full `serde` integration to `rust-yaml`: implement the `Serializer` /
`Deserializer` traits so users can convert between YAML and arbitrary Rust
types via `from_str::<T>` / `to_string(&T)`, plus `Serialize` / `Deserialize`
for the public `Value` type. The public API mirrors `serde_yaml` so projects
migrating off the deprecated `serde_yaml` crate can swap the import.

## Goals

- Full serde data format: `from_str`, `from_slice`, `from_reader`,
  `to_string`, `to_writer`.
- `Serialize` and `Deserialize` impls for `Value`.
- Drop-in source-compatible with `serde_yaml`'s public API.
- All code feature-gated behind `#[cfg(feature = "serde")]`; default build
  unaffected.
- Round-trip property tests and a parity test against `serde_yaml`.
- Benchmark suite (`benches/serde_*.rs`) comparing against `serde_yaml`.
- `yaml/yaml-test-suite` conformance stays 735 / 735.

## Non-goals (this issue)

- Borrowing deserialization (`from_str<'a, T: Deserialize<'a>>`). Deferred —
  can be added later, additively.
- Derive macros and a larger comparative benchmark harness. Tracked in #39.
- Tagged `!!binary` round-trip for `serialize_bytes`. v1.1.0 emits bytes as
  `Sequence<Int>`; the tagged form is a follow-up.

## Architecture

Layered, with the existing `Value` tree as the pivot. The serde layer never
touches the scanner / parser / emitter directly — it only consumes the
public `Value` API. That is the property that keeps the serde code stable
even as the parser / emitter evolve.

```text
T: Serialize   ──►  Serializer (T → Value)  ──►  Value  ──►  dump_str  ──►  YAML
T: Deserialize ◄──  Deserializer (Value → T) ◄──  Value  ◄──  load_str  ◄──  YAML
```

## Module layout

The current 28-line `src/serde_integration.rs` stub becomes a directory.

```text
src/serde_integration/
  mod.rs    — public API + re-exports
  ser.rs    — Serializer impl (T → Value); to_string / to_writer entry points
  de.rs     — Deserializer impl (Value → T); from_str / from_slice / from_reader
  value.rs  — Serialize / Deserialize impls for Value
  error.rs  — serde::ser::Error + serde::de::Error impls on crate::Error
```

Each file stays under the project's ~300-line ceiling. `lib.rs` adds
top-level re-exports under `#[cfg(feature = "serde")]` so
`rust_yaml::from_str::<T>` is the drop-in for `serde_yaml::from_str::<T>`.

## Public API

```rust
#[cfg(feature = "serde")]
pub fn from_str<T: DeserializeOwned>(s: &str) -> Result<T>;
#[cfg(feature = "serde")]
pub fn from_slice<T: DeserializeOwned>(b: &[u8]) -> Result<T>;
#[cfg(feature = "serde")]
pub fn from_reader<R: Read, T: DeserializeOwned>(r: R) -> Result<T>;
#[cfg(feature = "serde")]
pub fn to_string<T: Serialize + ?Sized>(v: &T) -> Result<String>;
#[cfg(feature = "serde")]
pub fn to_writer<W: Write, T: Serialize + ?Sized>(w: W, v: &T) -> Result<()>;
```

Signatures mirror `serde_yaml`'s. `T: DeserializeOwned` only at first —
keeps scope tight; a borrowing variant can be added later additively.

## Type mapping (Value ↔ serde data model)

| Value variant | serde direction                                                                         |
|---------------|-----------------------------------------------------------------------------------------|
| `Null`        | `unit`, `Option::None`                                                                  |
| `Bool`        | `bool`                                                                                  |
| `Int(i64)`    | `i64`. Smaller integer widths fit via `i64`; `u64 > i64::MAX` is a serialization error. |
| `Float(f64)`  | `f64`                                                                                   |
| `String`      | `&str`, `String`, `char` (1-char string)                                                |
| `Sequence`    | seq, tuple, tuple_struct, tuple_variant                                                 |
| `Mapping`     | map, struct (field names as keys)                                                       |

Serde-specific cases:

- `newtype_struct` flattens to its inner value (no wrapping).
- `unit_struct` → `Null`.
- Enums are externally tagged by default — `{"VariantName": value}`.
- `serialize_bytes` emits `Sequence<Int>` for v1.1.0. Tagged `!!binary` is
  deferred (see Non-goals).

`IndexMap` preserves insertion order, which matches serde's expectation
for struct field order.

## Error handling

Reuse the existing `crate::Error` — no new variants (avoids the additive-
enum semver question). Implement serde's error traits via the existing
`Error::Construction` variant.

```rust
#[cfg(feature = "serde")]
impl serde::ser::Error for crate::Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::construction(Position::new(), msg.to_string())
    }
}
// equivalent impl for serde::de::Error
```

`Error::Construction` already carries a message and optional context, so it
fits serde's needs without growing the enum.

## Testing strategy

Three layers:

- **Unit** — in each file: per-type round-trip for primitives, `Option`,
  `Vec`, struct, tuple, enum variants, and nested structures.
- **Integration** (`tests/serde_integration.rs`):
  - **Round-trip property test** with `proptest` (already a dev-dep):
    generate arbitrary `Value`, dump, load, assert equal.
  - **`serde_yaml` parity** — ~30 hand-curated `(yaml, expected_struct)`
    cases. Assert `rust_yaml::from_str` matches `serde_yaml::from_str` on
    the resulting value, covering primitives / struct / enum / nested.
- **Conformance** — `make -B yaml-test-suite` stays 735 / 735. The serde
  layer does not touch the parser, but the run is verified.

## Benchmarks

Two files under `benches/`:

- `serde_load.rs` — `rust_yaml::from_str::<Config>(yaml)` vs.
  `serde_yaml::from_str::<Config>(yaml)` on small / medium / large config
  fixtures.
- `serde_dump.rs` — symmetric for the serialize direction.

Shared fixtures. #39 will extend with derive-macro workloads and larger
realistic inputs.

## Feature gating & dependencies

- All new code under `#[cfg(feature = "serde")]`. The `serde` feature
  already exists in `Cargo.toml` and is off by default.
- New **dev-dependency**: `serde_yaml = "0.9"` for parity tests and
  benchmark comparison. `serde_yaml` is archived but still functional and
  available on crates.io; dev-only usage for differential validation is
  acceptable. Fallback: replace with hand-curated expected values if a
  policy concern arises.
- Existing `serde` (with the `derive` feature) and `proptest` are reused.

## Definition of done

- All five public functions implemented and exported.
- `Serialize` and `Deserialize` impls for `Value`.
- Round-trip property test passes.
- Parity test against `serde_yaml` passes (≥ 30 cases).
- `benches/serde_load.rs` and `benches/serde_dump.rs` build and produce
  numbers.
- `mise run ci` ✓, `mise run check-all` ✓, conformance 735 / 735 ✓.
- All new code feature-gated; default build untouched.

## Open questions / future work

- Borrowing deserialization (`Deserialize<'a>`) — additive follow-up.
- Tagged `!!binary` round-trip for `serialize_bytes`.
- Streaming serde (`Deserializer` driven directly off the parser event
  stream, no `Value` allocation). Would land additively as
  `from_reader_streaming` if a real perf case arises.
- Optional `serde_yaml`-style `Mapping` new type if downstream code reaches
  for it directly. Deferred until requested.
- **Non-unit enum variant format divergence from `serde_yaml`.** `serde_yaml`
  0.9 emits and consumes tuple / struct variants as YAML-tagged values
  (`!Variant\n- 1\n- 2\n`); this crate uses the single-entry-mapping form
  (`Variant:\n  - 1\n  - 2\n`). Unit variants are compatible; non-unit
  variants are not bidirectionally portable. The mapping form is the
  natural fit for our `Value` data model and avoids a leaky abstraction
  through the existing tag system. A follow-up could accept both forms on
  input (and pick a default on output) if drop-in migration from
  `serde_yaml` for codebases with non-unit enum variants becomes a
  requirement.
- **`src/serde_integration/ser.rs` size (475 lines)** exceeds the project's
  ~300-line ceiling. The file is one coherent unit (`ValueSerializer` + 4
  builder structs that the `Serializer` trait associates types with);
  splitting requires re-exports and cross-module visibility. A mechanical
  split into `ser.rs` (the `Serializer` impl) + `ser_builders.rs` (the
  builder structs) is a reasonable cleanup whenever the area is touched
  next.
- **Load performance.** Initial microbenchmarks show `rust_yaml::from_str`
  is roughly 2× slower than `serde_yaml::from_str` on small/medium config
  inputs; dump performance is comparable. #39 (`feat(serde): derive macros
  - serde benchmarks`) is the right place to land profiling-driven
  improvements.
