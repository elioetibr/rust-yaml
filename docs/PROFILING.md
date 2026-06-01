# Profiling with Flamegraphs

This project ships a ready-to-run [flamegraph](https://github.com/flamegraph-rs/flamegraph)
setup for finding CPU hot spots in the scanner, parser, and serde data-format
paths. A flamegraph is a sampled view of where wall-clock time goes: box width
is the share of samples a function (plus its callees) was on the stack, and
the boxes stacked above a function are the things it called.

## Prerequisites

Install the `cargo flamegraph` subcommand once:

```bash
cargo install flamegraph
```

Platform profiler (used under the hood):

- **macOS** — `dtrace`, bundled with the OS. Requires `sudo`, which the
  aliases handle via `--root` (you will be prompted for your password). With
  System Integrity Protection enabled, dtrace works on binaries you build
  yourself (our benches) but not on system binaries — no extra setup needed here.
- **Linux** — `perf` (e.g. `apt install linux-tools-$(uname -r)`). If you hit
  `perf_event_paranoid` errors, either keep the `--root` the aliases pass, or
  relax it: `sudo sysctl kernel.perf_event_paranoid=1`.

## Quick start

Three aliases are defined in [`.cargo/config.toml`](../.cargo/config.toml):

```bash
cargo flame-serde-load   # from_str::<T>  hot path (serde feature)
cargo flame-serde-dump   # to_string      hot path (serde feature)
cargo flame-parsing      # core scanner/parser/composer (no serde)
```

Each produces `flamegraph.svg` in the repo root. **Open it in a browser** — it
is interactive: click any box to zoom into that subtree, and use the search box
to highlight a function across the graph. The artifact is git-ignored.

## How the aliases are built

Taking `flame-serde-load` as the example:

```text
cargo flamegraph --root --profile profiling --features serde \
      --bench serde_load -- --profile-time 12 rust_yaml/large
```

| Flag | Why |
| --- | --- |
| `--root` | dtrace/perf needs elevated privileges (sudo prompt on macOS). |
| `--profile profiling` | Builds **optimized code with debug symbols** (see `[profile.profiling]` in `Cargo.toml`), so stacks are both realistic and readable. |
| `--features serde` | The serde benches are gated behind `required-features = ["serde"]`. |
| `--bench serde_load` | Target the Criterion bench, not a unit test. |
| `-- --profile-time 12` | Puts Criterion in **profiler mode**: run the benchmark for ~12s with no statistical analysis or plotting — exactly what an external sampler wants. |
| `rust_yaml/large` | Regex filter so we profile **our** code path on the large fixture, not the `serde_yaml` comparison case. |

To profile a different case, run the underlying command directly and change the
filter, e.g. `rust_yaml/small` or drop the filter to capture everything:

```bash
cargo flamegraph --root --profile profiling --features serde \
      --bench serde_load -- --profile-time 12 rust_yaml/medium
```

## Reading the result

- **Width = cost.** A wide box was on the stack for a large share of samples.
  Width is *cumulative* (function + everything it called), so a wide box with a
  single wide child is just a pass-through — look at the **flat top edges**,
  which is where CPU time is actually spent.
- **The serde `from_str` 1.5–1.8x gap** reported in
  [PR #67](https://github.com/elioetibr/rust-yaml/pull/67) should show up as
  time split across `Scanner`/tokenizing, scalar resolution
  (`resolve_plain_scalar` and friends), and the serde `Visitor` bridge from
  `Value` into the derived struct. Use the graph to confirm which dominates
  before optimizing.

## Troubleshooting

- **Truncated / `[unknown]` stacks through inlined code.** LTO inlines
  aggressively. Opt in to frame pointers for one run (not committed, to keep
  CI/release builds untouched):

  ```bash
  RUSTFLAGS="-C force-frame-pointers=yes" cargo flame-serde-load
  ```

- **`dtrace: failed to initialize` / permission denied (macOS).** Make sure the
  command kept `--root`; rerun and enter your password at the prompt.
- **Empty or near-empty graph.** The benchmark ran too briefly to collect
  samples — increase `--profile-time` (e.g. `24`).
