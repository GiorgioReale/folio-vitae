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
//! 2. `run_build()`
//! 3. `run_servers()`
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
};
#[cfg(feature = "build")]
use std::{fs, path::Path};

#[cfg(feature = "build")]
use ::http::StatusCode;
use app::{
    cache::{self, BuildCache, InputSnapshot, latest_modification},
    environment::Environment,
    localization::{detect_languages_with_last_change, load_language_names, load_localizations},
    setup::{build_app_states, load_templates},
    state::{AppState, SiteKind},
};
#[cfg(feature = "build")]
use app::{
    compression, images,
    render::{render_cv_curriculum, render_cv_homepage, render_cv_menu, render_main_homepage},
    scripts, styles,
};
use config::{TEMPLATES_PATTERN, data_dir};
use dotenvy::from_path_override;
#[cfg(feature = "serve")]
use http::build_router;
#[cfg(feature = "serve")]
use tokio::sync::broadcast;
use tracing::debug;
#[cfg(any(feature = "build", feature = "serve"))]
use tracing::info;
#[cfg(feature = "build")]
use walkdir::WalkDir;

pub use error::{AppError, AppResult, ResultExt};

#[cfg(feature = "build")]
pub struct PreparedApplication {
    pub main_state: AppState,
    pub cv_state: AppState,
    pub build_cache: BuildCache,
    pub snapshot: InputSnapshot,
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

#[cfg(feature = "build")]
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
        snapshot: InputSnapshot {
            data: cache::system_time_to_timestamp(data_last_modified),
            assets: cache::system_time_to_timestamp(assets_last_modified),
            templates: cache::system_time_to_timestamp(templates_last_modified),
            images: cache::system_time_to_timestamp(images_last_modified),
            minify_assets,
        },
    })
}

#[cfg(feature = "build")]
pub fn run_build(environment: Environment) -> AppResult<()> {
    let PreparedApplication { main_state, cv_state, mut build_cache, snapshot } =
        prepare_application_state(environment)?;

    let dist_root = Path::new("dist");

    if build_cache.last_build_matches(&snapshot) && dist_root.exists() {
        info!("Skipping build; no changes detected since last run.");
        return Ok(());
    }

    build_static_sites(&main_state, &cv_state, dist_root)?;

    build_cache.record_build(&snapshot);
    build_cache.save()?;

    Ok(())
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

#[cfg(feature = "build")]
pub fn build_static_sites(
    main_state: &AppState,
    cv_state: &AppState,
    dist_root: impl AsRef<Path>,
) -> AppResult<()> {
    let dist_root = dist_root.as_ref();

    if dist_root.exists() {
        fs::remove_dir_all(dist_root).with_context(|| "clearing existing dist directory")?;
    }

    fs::create_dir_all(dist_root).with_context(|| "creating dist directory")?;

    if main_state.site_available(SiteKind::Main) {
        build_site(main_state, SiteKind::Main, &dist_root.join("main"))?;
    }

    if cv_state.site_available(SiteKind::Cv) {
        build_site(cv_state, SiteKind::Cv, &dist_root.join("cv"))?;
    }

    Ok(())
}

#[cfg(feature = "build")]
pub fn build_site(state: &AppState, site: SiteKind, output_root: &Path) -> AppResult<()> {
    fs::create_dir_all(output_root).with_context(|| format!("creating {output_root:?}"))?;

    copy_assets(output_root)?;
    write_static_files(state, site, output_root)?;
    render_site_pages(state, site, output_root)
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

#[cfg(feature = "build")]
fn copy_assets(target_root: &Path) -> AppResult<()> {
    copy_directory(Path::new("assets"), &target_root.join("assets"))
}

#[cfg(feature = "build")]
fn copy_directory(source: &Path, destination: &Path) -> AppResult<()> {
    for entry in WalkDir::new(source) {
        let entry = entry
            .map_err(|error| AppError::msg(format!("walking assets in {source:?}: {error}")))?;
        let relative_path = entry.path().strip_prefix(source).map_err(|error| {
            AppError::msg(format!("computing relative path for {:?}: {error}", entry.path()))
        })?;
        let target_path = destination.join(relative_path);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&target_path)
                .with_context(|| format!("creating directory {target_path:?}"))?;
            continue;
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating parent directory {parent:?}"))?;
        }

        fs::copy(entry.path(), &target_path)
            .with_context(|| format!("copying {:?} to {:?}", entry.path(), target_path))?;
    }

    Ok(())
}

#[cfg(feature = "build")]
fn write_static_files(state: &AppState, site: SiteKind, output_root: &Path) -> AppResult<()> {
    let assets = match site {
        SiteKind::Main => &state.main_static,
        SiteKind::Cv => &state.cv_static,
    };

    write_text_file(&output_root.join("robots.txt"), &assets.robots)?;
    write_text_file(&output_root.join("sitemap.xml"), &assets.sitemap)?;
    write_text_file(&output_root.join("site.webmanifest"), &assets.manifest)?;
    write_text_file(&output_root.join(".well-known").join("security.txt"), &assets.security_txt)
}

#[cfg(feature = "build")]
fn render_site_pages(state: &AppState, site: SiteKind, output_root: &Path) -> AppResult<()> {
    let snapshot = state.localization_snapshot();
    let is_multilingual = snapshot.supported_languages.len() > 1;
    let default_language = snapshot.default_language.clone();

    for language in &snapshot.supported_languages {
        if !snapshot
            .localizations
            .get(language)
            .map(|resources| match site {
                SiteKind::Main => resources.data.sites.has_main(),
                SiteKind::Cv => resources.data.sites.has_cv(),
            })
            .unwrap_or(false)
        {
            continue;
        }

        let language_prefix = language.url_prefix(&default_language, is_multilingual);
        let homepage_path = if language_prefix.is_empty() {
            "/".to_string()
        } else {
            format!("{language_prefix}/")
        };

        match site {
            SiteKind::Main => {
                let homepage = html_content(
                    render_main_homepage(state, language, &homepage_path),
                    &homepage_path,
                )?;

                write_page(output_root, &homepage_path, &homepage)?;
            },
            SiteKind::Cv => {
                let homepage = html_content(
                    render_cv_homepage(state, language, &homepage_path),
                    &homepage_path,
                )?;
                write_page(output_root, &homepage_path, &homepage)?;

                let curriculum_path = build_page_path(&language_prefix, "/curriculum");
                let curriculum = html_content(
                    render_cv_curriculum(state, None, language, &curriculum_path),
                    &curriculum_path,
                )?;
                write_page(output_root, &curriculum_path, &curriculum)?;

                let menu_path = build_page_path(&language_prefix, "/menu");
                let menu = html_content(render_cv_menu(state, language, &menu_path), &menu_path)?;
                write_page(output_root, &menu_path, &menu)?;
            },
        }
    }

    Ok(())
}

#[cfg(feature = "build")]
fn html_content(result: Result<String, StatusCode>, page: &str) -> AppResult<String> {
    result.map_err(|status| AppError::msg(format!("failed to render {page}: {status}")))
}

#[cfg(feature = "build")]
fn write_page(output_root: &Path, request_path: &str, contents: &str) -> AppResult<()> {
    let trimmed = request_path.trim_start_matches('/');
    let normalized = trimmed.trim_end_matches('/');
    let target_dir = if normalized.is_empty() {
        output_root.to_path_buf()
    } else {
        output_root.join(normalized)
    };

    fs::create_dir_all(&target_dir)
        .with_context(|| format!("creating directory {target_dir:?}"))?;

    let target_file = target_dir.join("index.html");

    fs::write(&target_file, contents)
        .with_context(|| format!("writing rendered page to {target_file:?}"))
}

#[cfg(feature = "build")]
fn write_text_file(path: &Path, contents: &str) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating directory {parent:?}"))?;
    }

    fs::write(path, contents).with_context(|| format!("writing {path:?}"))
}

#[cfg(feature = "build")]
fn build_page_path(prefix: &str, suffix: &str) -> String {
    if prefix.is_empty() { suffix.to_string() } else { format!("{prefix}{suffix}") }
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
