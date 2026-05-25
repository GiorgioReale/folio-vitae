use std::{env, path::PathBuf};

/// Environment variable name for the main website port.
pub const ENV_MAIN_PORT: &str = "MAIN_PORT";

/// Environment variable name for the CV website port.
pub const ENV_CV_PORT: &str = "CV_PORT";

/// Environment variable name for the project data directory.
pub const ENV_DATA_DIR: &str = "DATA_DIR";

/// Default port used when the main website port is not provided.
pub const DEFAULT_MAIN_PORT: u16 = 8080;

/// Default port used when the CV website port is not provided.
pub const DEFAULT_CV_PORT: u16 = 8081;

/// Default directory used for localized content and source images.
pub const DEFAULT_DATA_DIR: &str = "data";

/// Glob pattern used to discover template files.
pub const TEMPLATES_PATTERN: &str = "templates/**/*";

pub fn data_dir() -> PathBuf {
    data_dir_from_env(|| env::var_os(ENV_DATA_DIR))
}

fn data_dir_from_env<F>(env_lookup: F) -> PathBuf
where
    F: Fn() -> Option<std::ffi::OsString>,
{
    env_lookup()
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DATA_DIR))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dir_defaults_to_local_data_folder() {
        let path = data_dir_from_env(|| None);

        assert_eq!(path, PathBuf::from("data"));
    }

    #[test]
    fn data_dir_uses_environment_override() {
        let path = data_dir_from_env(|| Some("/tmp/my-data".into()));

        assert_eq!(path, PathBuf::from("/tmp/my-data"));
    }
}
