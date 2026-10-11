# Changelog

All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.6] - 2026-10-11

### Added
- Dedicated `/languages` page with return-to-path navigation for language switching

### Fixed
- Bare paths without a language prefix now honor `Accept-Language` (with `Vary` / no-store redirects)
- Successful contact submissions clear the form and show a confirmation state
- Docker image version tags are published correctly when the build is triggered from a GitHub Release

### Changed
- CV menu and curriculum section focus targets for smoother in-page navigation

### Performance
- Cheaper production HTML serving via shared `Arc` page cache entries
- Cached PDF skill-icon rasterization and profile photo bytes
- Release profile uses fat LTO and `panic = "abort"`

## [0.1.5] - 2026-10-11

### Fixed
- Contact form requires `Origin`, rate-limits submissions, keeps message newlines, and sets `Reply-To`
- Package upgrades no longer overwrite customized templates, assets, static files, or i18n
- Multilingual sitemaps/`hreflang`, localized social-card alts, and complete CV menu section links
- Accessibility and UX polish for the contact form, theme flash, hero LCP, and control hit targets

### Changed
- SMTP is enabled in the default feature set
- Bare `folio-vitae` requires an explicit subcommand (`init`, `dev`, or `prod`)
- `init` also scaffolds Spanish starter data

### Performance
- PDF render singleflight, production HTML page cache, and less aggressive localization recheck

## [0.1.4] - 2026-10-10

### Fixed
- SMTP on port 587 now uses STARTTLS instead of implicit TLS (contact form)

### CI
- Narrower workflow triggers and clearer Docker image tag behavior

## [0.1.3] - 2026-10-10

### Added
- Official Alpine container image published to GHCR
- `Dockerfile`, `docker-compose.yml`, and Docker docs in the README

### Changed
- Production `serve` now shuts down cleanly on SIGTERM (container-friendly)

### CI
- Docker build/publish workflow for `main`, releases, and manual runs
- Dependabot coverage for the Docker ecosystem

## [0.1.2] - 2026-10-10

### Added
- CV PDF generation and related UI polish
- Default local ports based on the FOLIO keypad layout

### Changed
- Migrated templates to Tera 2 and bumped dependencies
- Rewrote README for clearer install and usage docs
- Aligned Italian sample data with the English and Spanish examples

### Fixed
- rustfmt failure in the `read_port` test

### CI
- Publish to crates.io only from GitHub releases
- Dependabot for Cargo and GitHub Actions
