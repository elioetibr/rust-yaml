# YAML Test Suite Integration

`rust-yaml` runs against the upstream
[`yaml/yaml-test-suite`](https://github.com/yaml/yaml-test-suite) corpus to
track real-world conformance with the YAML 1.2 specification.

## Layout

The conformance harness lives in its **own workspace crate** —
[`yaml-test-suite`](../yaml-test-suite) — modeled after the
`skald/skald-test-suite` layout:

```text
yaml-test-suite/
├── Cargo.toml             # publish = false, depends on rust-yaml via path
├── src/lib.rs             # data loader + event-tree converter + runner
├── tests/yaml_test_suite.rs   # integration test that drives the suite
└── data/                 # upstream yaml-test-suite submodule (pinned)
```

Keeping it as a separate workspace member means none of the corpus or harness
ships to crates.io, and the main `rust-yaml` package stays lean.

## Submodule

The corpus is registered at `yaml-test-suite/data/` and
pinned to the **`data-2022-01-17` release tag** — the latest stable data
release at the time of integration. The upstream `data` branch is squashed
and force-pushed, so we never track it directly; we pin to a tagged commit and
bump explicitly when new releases land.

### Initializing the submodule

```bash
git submodule update --init --recursive
```

### Bumping the pinned release

```bash
cd yaml-test-suite/data
git fetch --tags
git checkout data-YYYY-MM-DD    # the new release tag
cd ../..
git add yaml-test-suite/data
git commit -m "chore(tests): bump yaml-test-suite to data-YYYY-MM-DD"
```

## Running the harness

```bash
mise run test-yaml-suite

# or directly:
cargo test -p yaml-test-suite -- --nocapture
```

The harness runs by default — there is no `#[ignore]` gate — because skald's
pattern treats conformance as a first-class workspace test. The
`yaml-test-suite` crate is excluded from publication via `publish = false`,
so this only affects local/CI builds.

## How conformance is enforced

Each leaf test directory (one containing `in.yaml`) maps to a single test
case. The harness:

1. Reads `in.yaml`.
2. Determines the **expected** outcome — parse failure if an `error` file
   exists, parse success otherwise.
3. Runs `BasicParser::new_eager(input)` and converts the event stream into
   the upstream test-suite tree DSL (`+STR`, `+DOC`, `+MAP`, `=VAL :…`, etc.).
4. If `test.event` is present, compares actual vs expected event lines.
   If not, falls back to parse-success comparison.
5. Categorizes failures: _wrong accept_ (parser accepted invalid input),
   _wrong reject_ (parser failed on valid input), _wrong events_ (parsed
   but produced a different event tree).
6. Asserts a minimum pass-rate threshold — currently **10%** — so aggregate
   regressions fail CI. Raise the threshold in `tests/yaml_test_suite.rs` as
   pass-rate climbs.

## Test ID convention

Top-level test IDs are the upstream four-character keys, e.g. `229Q`. Tests
with multiple subtests follow the `VJP3/00`, `VJP3/01`, ... pattern.

## Suite file layout reference

Per upstream `ReadMe.md`:

| File         | Meaning                                                     |
| ------------ | ----------------------------------------------------------- |
| `===`        | Test name / label                                           |
| `in.yaml`    | Input YAML to parse                                         |
| `in.json`    | JSON value the input should decode to                       |
| `out.yaml`   | Canonical dumper output                                     |
| `emit.yaml`  | Emitter output                                              |
| `test.event` | Event-stream DSL output (used for tree comparison)          |
| `error`      | Marker file — when present, parsing the input **must** fail |

`in.json`, `out.yaml`, and `emit.yaml` comparisons remain follow-up work; the
harness today asserts on `test.event` plus the `error` marker.
