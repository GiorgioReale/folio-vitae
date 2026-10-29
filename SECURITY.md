# Security Policy

## Reporting a Vulnerability

If you discover a security issue, please email security@giorgioreale.xyz with a detailed description of the vulnerability, steps to reproduce it, and any relevant logs or screenshots. Do not create public issues for security-sensitive information.

We aim to acknowledge new reports within five business days and will provide updates as we validate and triage the issue. If you have not received a response within that timeframe, feel free to send a polite follow-up.

## Supported Versions

Security fixes are applied to the most recent release and the active development branch. Older versions may not receive patches.

## Handling Sensitive Data

When sharing proof-of-concept material, please minimize sensitive or personal data. Use sample credentials where possible and avoid uploading real secrets to any repository or third-party service.

## Coordinated Disclosure

We appreciate coordinated disclosure. Once a fix is available, we will agree on a public disclosure timeline with the reporter, crediting those who responsibly report vulnerabilities (unless anonymity is requested).

## Ignoring advisories

Ignored findings must never be open-ended.

| Where | How |
|-------|-----|
| Rust (`deny.toml`) | Use `{ id = "RUSTSEC-…", reason = "…" }` and put **why**, **what clears it**, and **revisit by YYYY-MM-DD** in `reason`. |
| Container (`/.trivyignore`) | Prefer `CVE-… exp:YYYY-MM-DD` so the ignore expires automatically. |

Before adding an ignore: confirm impact on this project, prefer upgrading first, and schedule a revisit. Remove the entry when upstream is fixed or the dependency is gone.
