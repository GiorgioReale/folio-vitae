/** @type {import('@commitlint/types').UserConfig} */
export default {
  extends: ["@commitlint/config-conventional"],
  rules: {
    // Standard Conventional Commits types, plus `security` used in this repo.
    "type-enum": [
      2,
      "always",
      [
        "feat",
        "fix",
        "docs",
        "style",
        "refactor",
        "perf",
        "test",
        "build",
        "ci",
        "chore",
        "revert",
        "security",
      ],
    ],
    // Dependabot squash-merge bodies embed HTML release notes with long lines;
    // URLs also routinely exceed the conventional 100-char wrap.
    "body-max-line-length": [0],
    "footer-max-line-length": [0],
  },
};
