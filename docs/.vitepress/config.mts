// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { defineConfig } from "vitepress";

import { fileURLToPath } from "node:url";

import { mermaidMarkdown, mermaidVite } from "./mermaid";
import { rustdocDev } from "./rustdoc-dev";

// Derived from GITHUB_REPOSITORY ("owner/name" inside Actions) rather than
// hardcoded, so a build under a different owner -- a fork standing in as a
// staging deploy -- produces edit links, licence links and a base that point at
// the repository actually being built. Falls back to the canonical repo for
// local builds, where the variable is unset.
const SLUG = process.env.GITHUB_REPOSITORY ?? "elioetibr/rust-yaml";
const REPO = `https://github.com/${SLUG}`;
// `?? ` satisfies noUncheckedIndexedAccess; a slug without "/" is not a shape
// Actions produces, but the compiler cannot know that.
const REPO_NAME = SLUG.split("/")[1] ?? "rust-yaml";

// GitHub project pages serve under `/<repo>/`, so the base has to match or every
// asset 404s. A custom domain serves from the root instead, hence the override:
// `DOCS_BASE=/ bun run build` produces a build for root-domain hosting.
//
// The trailing slash is required, not cosmetic. VitePress asserts that `base`
// both starts and ends with `/`, and every `head` entry below interpolates it
// directly — without it, `${base}favicon.svg` resolves to the single path
// segment `/rust-yamlfavicon.svg`.
const base = process.env.DOCS_BASE ?? `/${REPO_NAME}/`;

// Pinned rather than left on Vite's 5173 default, which any other checkout on
// this machine also claims. `strictPort` makes a collision fail loudly instead
// of silently landing on 5174 and printing a URL nobody reads.
//
// DEV ONLY. `vitepress preview` does not run Vite's preview server -- it serves
// the built directory with Polka -- so a `vite.preview` block here is read by
// nothing and the command lands on its own 4173 default. Verified: with that
// block present, `bun run preview` still printed 4173. The preview port is
// therefore passed as a CLI flag from package.json, which honours DOCS_PORT the
// same way this does.
const port = Number(process.env.DOCS_PORT ?? 5273);

export default defineConfig({
  base,
  title: "rust-yaml",
  description:
    "A fast, safe YAML 1.2 library for Rust. Zero unsafe code in the default build, 735/735 yaml-test-suite conformance, streaming and zero-copy parsing.",
  lang: "en-GB",
  cleanUrls: true,
  lastUpdated: true,

  // Dead links FAIL the build. Keep it that way: the nav and sidebar below are
  // hand-maintained against the pages that actually exist, and this check is
  // what catches an entry added ahead of its page. VitePress only checks links
  // in markdown content, NOT in `themeConfig.nav`/`sidebar` — a bad entry there
  // renders a 404 at runtime and builds clean, so those still need care.
  //
  // A page that needs to reach a repo file outside this directory uses an
  // absolute REPO url. Relative `../README.md` reaches outside the srcDir and
  // is what this check exists to catch.
  ignoreDeadLinks: false,

  // ```mermaid fences become collapsible diagrams; see .vitepress/mermaid.ts for
  // why this is wired onto mermaid directly rather than through
  // vitepress-plugin-mermaid, and theme/Mermaid.vue for the component itself.
  // Without this, every fence renders as a plain highlighted code block.
  markdown: { config: mermaidMarkdown },

  vite: {
    // Spread first: `mermaidVite` carries only `build`, `optimizeDeps` and
    // `resolve`, so the port settings below cannot collide with it. Written this
    // way round so a future key added there does not silently outrank the ports.
    ...mermaidVite,
    server: { port, strictPort: true },
    // Dev only -- serves cargo's target/doc at `${base}api/` so the API nav
    // entry resolves here exactly as it does on the deployed site. The
    // production bundle still comes from create-docs-pages.sh; see
    // ./rustdoc-dev.ts for why this must not apply at build time.
    plugins: [
      rustdocDev(
        base,
        fileURLToPath(new URL("../../target/doc", import.meta.url)),
      ),
    ],
  },

  head: [
    // `base`-prefixed by hand: entries in `head` are emitted verbatim, so a
    // bare "/favicon.svg" 404s on project pages served under /rust-yaml/.
    [
      "link",
      { rel: "icon", type: "image/svg+xml", href: `${base}favicon.svg` },
    ],
    // Only the SVG is shipped. An `alternate icon` entry pointing at a
    // favicon.ico that does not exist in public/ is worse than no entry at all:
    // it turns one silent 404 into a request every browser makes on every page.
    // Every engine that reaches this site handles image/svg+xml.
    //
    // Neither entry silences the `GET /favicon.ico 404` in the dev console. That
    // is the browser probing the ORIGIN root, which ignores both `base` and
    // these tags -- under /rust-yaml/ nothing can answer it.
    ["meta", { name: "theme-color", content: "#1f5572" }],
    ["meta", { property: "og:type", content: "website" }],
    [
      "meta",
      {
        property: "og:title",
        content: "rust-yaml -- a fast, safe YAML 1.2 library for Rust",
      },
    ],
  ],

  themeConfig: {
    // Every entry below resolves to a page that exists in docs/. Add entries as
    // pages land, not before -- VitePress does not dead-link-check this block,
    // so an entry written ahead of its page builds clean and 404s in the browser.
    nav: [
      {
        text: "Guides",
        link: "/MIGRATION_GUIDE",
        activeMatch:
          "^/(MIGRATION_GUIDE|STREAMING|ZERO_COPY|MERGE_KEYS|DIRECTIVES|TAG_SYSTEM)",
      },
      {
        text: "Reference",
        link: "/YAML_1.2.2_COMPLIANCE",
        activeMatch:
          "^/(YAML_1\\.2\\.2_COMPLIANCE|COMPARISON|SERDE_INTEGRATION_DESIGN|YAML_TEST_SUITE|YAML_CONFORMANCE_ROADMAP)",
      },
      {
        text: "Performance",
        link: "/BENCHMARK_RESULTS",
        activeMatch:
          "^/(BENCHMARK_RESULTS|PERFORMANCE_OPTIMIZATIONS|PROFILING|PROFILING_REPORT)",
      },
      // rustdoc, not a VitePress route. `target` is load-bearing: the client
      // router intercepts same-origin clicks and would 404 on a path absent from
      // its route map, but it skips any anchor carrying a `target` attribute
      // (client/app/router.js checks `link.hasAttribute('target')`), so this
      // falls through to a normal navigation. On the deployed site /api/ comes
      // from create-docs-pages.sh; under `bun run dev` the rustdocDev plugin
      // above serves the same tree, so this entry resolves in both.
      { text: "API", link: "/api/", target: "_self" },
      {
        text: "Repository",
        items: [
          { text: "README", link: `${REPO}#readme` },
          { text: "Releases", link: `${REPO}/releases` },
          { text: "MIT licence", link: `${REPO}/blob/main/LICENSE-MIT` },
          {
            text: "Apache-2.0 licence",
            link: `${REPO}/blob/main/LICENSE-APACHE`,
          },
        ],
      },
    ],

    // Every markdown file in this directory appears exactly once below. Keep it
    // that way: a page absent from the sidebar is reachable only by search or by
    // guessing its URL, which is how the previous revision buried all 21 of
    // them behind entries for pages that never existed.
    sidebar: [
      {
        text: "Guides",
        items: [
          { text: "Migration guide", link: "/MIGRATION_GUIDE" },
          { text: "Streaming parser", link: "/STREAMING" },
          { text: "Zero-copy parsing", link: "/ZERO_COPY" },
          { text: "Merge keys", link: "/MERGE_KEYS" },
          { text: "Directives", link: "/DIRECTIVES" },
          { text: "Tag system", link: "/TAG_SYSTEM" },
        ],
      },
      {
        text: "Reference",
        items: [
          { text: "YAML 1.2.2 compliance", link: "/YAML_1.2.2_COMPLIANCE" },
          { text: "Library comparison", link: "/COMPARISON" },
          { text: "Serde integration", link: "/SERDE_INTEGRATION_DESIGN" },
          { text: "yaml-test-suite", link: "/YAML_TEST_SUITE" },
          { text: "Conformance roadmap", link: "/YAML_CONFORMANCE_ROADMAP" },
        ],
      },
      {
        text: "Performance",
        items: [
          { text: "Benchmark results", link: "/BENCHMARK_RESULTS" },
          { text: "Optimizations", link: "/PERFORMANCE_OPTIMIZATIONS" },
          { text: "Profiling guide", link: "/PROFILING" },
          { text: "Profiling report", link: "/PROFILING_REPORT" },
        ],
      },
      {
        text: "Project",
        items: [
          { text: "Roadmap", link: "/ROADMAP" },
          { text: "Development guide", link: "/DEVELOPMENT" },
          { text: "Git hooks", link: "/PRE_COMMIT" },
          { text: "Version management", link: "/VERSION_MANAGEMENT" },
          { text: "Codecov setup", link: "/CODECOV_SETUP" },
          { text: "docs.rs build fix", link: "/DOCS_RS_FIX" },
        ],
      },
    ],

    socialLinks: [{ icon: "github", link: REPO }],

    // Bundled at build time from the page content, so search needs no external
    // service and the site stays a set of static files.
    search: { provider: "local" },

    editLink: {
      pattern: `${REPO}/edit/main/docs/:path`,
      text: "Edit this page on GitHub",
    },

    footer: {
      // Two files, not one: the repository deliberately has no root `LICENSE`.
      // GitHub's `licensee` detector picks a single file and prefers a root
      // `LICENSE` over `LICENSE-*`, so an unmatchable combined file resolves the
      // repository to NOASSERTION instead of the dual licence.
      message: `Code released under <a href="${REPO}/blob/main/LICENSE-MIT">MIT</a> OR <a href="${REPO}/blob/main/LICENSE-APACHE">Apache-2.0</a>. Documentation under CC-BY-3.0+.`,
      copyright: "Copyright (c) rust-yaml contributors",
    },
  },
});
