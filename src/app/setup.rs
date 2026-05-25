use std::{
    collections::HashMap,
    env,
    fmt::Write as FmtWrite,
    fs,
    sync::{Arc, RwLock},
    time::SystemTime,
};

use sha2::{Digest, Sha256};
use tera::Tera;
use walkdir::WalkDir;

use crate::{
    app::{
        email::build_mailer,
        environment::Environment,
        localization::{Language, Localizations},
        state::{AppState, LocalizationState, SiteKind, compile_static_assets, order_languages},
    },
    error::{AppResult, ResultExt},
};

pub(crate) fn build_app_states(
    supported_languages: Vec<Language>,
    data_last_modified: Option<SystemTime>,
    localizations: Localizations,
    tera: Tera,
    environment: Environment,
    language_names: HashMap<Language, String>,
) -> AppResult<(AppState, AppState)> {
    let (default_language, default_from_env) = Language::default(&supported_languages)?;
    let supported_languages =
        order_languages(supported_languages, &default_language, default_from_env);
    let default_resources = localizations
        .get(&default_language)
        .cloned()
        .ok_or_else(|| crate::error::AppError::msg("loading default language resources"))?;

    let main_static =
        compile_static_assets("static/main", SiteKind::Main, &default_resources.data)?;
    let cv_static = compile_static_assets("static/cv", SiteKind::Cv, &default_resources.data)?;
    let mailer = build_mailer(&default_resources.data)?;

    let asset_version = compute_asset_version()?;
    let show_credits = credits_enabled_from_env();
    let generator = format!("folio-vitae v{}", env!("CARGO_PKG_VERSION"));

    let shared_localization_state = Arc::new(RwLock::new(Arc::new(LocalizationState::new(
        localizations,
        default_language.clone(),
        supported_languages.clone(),
        language_names.clone(),
    ))));
    let data_last_modified = Arc::new(RwLock::new(data_last_modified));

    let shared_state = AppState {
        localization_state: shared_localization_state,
        data_last_modified,
        tera: Arc::new(tera),
        main_static,
        cv_static,
        default_site: SiteKind::Main,
        environment,
        asset_version: asset_version.clone(),
        generator: generator.clone(),
        mailer: mailer.clone(),
        show_credits,
    };

    Ok((
        AppState { default_site: SiteKind::Main, ..shared_state.clone() },
        AppState { default_site: SiteKind::Cv, asset_version, generator, ..shared_state },
    ))
}

pub fn load_templates(_pattern: &str) -> AppResult<Tera> {
    let mut tera = Tera::default();

    for entry in WalkDir::new("templates")
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("html") {
            continue;
        }

        let template_name = path.strip_prefix("templates").map_err(|error| {
            crate::error::AppError::msg(format!("building template path for {path:?}: {error}"))
        })?;
        let template_name = template_name
            .to_string_lossy()
            .trim_start_matches(std::path::MAIN_SEPARATOR)
            .replace('\\', "/");

        let contents =
            fs::read_to_string(path).with_context(|| format!("reading template {path:?}"))?;
        tera.add_raw_template(&template_name, &contents)
            .with_context(|| format!("loading template {template_name}"))?;
    }

    tera.autoescape_on(vec![".html"]);
    Ok(tera)
}

fn compute_asset_version() -> AppResult<String> {
    let mut files: Vec<_> = WalkDir::new("assets")
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            matches!(entry.path().extension().and_then(|ext| ext.to_str()), Some("css" | "js"))
        })
        .map(|entry| entry.into_path())
        .collect();

    files.sort();

    let mut hasher = Sha256::new();

    for path in files {
        let contents = fs::read(&path).with_context(|| format!("reading {path:?}"))?;
        hasher.update(path.to_string_lossy().as_bytes());
        hasher.update(&contents);
    }

    let digest = hasher.finalize();
    let mut version = String::new();

    for byte in digest.as_slice().iter().take(8) {
        write!(&mut version, "{byte:02x}").map_err(|error| {
            crate::error::AppError::msg(format!("writing asset version: {error}"))
        })?;
    }

    Ok(version)
}

fn credits_enabled_from_env() -> bool {
    match env::var("CREDITS") {
        Ok(value) => !value.eq_ignore_ascii_case("false"),
        Err(_) => true,
    }
}
