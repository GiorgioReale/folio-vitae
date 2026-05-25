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

#[cfg(feature = "build")]
pub mod compression;
#[cfg(feature = "build")]
pub mod images;
#[cfg(feature = "build")]
pub mod render;
#[cfg(feature = "build")]
pub mod scripts;
#[cfg(feature = "build")]
pub mod styles;
