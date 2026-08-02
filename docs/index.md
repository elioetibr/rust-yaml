---
# This page carries NO inline SPDX header, unlike the other docs. Two rules
# collide on it and only an absent header satisfies both:
#   * VitePress recognises frontmatter only when it opens on line 1, so an HTML
#     comment above the `---` downgrades this page to ordinary markdown and the
#     home layout is lost. Keep the `---` first.
#   * `comply format` rewrites a header into a trailing HTML comment followed by
#     a blank line, which rumdl (MD012) and hk's end-of-file-fixer then strip --
#     the two fixers fight on every commit and never converge.
# REUSE.toml's `**` aggregate annotation already covers this file, so no inline
# header is required; `reuse lint` passes without one.
layout: home

hero:
  name: rust-yaml
  text: Safe YAML for Rust
  tagline: Zero unsafe by default, full YAML 1.2.2 spec compliance, high performance.
  image:
    src: /favicon.svg
    alt: rust-yaml
  actions:
    - theme: brand
      text: Migration guide
      link: /MIGRATION_GUIDE
    - theme: alt
      text: YAML 1.2.2 compliance
      link: /YAML_1.2.2_COMPLIANCE
    # rustdoc, outside the VitePress route map -- `target` makes the client
    # router hand this to a normal navigation. See the API nav entry in
    # .vitepress/config.mts for the mechanism.
    - theme: alt
      text: API docs
      link: /api/
      target: _self
    - theme: alt
      text: View on GitHub
      link: https://github.com/elioetibr/rust-yaml

features:
  - title: Safe by default
    details: No unsafe code in the default build - the workspace denies unsafe_code at the lint level rather than relying on review.
  - title: YAML 1.2.2
    details: Validated against the upstream yaml-test-suite conformance corpus, vendored as a submodule so results are reproducible.
  - title: Measured, not asserted
    details: Benchmarked against other Rust YAML engines with recorded baselines rather than claimed numbers.
---

<!--
SPDX-FileCopyrightText: rust-yaml contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->
