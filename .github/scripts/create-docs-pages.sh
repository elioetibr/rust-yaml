#!/usr/bin/env bash

# SPDX-FileCopyrightText: Rust Yaml contributors
#
# SPDX-License-Identifier: MIT OR Apache-2.0

# Assembles the GitHub Pages bundle from two independent generators:
#
#   VitePress (docs/)  -> site root   the hand-written guides and reference
#   rustdoc            -> /api/       the generated API documentation
#
# They are merged into one directory rather than deployed separately because a
# repository gets exactly one Pages deployment. Nesting rustdoc under /api/
# keeps every page it previously published reachable (only the prefix moves)
# while the VitePress home page takes the root.

set -Eeuo pipefail

# Every path below is relative to the repository root, so anchor there instead of
# trusting the caller's working directory.
cd "$(git rev-parse --show-toplevel)"

OUT="target/gh-pages"

echo "==> Building VitePress site"
# `--frozen-lockfile` is the point of running install here at all: it fails when
# bun.lock and package.json disagree, so a dependency edit that was never locked
# breaks the docs build instead of quietly resolving to something else in CI.
bun install --cwd docs --frozen-lockfile
bun run --cwd docs build

echo "==> Building rustdoc"
cargo doc --all-features --no-deps

echo "==> Assembling $OUT"
# Removed rather than overwritten: a page deleted from docs/ must disappear from
# the bundle, and cp alone would leave the previous run's copy in place. This
# matters locally; CI always starts from an empty target/.
rm -rf "$OUT"
mkdir -p "$OUT"

# `dist/.` (not `dist`) copies the directory *contents* into $OUT rather than
# nesting a `dist/` inside it.
cp -R docs/.vitepress/dist/. "$OUT/"
cp -R target/doc "$OUT/api"

# rustdoc's own root listing is unhelpful when the workspace has more than one
# member, so send /api/ straight to the library. Written unconditionally:
# whether cargo emits target/doc/index.html varies by version, and this must not
# depend on that.
cat > "$OUT/api/index.html" << 'EOF'
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>rust-yaml API documentation</title>
    <meta http-equiv="refresh" content="0; url=rust_yaml/index.html" />
    <link rel="canonical" href="rust_yaml/index.html" />
  </head>
  <body>
    <p>
      Redirecting to the
      <a href="rust_yaml/index.html">rust-yaml API documentation</a>.
    </p>
  </body>
</html>
EOF

# Without this, Pages runs the bundle through Jekyll, which drops every path with
# a leading underscore -- that silently guts rustdoc's asset directories.
touch "$OUT/.nojekyll"

echo "==> Pages bundle ready: $OUT"
printf '    root pages : %s\n' "$(find "$OUT" -maxdepth 1 -name '*.html' | wc -l | tr -d ' ')"
printf '    api/       : %s\n' "$(du -sh "$OUT/api" | cut -f1)"
