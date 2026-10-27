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

    let localization_state = LocalizationState::new(
        localizations,
        default_language.clone(),
        supported_languages.clone(),
        language_names,
    );
    let asset_version = Arc::<str>::from(compute_asset_version()?);
    let main_static = Arc::new(compile_static_assets(
        "static/main",
        SiteKind::Main,
        &default_resources.data,
        &localization_state,
        &asset_version,
    )?);
    let cv_static = Arc::new(compile_static_assets(
        "static/cv",
        SiteKind::Cv,
        &default_resources.data,
        &localization_state,
        &asset_version,
    )?);
    let mailer = build_mailer(&default_resources.data)?.map(Arc::new);
    let show_credits = credits_enabled_from_env();
    let generator = Arc::<str>::from(format!("folio-vitae v{}", env!("CARGO_PKG_VERSION")));
    let trusted_proxies = crate::config::trusted_proxies()?;

    let shared_localization_state = Arc::new(RwLock::new(Arc::new(localization_state)));
    let data_last_modified = Arc::new(RwLock::new(data_last_modified));

    let shared_state = AppState {
        localization_state: shared_localization_state,
        data_last_modified,
        localization_checked_at: Arc::new(std::sync::Mutex::new(None)),
        tera: Arc::new(tera),
        main_static,
        cv_static,
        default_site: SiteKind::Main,
        environment,
        asset_version,
        generator,
        mailer,
        show_credits,
        cv_pdf_cache: Arc::new(RwLock::new(HashMap::new())),
        cv_pdf_locks: Arc::new(std::sync::Mutex::new(HashMap::new())),
        html_page_cache: Arc::new(RwLock::new(HashMap::new())),
        contact_rate_limiter: Arc::new(crate::app::rate_limit::ContactRateLimiter::default()),
        trusted_proxies,
    };

    Ok((
        AppState { default_site: SiteKind::Main, ..shared_state.clone() },
        AppState { default_site: SiteKind::Cv, ..shared_state },
    ))
}

pub fn load_templates() -> AppResult<Tera> {
    let mut templates = Vec::new();

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
        templates.push((template_name, contents));
    }

    let mut tera = Tera::default();
    tera.add_raw_templates(templates).with_context(|| "loading templates".to_string())?;

    tera.autoescape_on(vec![".html"]);
    Ok(tera)
}

fn compute_asset_version() -> AppResult<String> {
    let mut files: Vec<_> = WalkDir::new("assets")
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("gz") {
                return false;
            }
            matches!(
                path.extension().and_then(|ext| ext.to_str()),
                Some(
                    "css"
                        | "js"
                        | "scss"
                        | "svg"
                        | "png"
                        | "jpg"
                        | "jpeg"
                        | "avif"
                        | "webp"
                        | "gif"
                        | "ico"
                        | "woff"
                        | "woff2"
                        | "mp3"
                )
            )
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

#[cfg(test)]
mod tests {
    use super::load_templates;

    #[test]
    fn templates_and_chrome_components_load() {
        let tera = load_templates().expect("templates should load under Tera 2");
        assert!(tera.contains_component("chrome.skip_link"));
        assert!(tera.contains_component("chrome.site_header"));
        assert!(tera.contains_component("chrome.site_footer"));
        assert!(tera.contains_template("main/pages/homepage.html"));
        assert!(tera.contains_template("cv/pages/curriculum.html"));
        assert!(tera.contains_template("errors/error.html"));
    }
}
