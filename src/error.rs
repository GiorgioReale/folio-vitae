use std::{env, num::ParseIntError, path::PathBuf};

use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    EnvVar(#[from] env::VarError),

    #[error(transparent)]
    ParseInt(#[from] ParseIntError),

    #[error(transparent)]
    Template(#[from] tera::Error),

    #[error(transparent)]
    SerdeYaml(#[from] serde_yaml::Error),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[cfg(feature = "build")]
    #[error(transparent)]
    Image(#[from] image::ImageError),

    #[cfg(feature = "smtp")]
    #[error(transparent)]
    Address(#[from] lettre::address::AddressError),

    #[cfg(feature = "smtp")]
    #[error(transparent)]
    Lettre(#[from] lettre::error::Error),

    #[cfg(feature = "smtp")]
    #[error(transparent)]
    LettreSmtp(#[from] lettre::transport::smtp::Error),

    #[error(transparent)]
    Dotenv(#[from] dotenvy::Error),

    #[cfg(feature = "serve")]
    #[error(transparent)]
    Hyper(#[from] hyper::Error),

    #[cfg(feature = "build")]
    #[error(transparent)]
    Svg(#[from] resvg::usvg::Error),

    #[cfg(feature = "build")]
    #[error(transparent)]
    Grass(#[from] Box<grass::Error>),

    #[error("Unsupported image format for {0:?}")]
    // Kept for future explicit image format validation.
    #[allow(dead_code)]
    UnsupportedImageFormat(PathBuf),

    #[error("{context}: {source}")]
    Contextual {
        context: String,
        #[source]
        source: Box<AppError>,
    },

    #[error("{0}")]
    Message(String),
}

pub trait ResultExt<T> {
    fn with_context<C, F>(self, f: F) -> AppResult<T>
    where
        F: FnOnce() -> C,
        C: Into<String>;
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    AppError: From<E>,
{
    fn with_context<C, F>(self, f: F) -> AppResult<T>
    where
        F: FnOnce() -> C,
        C: Into<String>,
    {
        self.map_err(|error| {
            let source = AppError::from(error);
            source.with_context(f())
        })
    }
}

impl AppError {
    pub fn with_context(self, context: impl Into<String>) -> AppError {
        AppError::Contextual { context: context.into(), source: Box::new(self) }
    }

    pub fn msg(message: impl Into<String>) -> AppError {
        AppError::Message(message.into())
    }
}
