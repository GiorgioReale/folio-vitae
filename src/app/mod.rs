//! Application internals used by the CLI.
//!
//! Rough guide:
//!
//! - `setup`, `state`, `models`: load and organize project data
//! - `localization`: language and translation handling
//! - `styles`, `scripts`, `images`, `compression`: asset pipeline
//! - `render`: turns state into final HTML pages

pub mod cache;
pub mod email;
pub mod environment;
pub mod localization;
pub mod models;
pub mod setup;
pub mod state;

#[cfg(feature = "serve")]
pub mod compression;
#[cfg(feature = "serve")]
pub mod images;
#[cfg(feature = "serve")]
pub mod render;
#[cfg(feature = "serve")]
pub mod scripts;
#[cfg(feature = "serve")]
pub mod styles;
