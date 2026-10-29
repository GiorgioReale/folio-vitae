# Required status checks on `main`

Branch ruleset **Protect main** requires these checks before merge:

| Check name | Workflow | Meaning |
|------------|----------|---------|
| `Merge gate` | Rust CI | Format/clippy/tests/MSRV when Rust paths change |
| `Rust Security gate` | Rust Security | `cargo audit` / `cargo deny` when dependency paths change |
| `Docker gate` | Docker | Multi-arch image **build** when Docker-related paths change |
| `Trivy gate` | Docker | Image vulnerability scan on PR builds (separate from build failures) |

Gates always run on PRs to `main`. If the underlying jobs are skipped (paths unchanged), the gate reports **success**. If a required job fails, the gate fails.

`Docker gate` and `Trivy gate` are separate on purpose: a broken Dockerfile fails Docker; a vulnerable image fails Trivy.

Ruleset:
https://github.com/GiorgioReale/folio-vitae/rules/16951196
