<!--
SPDX-FileCopyrightText: Rust Yaml contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Git Hooks for rust-yaml

Git hooks are managed by [hk](https://hk.jdx.dev), configured in `hk.pkl` at the
repository root. hk replaced the Python `pre-commit` framework; there is no
`.pre-commit-config.yaml` and no `.githooks/` directory.

## Overview

One step list, named `linters` in `hk.pkl`, backs every hook. Each step declares
a `glob`, and hk runs a step only when the staged file set matches it — so a
commit touching a single Markdown file runs the whitespace fixers, `rumdl`, and
the licence checks, but not clippy or `cargo deny`.

The `pre-commit` hook runs with `fix = true` and `stash = "git"`: fixable
problems are corrected and restaged in place, and unstaged work is stashed for
the duration so the hooks see exactly what you are committing.

## Quick Start

### Installation

```bash
mise run setup           # Full dev environment, including the hooks
mise run hooks:install   # Or just (re)install the hooks — idempotent
```

### Basic Usage

The hooks fire on their own once installed. To run them by hand:

```bash
mise run hooks:run       # Whole tree, the same set CI gates on
mise run hooks:pre-push  # The heavier pre-push gate
```

`mise run pre-commit` is an alias for `hooks:run`, and `mise run pre-push` for
`hooks:pre-push`.

To skip the hooks for a single commit — use sparingly, CI runs them anyway:

```bash
git commit --no-verify
```

## Hook Categories

### 1. General Code Quality and Standards

Applied to every staged file (`glob = "**"`):

- `check-added-large-files`, `check-merge-conflict`, `check-case-conflict`
- `check-symlinks`, `check-executables-have-shebangs`, `check-byte-order-marker`
- `trailing-whitespace-fixer`, `end-of-file-fixer`, `mixed-line-ending-fixer`

The last three are fixers: under `pre-commit` they rewrite and restage the file
rather than failing the commit.

### 2. Rust-Specific Quality Checks

- `fmt-check` and `clippy` on `*.rs`
- `cargo-sort` on `**/Cargo.toml`, `cargo-lock` on the manifests and `Cargo.lock`

`clippy` here is `mise run cargo:clippy`. The stricter CI gate,
`mise run cargo:clippy:strict`, is **not** part of the pre-commit set — run it
before pushing (see [Development](./DEVELOPMENT.md)).

### 3. Security and Vulnerability Management

- `gitleaks` scans every staged file for secrets (config: `.gitleaks.toml`)
- `audit` runs `cargo audit` when a manifest or lockfile changes

### 4. Dependency and License Management

- `deny` runs `cargo deny` when a manifest, lockfile or `deny.toml` changes
- `comply` / `comply-format` check REUSE/SPDX headers across the tree;
  `comply-fix` adds or refreshes the header on `*.rs`

### 5. Documentation and Standards

- `rumdl` on `*.md`
- `taplo` on `*.toml`, `yamllint-rs` on `*.{yml,yaml}`
- `actionlint` on `.github/workflows/*.{yml,yaml}`

### 6. Repository Consistency

Narrow globs that guard generated or duplicated state:

- `commit-config` — fires on `commit-types.toml`, `scripts/commit-config.py`,
  `committed.toml`, `cliff.toml` or `.gitmessage`, and fails if the generated
  files drift from the manifest
- `version-sync` — fires on `Cargo.toml`, `docs/package.json` or
  `docs/reference/cli.md`, and fails if any restated version disagrees with the
  `[workspace.package] version` anchor
- `gitignore` — fires on `ignorefile.toml` or `.gitignore`

### Commit messages

The `commit-msg` hook runs `mise run commit:lint`, which is
[committed](https://github.com/crate-ci/committed) reading `committed.toml`.
Commits must follow Conventional Commits.

`committed.toml`, `cliff.toml`'s parser block and `.gitmessage` are **generated**
from `commit-types.toml` by `scripts/commit-config.py`. Edit the manifest, then:

```bash
mise run commit:config        # Regenerate
mise run commit:config:check  # Fail on drift (what the hook runs)
```

## Configuration Files

| File | Purpose |
| --- | --- |
| `hk.pkl` | Step definitions, globs and hook wiring |
| `deny.toml` | `cargo deny` licence and advisory policy |
| `.gitleaks.toml` | Secret-scanning rules and allowlist |
| `REUSE.toml` | SPDX/REUSE licensing metadata |
| `committed.toml` | Commit-message rules (generated) |
| `commit-types.toml` | Source of truth for commit types |
| `taplo.toml`, `.rumdl.toml`, `.yamllint` | Formatter and linter settings |
| `.github/actionlint.yaml` | Workflow-linter settings |
| `ignorefile.toml` | Source of truth for `.gitignore` |

## Troubleshooting

### Hooks are not running

```bash
git config --get core.hooksPath
ls -la "$(git rev-parse --git-path hooks)"
mise run hooks:install
```

### A tool is missing

Every tool is pinned in `mise.toml`. Reinstall with:

```bash
mise install
mise doctor
```

### A commit is slow

hk only runs steps whose `glob` matches the staged files, so a slow commit
usually means a Rust or manifest change pulled in `clippy`, `cargo audit` or
`cargo deny`. Stage a narrower change set, or use `git commit --no-verify` and
let CI do the full pass.

### A fixer keeps rewriting a file

`trailing-whitespace-fixer`, `end-of-file-fixer` and `mixed-line-ending-fixer`
restage their own output under `pre-commit`. If an editor writes the file back
during the commit, the two fight — save and close the file, then retry.

### Generated files fail the hook

`commit-config` and `gitignore` compare on-disk output against the manifest.
Never hand-edit the generated side; regenerate instead:

```bash
mise run commit:config
mise run gitignore
```

## Integration with CI/CD

The GitHub Actions pipeline runs the same step list, so a clean
`mise run hooks:run` locally is a good predictor of a green CI lint job. CI adds
checks the hooks deliberately leave out — `cargo:clippy:strict`, the full test
matrix, and coverage.
