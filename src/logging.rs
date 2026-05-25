use tracing_subscriber::{EnvFilter, fmt};

use crate::error::{AppError, AppResult};

/// Initialize structured logging with sensible defaults.
pub fn init_tracing() -> AppResult<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=debug"));

    init_tracing_with_filter(env_filter)
}

pub fn init_tracing_with_filter(env_filter: EnvFilter) -> AppResult<()> {
    fmt().with_env_filter(env_filter).with_target(false).compact().try_init().map_err(|error| {
        AppError::msg(format!("failed to initialize tracing subscriber: {error}"))
    })?;

    Ok(())
}
