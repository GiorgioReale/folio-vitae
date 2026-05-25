//! Core library for `folio-vitae`.
//!
//! Beginner-friendly map:
//!
//! - `src/main.rs` parses CLI commands
//! - this file wires together the high-level flows
//! - `app/` contains the website preparation and rendering logic
//! - `resources.rs` contains embedded starter files and runtime assets
//!
//! If you are new to Rust, start by reading:
//!
//! 1. `main()`
//! 2. `run_servers()`
//! 3. `run_servers_with_watch()`
//! 4. `prepare_application_state()`

pub mod app;
pub mod config;
pub mod error;
#[cfg(feature = "serve")]
pub mod http;
pub mod logging;
pub mod resources;

use std::{
    env,
    io::ErrorKind,
    net::{IpAddr, SocketAddr},
    path::Path,
    time::{Duration, SystemTime},
};

use app::{
    cache::{BuildCache, latest_modification},
    environment::Environment,
    localization::{detect_languages_with_last_change, load_language_names, load_localizations},
    setup::{build_app_states, load_templates},
    state::AppState,
};
use app::{compression, images, scripts, styles};
use config::{TEMPLATES_PATTERN, data_dir};
use dotenvy::from_path_override;
#[cfg(feature = "serve")]
use http::build_router;
#[cfg(feature = "serve")]
use tokio::sync::broadcast;
use tracing::debug;
use tracing::info;

pub use error::{AppError, AppResult, ResultExt};

pub struct PreparedApplication {
    pub main_state: AppState,
    pub cv_state: AppState,
    pub build_cache: BuildCache,
}

pub fn initialize_project(force: bool) -> AppResult<()> {
    resources::init_project_files(force)
}

pub fn load_env_file(path: &str) -> AppResult<()> {
    match from_path_override(path) {
        Ok(_) => Ok(()),
        Err(dotenvy::Error::Io(error)) if error.kind() == ErrorKind::NotFound => {
            debug!("No {path} file found; skipping environment loading from disk.");
            Ok(())
        },
        Err(error) => Err(AppError::from(error).with_context(format!("loading {path} file"))),
    }
}

pub fn prepare_application_state(environment: Environment) -> AppResult<PreparedApplication> {
    let mut build_cache = BuildCache::load()?;
    let data_directory = data_dir();

    let (available_languages, data_last_modified) = detect_languages_with_last_change()?;
    let assets_last_modified = latest_modification(&["assets/styles", "assets/scripts"])?;
    let templates_last_modified = latest_modification(&["templates"])?;
    let images_last_modified = latest_modification(&[data_directory.join("images")])?;
    let images_outputs_missing = images::generated_outputs_missing()?;
    let minify_assets = environment.should_minify_assets();

    if images_outputs_missing || build_cache.should_refresh_images(images_last_modified) {
        images::generate_all_images().with_context(|| "generating images")?;
        build_cache.record_images(images_last_modified);
    }

    if build_cache.should_refresh_assets(assets_last_modified, minify_assets) {
        styles::compile_styles(environment).with_context(|| "compiling SCSS")?;
        scripts::process_scripts(environment).with_context(|| "processing scripts")?;
        compression::gzip_assets().with_context(|| "gzipping assets")?;
        build_cache.record_assets(assets_last_modified, minify_assets);
    }

    build_cache.record_templates(templates_last_modified);
    build_cache.record_data(data_last_modified);

    let data = load_localizations(&available_languages)?;
    let language_names = load_language_names(&available_languages)?;
    let tera = load_templates(TEMPLATES_PATTERN)?;

    build_app_states(
        available_languages,
        data_last_modified,
        data,
        tera,
        environment,
        language_names,
    )
    .map(|states| PreparedApplication {
        main_state: states.0,
        cv_state: states.1,
        build_cache,
    })
}

#[cfg(feature = "serve")]
pub async fn run_servers<T>(
    environment: Environment,
    host: IpAddr,
    main_port: u16,
    cv_port: u16,
    shutdown_signal: impl std::future::Future<Output = AppResult<T>>,
) -> AppResult<T> {
    let PreparedApplication { main_state, cv_state, build_cache, .. } =
        prepare_application_state(environment)?;

    build_cache.save()?;

    let main_app = build_router(main_state);
    let cv_app = build_router(cv_state);

    let main_addr = SocketAddr::new(host, main_port);
    let cv_addr = SocketAddr::new(host, cv_port);

    debug!("Serving website_main on http://{main_addr}");
    debug!("Serving website_cv   on http://{cv_addr}");

    let main_listener = tokio::net::TcpListener::bind(main_addr)
        .await
        .with_context(|| "binding main tcp listener")?;

    let cv_listener =
        tokio::net::TcpListener::bind(cv_addr).await.with_context(|| "binding cv tcp listener")?;

    let main_server =
        axum::serve(main_listener, main_app.into_make_service_with_connect_info::<SocketAddr>());
    let cv_server =
        axum::serve(cv_listener, cv_app.into_make_service_with_connect_info::<SocketAddr>());

    let (shutdown_tx, _) = broadcast::channel(1);
    let mut main_shutdown = shutdown_tx.subscribe();
    let mut cv_shutdown = shutdown_tx.subscribe();

    let main_server_task = async move {
        main_server
            .with_graceful_shutdown(async move {
                let _ = main_shutdown.recv().await;
            })
            .await
            .with_context(|| "running main server")
    };

    let cv_server_task = async move {
        cv_server
            .with_graceful_shutdown(async move {
                let _ = cv_shutdown.recv().await;
            })
            .await
            .with_context(|| "running cv server")
    };

    let shutdown_task = async move {
        let shutdown_result = shutdown_signal.await;
        info!("Received shutdown signal. Shutting down servers...");
        let _ = shutdown_tx.send(());
        shutdown_result
    };

    let (_, _, shutdown_value) = tokio::try_join!(main_server_task, cv_server_task, shutdown_task)
        .with_context(|| "running servers")?;

    Ok(shutdown_value)
}

pub fn read_port_from_env(name: &str, default: u16) -> AppResult<u16> {
    read_port_from_source(name, default, |var_name| env::var(var_name))
}

fn read_port_from_source(
    name: &str,
    default: u16,
    get_var: impl for<'a> Fn(&'a str) -> Result<String, env::VarError>,
) -> AppResult<u16> {
    match get_var(name) {
        Ok(value) => value.parse::<u16>().with_context(|| format!("parsing {name} as port number")),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(env::VarError::NotUnicode(_)) => {
            Err(AppError::msg(format!("{name} contains invalid unicode characters")))
        },
    }
}

#[cfg(feature = "serve")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerLifecycle {
    Restart,
    Stop,
}

#[cfg(feature = "serve")]
#[derive(Clone, Debug, PartialEq, Eq)]
struct WatchSnapshot {
    assets: Option<SystemTime>,
    config: Option<SystemTime>,
    data: Option<SystemTime>,
    env_file: Option<SystemTime>,
    i18n: Option<SystemTime>,
    static_files: Option<SystemTime>,
    templates: Option<SystemTime>,
}

#[cfg(feature = "serve")]
pub async fn run_servers_with_watch(
    environment: Environment,
    host: IpAddr,
    main_port: u16,
    cv_port: u16,
    env_file: impl AsRef<Path>,
) -> AppResult<()> {
    let env_file = env_file.as_ref().to_path_buf();

    loop {
        let watch_snapshot = read_watch_snapshot(&env_file)?;
        let lifecycle = run_servers(environment, host, main_port, cv_port, async {
            tokio::select! {
                result = tokio::signal::ctrl_c() => {
                    result.with_context(|| "listening for shutdown signal")?;
                    Ok(ServerLifecycle::Stop)
                }
                result = wait_for_project_change(&env_file, watch_snapshot) => {
                    result?;
                    Ok(ServerLifecycle::Restart)
                }
            }
        })
        .await?;

        match lifecycle {
            ServerLifecycle::Restart => {
                info!("Project files changed. Restarting development servers...");
                load_env_file(&env_file.to_string_lossy())?;
            },
            ServerLifecycle::Stop => return Ok(()),
        }
    }
}

#[cfg(feature = "serve")]
fn read_watch_snapshot(env_file: &Path) -> AppResult<WatchSnapshot> {
    Ok(WatchSnapshot {
        assets: latest_modification(&[Path::new("assets")])?,
        config: latest_modification(&[Path::new("config.yml")])?,
        data: latest_modification(&[data_dir()])?,
        env_file: latest_modification(&[env_file])?,
        i18n: latest_modification(&[Path::new("i18n")])?,
        static_files: latest_modification(&[Path::new("static")])?,
        templates: latest_modification(&[Path::new("templates")])?,
    })
}

#[cfg(feature = "serve")]
async fn wait_for_project_change(env_file: &Path, baseline: WatchSnapshot) -> AppResult<()> {
    loop {
        tokio::time::sleep(Duration::from_millis(750)).await;

        if read_watch_snapshot(env_file)? != baseline {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, time::SystemTime};

    #[test]
    fn read_port_returns_default_when_missing() {
        let result =
            read_port_from_source(config::ENV_MAIN_PORT, 3000, |_| Err(env::VarError::NotPresent))
                .unwrap();

        assert_eq!(result, 3000);
    }

    #[test]
    fn read_port_parses_valid_number() {
        let parsed =
            read_port_from_source(config::ENV_MAIN_PORT, 3000, |_| Ok("8080".to_string())).unwrap();

        assert_eq!(parsed, 8080);
    }

    #[test]
    fn read_port_rejects_invalid_value() {
        let error =
            read_port_from_source(config::ENV_MAIN_PORT, 3000, |_| Ok("not-a-port".to_string()))
                .unwrap_err();

        assert!(error.to_string().contains("parsing"));
    }

    #[test]
    fn load_env_file_tolerates_missing_file() {
        let unique_stamp = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let temp_path = env::temp_dir().join(format!("folio-vitae-missing-{unique_stamp}.env"));

        if temp_path.exists() {
            fs::remove_file(&temp_path).expect("failed to clear pre-existing test file");
        }

        assert!(load_env_file(temp_path.to_str().expect("valid temp path")).is_ok());
    }
}
