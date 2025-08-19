module.exports = {
  extends: ["@commitlint/config-conventional"],
  rules: {
    // Allow longer subject lines for detailed descriptions
    "subject-max-length": [2, "always", 72],

    // Allow these types
    "type-enum": [
      2,
      "always",
      [
        "init",
        "build",
        "ci",
        "chore",
        "docs",
        "feat",
        "fix",
        "perf",
        "refactor",
        "revert",
        "style",
        "test",
      ],
    ],

    // Allow these scopes (Rust/YAML specific)
    "scope-enum": [
      2,
      "always",
      [
        "init",
        "parser",
        "scanner",
        "emitter",
        "composer",
        "constructor",
        "lib",
        "cli",
        "docs",
        "tests",
        "benches",
        "examples",
        "ci",
        "deps",
        "release",
      ],
    ],

    // Make scope optional
    "scope-empty": [0],

    // Allow both lowercase and sentence case for subject
    "subject-case": [0],

    // Allow empty body
    "body-leading-blank": [1, "always"],
    "body-max-line-length": [2, "always", 100],

    // Allow empty footer
    "footer-leading-blank": [1, "always"],
    "footer-max-line-length": [2, "always", 100],

    // Custom rules for our project
    "header-max-length": [2, "always", 100],
    "header-min-length": [2, "always", 10],
  },

  // Custom parser options for handling special patterns
  parserPreset: {
    parserOpts: {
      // Handle references and special patterns correctly
      referencesIgnorePattern: /\[skip ci\]|\[ci skip\]|\+semver:/,
    },
  },

  // Help text
  helpUrl:
    "https://github.com/conventional-changelog/commitlint/#what-is-commitlint",

  // Default ignore patterns - allow merge commits, revert commits, etc.
  ignores: [
    (message) => message.includes("WIP"),
    (message) => message.startsWith("Merge "),
    (message) => message.startsWith("Revert "),
    (message) => /^chore\(release\): \d+\.\d+\.\d+/.test(message),
    (message) =>
      message.includes("[skip ci]") &&
      message.includes("[ci skip]") &&
      message.startsWith("chore:"),
    // Allow initial commit
    (message) => /^init: [Ii]nitial [Cc]ommit/.test(message),
  ],
};
