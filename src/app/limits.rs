//! Soft resource ceilings for operator-controlled inputs and in-memory caches.
//!
//! These are defense-in-depth bounds against accidental oversized YAML/images
//! and unbounded cache growth — not a visitor-facing rate limit.

/// Max bytes for a single YAML or translations JSON content file.
pub(crate) const MAX_CONTENT_FILE_BYTES: u64 = 2 * 1024 * 1024;

/// Max bytes for a source image on disk (before decode).
pub(crate) const MAX_IMAGE_FILE_BYTES: u64 = 30 * 1024 * 1024;

/// Max bytes for an SVG document loaded for rasterization.
pub(crate) const MAX_SVG_FILE_BYTES: u64 = 5 * 1024 * 1024;

/// Max decoded width×height for a raster/SVG source image.
pub(crate) const MAX_DECODED_PIXELS: u64 = 40_000_000;

/// Max items in a single CV profile collection (experiences, projects, …).
pub(crate) const MAX_PROFILE_LIST_ITEMS: usize = 200;

/// Max HTML pages retained in the production page cache.
pub(crate) const MAX_HTML_CACHE_ENTRIES: usize = 512;

/// Skip caching an individual HTML page larger than this.
pub(crate) const MAX_HTML_PAGE_BYTES: usize = 2 * 1024 * 1024;

/// Skip caching a CV PDF larger than this.
pub(crate) const MAX_PDF_CACHE_BYTES: usize = 15 * 1024 * 1024;
