# Required status checks on `main`

Branch ruleset **Protect main** requires these checks before merge:

| Check name | Workflow | Meaning |
|------------|----------|---------|
| `Merge gate` | Rust CI | Format/clippy/tests/MSRV when Rust paths change; `cargo audit` / `cargo deny` when dependency paths change |
| `Docker gate` | Docker | Multi-arch image build when Docker-related paths change |

Gates always run on PRs to `main`. If the underlying jobs are skipped (paths unchanged), the gate reports **success**. If a required job fails, the gate fails.

## After related security PRs merge

When these land, add matching always-on gates and extend the ruleset:

| Future check | Source PR / workflow |
|--------------|----------------------|
| Rust Security gate | `rust-security.yml` (scheduled audit/deny) |
| Trivy / image scan gate | Docker + Trivy image scanning |

Update the ruleset at:
https://github.com/GiorgioReale/folio-vitae/rules/16951196
