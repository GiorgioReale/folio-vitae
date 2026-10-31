# Contributing Guide

Thank you for considering a contribution to this project! This document outlines how to work with the repository and propose changes effectively.

## Getting Started

- **Prerequisites:** Ensure you have Rust and Cargo installed. Use the Rust toolchain specified in `rust-toolchain.toml` if present.
- **Dependencies:** Run `cargo fetch` or `cargo build` to download dependencies.
- **Branches:** Base your work on the latest `main` branch unless directed otherwise.

## Development Workflow

1. **Fork and clone** the repository, then create a feature branch for your work.
2. **Install tools** required for formatting and linting (see below).
3. **Make your changes**, keeping commits focused and descriptive.
4. **Run tests and checks** before opening a pull request.
5. **Open a pull request (PR)** with a clear summary of the change and any relevant context or screenshots.

## Code Style and Formatting

- Use `cargo fmt` to format Rust code.
- Run `cargo clippy --all-targets --all-features` to catch common issues; address warnings or justify them in the PR.
- Keep functions focused and add documentation comments where helpful.

## Testing

- Run `cargo test` to ensure the test suite passes.
- If your change affects integration behavior or data, add or update tests accordingly.

## Commit Guidelines

Use [Conventional Commits](https://www.conventionalcommits.org/). CI rejects pull requests whose **title** or **commit messages** do not match.

Format:

```text
<type>(optional scope): <description>
```

Allowed types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`, `security`.

Examples:

- `feat: add SMTP contact form option`
- `fix(cv): correct PDF page margins`
- `docs: clarify Docker quick start`
- `chore(deps): bump axum`
- `security: ignore advisory with revisit date`

Write the description in the imperative mood (`add`, `fix`, `update`), keep commits focused, and group related changes when practical.

## Pull Requests

- Use a Conventional Commits title (same rules as commit messages). With squash merge, the PR title becomes the commit on `main`.
- Clearly describe the problem being solved and the approach taken.
- Reference related issues or discussions when applicable.
- Include any manual testing steps or screenshots if the change affects the UI.

## Reporting Issues

- When filing an issue, include:
  - Expected behavior and observed behavior
  - Steps to reproduce
  - Environment details (OS, Rust version)
  - Any logs or screenshots that help clarify the problem

## Community Expectations

- Be respectful and constructive in all interactions.
- Reviewers may request changes; please address feedback promptly.
- If you are unsure about anything, feel free to ask questions in the PR or issue thread.

Thank you for helping improve the project!
