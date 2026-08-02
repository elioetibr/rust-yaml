// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// Module shim for single-file components.
//
// `docs:typecheck` runs plain `tsc`, which has no notion of `.vue`. vue-tsc is
// what normally supplies these types, but it resolves TypeScript's internal
// layout and does not support the TS 7 package exports yet -- see tsconfig.json.
// This declaration is the minimum that lets an `import Mermaid from
// "./Mermaid.vue"` resolve; it does not type-check the component's own script.
declare module "*.vue" {
  import type { DefineComponent } from "vue";

  const component: DefineComponent<
    Record<string, unknown>,
    Record<string, unknown>,
    unknown
  >;
  export default component;
}
