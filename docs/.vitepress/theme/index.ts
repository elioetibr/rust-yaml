// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import type { Theme } from "vitepress";
import DefaultTheme from "vitepress/theme";
import { defineAsyncComponent } from "vue";

// The default theme, extended only to register <Mermaid>.
//
// `defineAsyncComponent` is the whole point of this file: registering the
// component eagerly would put mermaid on every page's critical path (see
// ../mermaid.ts). Async keeps it in its own chunk, fetched when a page that
// actually renders a diagram mounts. The markdown transform already wraps each
// diagram in <Suspense>, which is what handles the pending state.
export default {
  extends: DefaultTheme,
  enhanceApp({ app }): void {
    app.component(
      "Mermaid",
      defineAsyncComponent(() => import("./Mermaid.vue")),
    );
  },
} satisfies Theme;
