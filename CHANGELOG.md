# Changelog

All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.10] - 2026-10-24

### Added
- Docker images published for `linux/amd64` and `linux/arm64`
- Compose HTTP healthcheck that probes both site ports

### Changed
- Compose file renamed to `compose.yaml` (Compose V2)

## [0.1.9] - 2026-10-19

### Added
- Optional employment and location types on work roles (with i18n labels)
- Cache-busting query params on asset URLs via `asset_version`

### Fixed
- Other-projects links use `link_title` for their labels

### Changed
- Generated assets rebuild on every start/restart
- CV birth date shows the date only (no calculated age)

## [0.1.8] - 2026-10-18

### Fixed
- CV home background images no longer distort (center-crop cover resize and CSS `object-fit`)

## [0.1.7] - 2026-10-17

### Fixed
- Transparent profile photos are kept as PNG (instead of JPEG) and PDF portraits blend onto the text-color background
- CV PDF download filenames include the language (e.g. `Name - CV - en.pdf`)
- Website and email values in the CV PDF are clickable links
- Curriculum website falls back to the CV site domain when no main site is configured

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
