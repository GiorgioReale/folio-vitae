use std::{env, net::IpAddr, path::PathBuf, sync::Arc};

use crate::error::{AppError, AppResult};

/// Environment variable name for the main website port.
pub const ENV_MAIN_PORT: &str = "MAIN_PORT";

/// Environment variable name for the CV website port.
pub const ENV_CV_PORT: &str = "CV_PORT";

/// Environment variable name for the project data directory.
pub const ENV_DATA_DIR: &str = "DATA_DIR";

/// Optional comma-separated proxy IPs trusted for `X-Forwarded-Proto`.
pub const ENV_TRUSTED_PROXIES: &str = "TRUSTED_PROXIES";

/// Default port used when the main website port is not provided.
pub const DEFAULT_MAIN_PORT: u16 = 36546;

/// Default port used when the CV website port is not provided.
pub const DEFAULT_CV_PORT: u16 = 36547;

/// Default directory used for localized content and source images.
pub const DEFAULT_DATA_DIR: &str = "data";

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

pub fn trusted_proxies() -> AppResult<Arc<[IpAddr]>> {
    trusted_proxies_from_env(|| env::var(ENV_TRUSTED_PROXIES).ok())
}

fn trusted_proxies_from_env<F>(env_lookup: F) -> AppResult<Arc<[IpAddr]>>
where
    F: Fn() -> Option<String>,
{
    let Some(raw) =
        env_lookup().map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
    else {
        return Ok(Arc::from([]));
    };

    let mut proxies = Vec::new();
    for part in raw.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let ip = part.parse::<IpAddr>().map_err(|_| {
            AppError::msg(format!(
                "invalid IP `{part}` in {ENV_TRUSTED_PROXIES}; use comma-separated addresses like `10.0.0.5,2001:db8::1`"
            ))
        })?;
        proxies.push(ip);
    }

    Ok(Arc::from(proxies))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

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

    #[test]
    fn trusted_proxies_defaults_to_empty() {
        let proxies = trusted_proxies_from_env(|| None).unwrap();
        assert!(proxies.is_empty());
    }

    #[test]
    fn trusted_proxies_parses_comma_separated_ips() {
        let proxies =
            trusted_proxies_from_env(|| Some(" 10.0.0.5, 2001:db8::1 , ".into())).unwrap();
        assert_eq!(
            proxies.as_ref(),
            &[
                IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5)),
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1))
            ]
        );
    }

    #[test]
    fn trusted_proxies_rejects_invalid_entries() {
        let error = trusted_proxies_from_env(|| Some("10.0.0.5,not-an-ip".into())).unwrap_err();
        assert!(error.to_string().contains("invalid IP `not-an-ip`"));
    }
}
