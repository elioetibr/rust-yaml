// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import type { MarkdownRenderer } from "vitepress";

// Mermaid rendering, wired directly onto `mermaid` rather than through
// `vitepress-plugin-mermaid`.
//
// The plugin registers its component by rewriting VitePress's client entry to
// import mermaid eagerly. That pulls every diagram renderer it can lazily load --
// katex, sequence, gantt, C4, venn, and the rest -- into the entry's module graph,
// so VitePress emits `modulepreload` links for all of them on *every* page,
// including pages with no diagram at all. Registering the component
// asynchronously instead (see theme/index.ts) keeps mermaid in its own chunk.
//
// Its component also forces `theme: "dark"` whenever the page is dark, which
// overrides the themeVariables that make diagrams match the site palette. Between
// that and the eager import, the two pieces below -- a fence transform and a
// little Vite config -- are all that was worth borrowing, so they are spelled out
// here and the dependency is gone.

/**
 * Replaces ```mermaid fences with the <Mermaid> component.
 *
 * Passed straight to VitePress's `markdown.config`. The diagram source is
 * URL-encoded into an attribute: it is arbitrary multi-line text, and encoding
 * escapes the quotes and newlines that would otherwise break the template.
 * <Suspense> is what lets <Mermaid> be registered as an async component.
 */
export const mermaidMarkdown = (md: MarkdownRenderer): void => {
  const renderFence = md.renderer.rules.fence;
  if (!renderFence) return;

  md.renderer.rules.fence = (tokens, index, options, env, self): string => {
    const token = tokens[index];
    // markdown-it always passes a valid index, but `noUncheckedIndexedAccess`
    // types the lookup as possibly undefined. Deferring to the original renderer
    // is the correct answer either way: it is what handles every non-mermaid
    // fence below.
    if (!token || token.info.trim() !== "mermaid") {
      return renderFence(tokens, index, options, env, self);
    }
    const graph = encodeURIComponent(token.content);
    return `<Suspense><Mermaid id="mermaid-${index}" graph="${graph}" /></Suspense>`;
  };
};

/**
 * Vite configuration mermaid needs, merged into the site's `vite` block.
 *
 * The aliases exist because these dependencies ship both CommonJS and ESM builds
 * and mermaid reaches for the CommonJS paths. The production build survives that
 * -- Rollup interops it -- but the dev server serves native ESM and fails with
 * "does not provide an export named 'default'".
 *
 * The list is every `dayjs/plugin/*` mermaid imports. It is one longer than the
 * equivalent list in `vitepress-plugin-mermaid`, which omits `duration` and so
 * breaks any gantt chart under `mise run docs:dev`. This site has no gantt today,
 * but the omission is the kind that only surfaces when someone adds one.
 */
export const mermaidVite = {
  // Mermaid's own chunk is ~650 kB (its core plus the langium parser), which
  // trips rolldown's flat 500 kB warning. Raising the threshold rather than
  // splitting further, because the size is not on the critical path: Mermaid.vue
  // loads mermaid through a dynamic import, so only a page containing a diagram
  // fetches it, and mermaid already splits its own renderers into separate
  // chunks (cytoscape, katex, architectureDiagram, ...). Further manual chunking
  // cannot shrink a parser that has to arrive whole.
  //
  // The number is a threshold for a warning, not a budget: raise it only when the
  // chunk it silences is genuinely lazy, or it stops reporting real regressions.
  build: {
    chunkSizeWarningLimit: 700,
  },
  optimizeDeps: {
    include: [
      "@braintree/sanitize-url",
      "dayjs",
      "debug",
      "cytoscape-cose-bilkent",
      "cytoscape",
    ],
  },
  resolve: {
    alias: {
      "dayjs/plugin/advancedFormat.js": "dayjs/esm/plugin/advancedFormat",
      "dayjs/plugin/customParseFormat.js": "dayjs/esm/plugin/customParseFormat",
      "dayjs/plugin/duration.js": "dayjs/esm/plugin/duration",
      "dayjs/plugin/isoWeek.js": "dayjs/esm/plugin/isoWeek",
      "cytoscape/dist/cytoscape.umd.js": "cytoscape/dist/cytoscape.esm.js",
    },
  },
};
