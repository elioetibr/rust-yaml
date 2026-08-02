#!/usr/bin/env bash

# SPDX-FileCopyrightText: Rust Yaml contributors
#
# SPDX-License-Identifier: MIT OR Apache-2.0

# Builds rustdoc and prepares it for the GitHub Pages artifact.
#
# rustdoc ONLY. The VitePress site under docs/ is a separate concern with its
# own workflow; this script briefly assembled both into one bundle, which made
# two publishers compete for the single Pages deployment a repository gets.
# Keeping the two apart is what lets each be reasoned about on its own.

set -Eeuo pipefail

# Every path below is relative to the repository root, so anchor there instead of
# trusting the caller's working directory.
cd "$(git rev-parse --show-toplevel)"

OUT="target/doc"

echo "==> Building rustdoc"
cargo doc --all-features --no-deps

# rustdoc's own root listing is unhelpful when the workspace has more than one
# member, so send the root straight to the library. Written unconditionally:
# whether cargo emits target/doc/index.html varies by version, and this must not
# depend on that.
cat > "$OUT/index.html" << 'EOF'
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

# Without this, Pages runs the output through Jekyll, which drops every path with
# a leading underscore -- that silently guts rustdoc's asset directories.
touch "$OUT/.nojekyll"

echo "==> rustdoc ready: $OUT"
printf '    size: %s\n' "$(du -sh "$OUT" | cut -f1)"
