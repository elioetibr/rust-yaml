// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// The single ESLint config for this repository — there is no mirror or sync
// script.
//
// Flat config — required by ESLint v9+. Replaces the legacy `.eslintrc.js` +
// `.eslintignore` pair (both silently ignored in v9 and emit a hard warning
// in v10 per https://eslint.org/docs/latest/use/configure/migration-guide).
//
// Severity policy: every project-owned rule is 'error'. We don't emit 'warn'
// so `eslint --max-warnings 0` (wired into `bun run eslint:lint` and the hk
// `eslint` step) has no gray zone — a violation either fails the build or
// isn't a rule at all. `reportUnusedDisableDirectives: 'error'` catches stale
// `eslint-disable` comments that no longer suppress anything.
import js from "@eslint/js";
import tsPlugin from "@typescript-eslint/eslint-plugin";
import tsParser from "@typescript-eslint/parser";
import importPlugin from "eslint-plugin-import-x";
import globals from "globals";

export default [
  {
    // `golden-paths/**` holds template files with `{{placeholders}}` that are
    // not valid TypeScript by design — they're rendered into real .ts files
    // by the scaffolder. Skip them at the parser level so eslint doesn't
    // emit "Parsing error: Property assignment expected" on every run.
    ignores: [
      "dist/**",
      // `dist/**` does NOT match these: the glob is anchored at the config's
      // directory, and VitePress writes its bundle and cache one level down.
      // Without them eslint lints minified build output -- 4,731 of 4,733
      // errors on the first run came from `.vitepress/dist` and
      // `.vitepress/cache`, drowning the two real ones.
      ".vitepress/dist/**",
      ".vitepress/cache/**",
      "coverage/**",
      "node_modules/**",
      "**/*.d.ts",
      "golden-paths/**",
      // Gitignored scratch directory written by local tooling, not project
      // source. Linting it fails `bun run fix:all` on files we do not own.
      ".remember/**",
    ],
  },
  js.configs.recommended,
  {
    files: ["**/*.ts"],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: 2024,
        sourceType: "module",
      },
      globals: {
        ...globals.node,
      },
    },
    plugins: {
      "@typescript-eslint": tsPlugin,
      "import-x": importPlugin,
    },
    // Tell eslint-plugin-import-x how to resolve bare module specifiers,
    // TS-aliased paths (@ports/* etc. from tsconfig.json `paths`), and
    // the Node built-in protocol (`node:*`). Without this, `import-x/order`
    // misclassifies every alias as an unresolvable external.
    settings: {
      "import-x/parsers": {
        "@typescript-eslint/parser": [".ts", ".tsx"],
      },
      // Use import-x v4's built-in TypeScript resolver (via unrs-resolver)
      // instead of the legacy eslint-import-resolver-typescript package.
      "import-x/resolver": {
        typescript: true,
        node: {
          extensions: [".ts", ".tsx", ".js", ".mjs"],
        },
      },
    },
    linterOptions: {
      reportUnusedDisableDirectives: "error",
    },
    rules: {
      ...tsPlugin.configs.recommended.rules,
      "no-console": "off",
      // Disabled for TS per @typescript-eslint guidance — tsc's own
      // resolver catches undefined identifiers and understands TS-only
      // constructs (ambient types like BufferEncoding, type-only imports,
      // namespace merging). Leaving `no-undef` on produces false positives.
      "no-undef": "off",
      "no-unused-vars": "off",
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
      "@typescript-eslint/explicit-function-return-type": "error",
      "@typescript-eslint/no-explicit-any": "error",
      // Import organization (per https://medium.com/@python-javascript-php-html-css/
      // optimizing-typescript-imports-configuring-prettier-and-eslint-for-
      // multi-line-format-ec282b65d64e, adapted to this repo's @-alias
      // convention). `prettier-plugin-organize-imports` handles the mechanical
      // de-duping, removing-unused, and multi-line wrapping on save/format.
      // `import/order` enforces the GROUP ORDER: Node builtins first, then
      // npm externals, then @-aliased internal modules, then relative paths.
      // Blank lines between groups + alphabetical within each group keeps
      // diffs minimal and PR reviews scannable.
      "import-x/order": [
        "error",
        {
          groups: [
            "builtin",
            "external",
            "internal",
            "parent",
            ["sibling", "index"],
          ],
          "newlines-between": "always",
          alphabetize: { order: "asc", caseInsensitive: true },
          pathGroups: [
            // `bun:*` specifiers (bun:test, bun:sqlite, etc.) are
            // classified inconsistently by eslint-plugin-import-x's
            // resolver across environments — locally they are treated as
            // `external`, on CI's ARM64 Linux runner they are classified
            // as `builtin`. That disagreement made import-x/order
            // produce opposite "fixes" on the two machines. Pin them
            // explicitly to `external` so the group assignment is
            // deterministic regardless of the resolver's guess. Paired
            // with removing `builtin` from `pathGroupsExcludedImportTypes`
            // below so this rule applies even when the resolver calls
            // them builtin.
            { pattern: "bun:**", group: "external", position: "before" },
            // Workspace packages: every `@elioseverojunior/platform-*` package
            // (tags, ssm-paths, builder-core, aws-rds-postgres, crossguard)
            // is technically external — bun ships them through symlinked
            // node_modules — but semantically they're OUR code. Pin them
            // to the `internal` group so they sort *under* third-party
            // imports (npm/Pulumi) and *over* relative imports.
            {
              pattern: "@elioseverojunior/platform-*",
              group: "internal",
              position: "before",
            },
            // This package's own tsconfig `paths`. `@rust-yaml/*` is what
            // library source uses and what a consumer maps, so the same
            // specifier resolves on both sides; `@/*` is the short form and is
            // confined to tests, which are never consumed. Both are our code,
            // so they sort under third-party and over relative imports.
            {
              pattern: "@rust-yaml",
              group: "internal",
              position: "before",
            },
            {
              pattern: "@rust-yaml/**",
              group: "internal",
              position: "before",
            },
            { pattern: "@/**", group: "internal", position: "before" },
          ],
          // `type` still excluded (type-only imports skip pathGroups by
          // design). `builtin` removed so the `bun:**` rule above
          // applies even when the resolver classifies bun: modules as
          // builtin.
          pathGroupsExcludedImportTypes: ["type"],
        },
      ],
      "import-x/first": "error",
      "import-x/no-duplicates": "error",
    },
  },
];
