# Changelog

All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
