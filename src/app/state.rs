use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::SystemTime,
};

use chrono::{Datelike, SecondsFormat, TimeZone, Utc};
use serde::Serialize;
use tera::{Context, Tera};

use crate::{
    app::{
        email::Mailer,
        environment::Environment,
        localization::{
            Language, Localizations, LocalizedResources, detect_languages_with_last_change,
            load_language_names, load_localizations,
        },
        models::SiteData,
    },
    error::{AppError, AppResult, ResultExt},
};

#[derive(Clone)]
pub struct AppState {
    pub(crate) localization_state: Arc<RwLock<Arc<LocalizationState>>>,
    pub(crate) data_last_modified: Arc<RwLock<Option<SystemTime>>>,
    pub(crate) tera: Arc<Tera>,
    pub(crate) main_static: StaticAssets,
    pub(crate) cv_static: StaticAssets,
    pub(crate) default_site: SiteKind,
    pub(crate) environment: Environment,
    pub(crate) asset_version: String,
    pub(crate) generator: String,
    pub(crate) mailer: Option<Mailer>,
    pub(crate) show_credits: bool,
}

#[derive(Clone)]
pub(crate) struct LocalizationState {
    pub(crate) localizations: Localizations,
    pub(crate) default_language: Language,
    pub(crate) supported_languages: Vec<Language>,
    pub(crate) language_names: HashMap<Language, String>,
    pub(crate) main_domains: Vec<String>,
    pub(crate) cv_domains: Vec<String>,
    pub(crate) main_available: bool,
    pub(crate) cv_available: bool,
}

#[derive(Clone)]
pub(crate) struct StaticAssets {
    pub robots: String,
    pub sitemap: String,
    pub manifest: String,
    pub security_txt: String,
}

#[derive(Serialize)]
pub(crate) struct LanguageLink {
    pub code: String,
    pub label: String,
    pub url: String,
    pub is_current: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SiteKind {
    Main,
    Cv,
}

impl AppState {
    pub(crate) fn resources_for(&self, language: &Language) -> AppResult<LocalizedResources> {
        let snapshot = self.localization_snapshot();

        snapshot
            .localizations
            .get(language)
            .cloned()
            .or_else(|| snapshot.localizations.get(&snapshot.default_language).cloned())
            .or_else(|| snapshot.localizations.values().next().cloned())
            .ok_or_else(|| AppError::msg("no localization resources available"))
    }

    pub(crate) fn language_from_path(&self, path: &str) -> (Language, String, String) {
        let snapshot = self.localization_snapshot();
        let clean_path = path.trim_matches('/');

        if snapshot.supported_languages.len() == 1 {
            let language = snapshot.supported_languages.first().cloned().unwrap_or_else(|| {
                eprintln!("Supported languages missing; defaulting to English");
                Language::new("en")
            });

            let mut segments = clean_path.split('/');

            if let Some(first_segment) = segments.next()
                && first_segment == language.as_code()
            {
                let rest: String = segments.collect::<Vec<_>>().join("/");
                let normalized = if rest.is_empty() { "/".to_string() } else { format!("/{rest}") };

                return (language, normalized, String::new());
            }

            let normalized =
                if clean_path.is_empty() { "/".to_string() } else { format!("/{clean_path}") };

            return (language, normalized, String::new());
        }

        if clean_path.is_empty() {
            return (snapshot.default_language.clone(), "/".to_string(), String::new());
        }

        let mut segments = clean_path.split('/');
        if let Some(first_segment) = segments.next()
            && let Some(language) =
                Language::from_code(first_segment, &snapshot.supported_languages)
        {
            let rest: String = segments.collect::<Vec<_>>().join("/");
            let normalized = if rest.is_empty() { "/".to_string() } else { format!("/{rest}") };

            return (language, normalized, format!("/{first_segment}"));
        }

        (snapshot.default_language.clone(), format!("/{clean_path}"), String::new())
    }

    pub(crate) fn language_has_site(&self, language: &Language, site: SiteKind) -> bool {
        let snapshot = self.localization_snapshot();

        language_has_site_in_snapshot(language, site, &snapshot)
    }

    pub(crate) fn site_available(&self, site: SiteKind) -> bool {
        let snapshot = self.localization_snapshot();

        site_available_in_snapshot(site, &snapshot)
    }

    pub(crate) fn redirect_path_for_missing_language(&self, path: &str) -> Option<String> {
        let snapshot = self.localization_snapshot();

        if snapshot.supported_languages.len() == 1 {
            let language = snapshot.supported_languages.first()?;
            let mut segments = path.trim_matches('/').split('/');

            if let Some(first_segment) = segments.next()
                && first_segment == language.as_code()
            {
                let rest: String = segments.collect::<Vec<_>>().join("/");

                return Some(if rest.is_empty() { "/".to_string() } else { format!("/{rest}") });
            }

            return None;
        }

        if path.trim_matches('/').is_empty() {
            return Some(format!("/{}", snapshot.default_language.as_code()));
        }

        let (_, normalized_path, language_prefix) = self.language_from_path(path);

        if language_prefix.is_empty() {
            let default_code = snapshot.default_language.as_code();
            let mut target_suffix =
                if normalized_path == "/" { "".to_string() } else { normalized_path };

            if let Some(first_segment) = target_suffix.trim_start_matches('/').split('/').next()
                && !first_segment.is_empty()
                && !snapshot
                    .supported_languages
                    .iter()
                    .any(|language| language.as_code() == first_segment)
            {
                let remainder: Vec<_> =
                    target_suffix.trim_start_matches('/').split('/').skip(1).collect();

                target_suffix = if remainder.is_empty() {
                    "/".to_string()
                } else {
                    format!("/{}", remainder.join("/"))
                };
            }

            if target_suffix == "/" {
                target_suffix.clear();
            }

            Some(format!("/{default_code}{target_suffix}"))
        } else {
            None
        }
    }

    pub(crate) fn language_links(
        &self,
        current_language: &Language,
        request_path: &str,
        site: SiteKind,
    ) -> Vec<LanguageLink> {
        let snapshot = self.localization_snapshot();

        let available_languages: Vec<_> = snapshot
            .supported_languages
            .iter()
            .filter(|language| language_has_site_in_snapshot(language, site, &snapshot))
            .collect();

        if available_languages.len() <= 1 {
            return Vec::new();
        }

        let (_, normalized_path, _) = self.language_from_path(request_path);
        let path_suffix = if normalized_path == "/" { "/".to_string() } else { normalized_path };

        available_languages
            .into_iter()
            .map(|language| LanguageLink {
                code: language.as_code().to_string(),
                label: snapshot
                    .language_names
                    .get(language)
                    .cloned()
                    .unwrap_or_else(|| language.as_code().to_string()),
                url: format!(
                    "{}{}",
                    language.url_prefix(&snapshot.default_language, true),
                    path_suffix
                ),
                is_current: language == current_language,
            })
            .collect()
    }

    pub(crate) fn localization_snapshot(&self) -> Arc<LocalizationState> {
        if self.should_refresh_localizations()
            && let Err(error) = self.try_refresh_localizations()
        {
            eprintln!("Failed to refresh localization data: {error:#?}");
        }

        self.localization_state.read().unwrap_or_else(|error| error.into_inner()).clone()
    }

    fn should_refresh_localizations(&self) -> bool {
        matches!(self.environment, Environment::Development)
    }

    fn try_refresh_localizations(&self) -> AppResult<()> {
        let (available_languages, latest_change) = detect_languages_with_last_change()?;

        {
            let current_state =
                self.localization_state.read().unwrap_or_else(|error| error.into_inner());

            let languages_unchanged = available_languages == current_state.supported_languages;
            let timestamp_unchanged = latest_change
                == *self.data_last_modified.read().unwrap_or_else(|error| error.into_inner());

            if languages_unchanged && timestamp_unchanged {
                return Ok(());
            }
        }

        let localizations = load_localizations(&available_languages)?;
        let language_names = load_language_names(&available_languages)?;
        let (default_language, default_from_env) = Language::default(&available_languages)?;
        let ordered_languages =
            order_languages(available_languages, &default_language, default_from_env);

        {
            let mut state =
                self.localization_state.write().unwrap_or_else(|error| error.into_inner());
            *state = Arc::new(LocalizationState::new(
                localizations,
                default_language,
                ordered_languages,
                language_names,
            ));
        }

        *self.data_last_modified.write().unwrap_or_else(|error| error.into_inner()) = latest_change;

        Ok(())
    }
}

impl LocalizationState {
    pub(crate) fn new(
        localizations: Localizations,
        default_language: Language,
        supported_languages: Vec<Language>,
        language_names: HashMap<Language, String>,
    ) -> Self {
        let main_domains =
            collect_site_domains(&supported_languages, &localizations, SiteKind::Main);
        let cv_domains = collect_site_domains(&supported_languages, &localizations, SiteKind::Cv);
        let main_available = supported_languages.iter().any(|language| {
            language_has_site_in_localizations(language, SiteKind::Main, &localizations)
        });
        let cv_available = supported_languages.iter().any(|language| {
            language_has_site_in_localizations(language, SiteKind::Cv, &localizations)
        });

        Self {
            localizations,
            default_language,
            supported_languages,
            language_names,
            main_domains,
            cv_domains,
            main_available,
            cv_available,
        }
    }
}

fn language_has_site_in_snapshot(
    language: &Language,
    site: SiteKind,
    snapshot: &LocalizationState,
) -> bool {
    language_has_site_in_localizations(language, site, &snapshot.localizations)
}

fn site_available_in_snapshot(site: SiteKind, snapshot: &LocalizationState) -> bool {
    match site {
        SiteKind::Main => snapshot.main_available,
        SiteKind::Cv => snapshot.cv_available,
    }
}

fn language_has_site_in_localizations(
    language: &Language,
    site: SiteKind,
    localizations: &Localizations,
) -> bool {
    localizations
        .get(language)
        .map(|resources| match site {
            SiteKind::Main => resources.data.sites.has_main(),
            SiteKind::Cv => resources.data.sites.has_cv(),
        })
        .unwrap_or(false)
}

fn collect_site_domains(
    supported_languages: &[Language],
    localizations: &Localizations,
    site: SiteKind,
) -> Vec<String> {
    let mut domains = supported_languages
        .iter()
        .filter_map(|language| localizations.get(language))
        .filter_map(|resources| {
            let domain = match site {
                SiteKind::Main => resources.data.sites.main.domain.as_str(),
                SiteKind::Cv => resources.data.sites.cv.domain.as_str(),
            };

            let normalized = domain.trim().to_ascii_lowercase();

            (!normalized.is_empty()).then_some(normalized)
        })
        .collect::<Vec<_>>();

    domains.sort();
    domains.dedup();
    domains
}

pub(crate) fn order_languages(
    mut supported_languages: Vec<Language>,
    default_language: &Language,
    default_from_env: bool,
) -> Vec<Language> {
    supported_languages.sort_by(|a, b| a.as_code().cmp(b.as_code()));

    let default_is_english = default_language.as_code() == "en";

    if supported_languages.len() > 1 && (default_from_env || default_is_english) {
        supported_languages.sort_by(|a, b| {
            use std::cmp::Ordering;

            if a == default_language {
                return Ordering::Less;
            }

            if b == default_language {
                return Ordering::Greater;
            }

            a.as_code().cmp(b.as_code())
        });
    }

    supported_languages
}

pub(crate) fn compile_static_assets(
    path: &str,
    site: SiteKind,
    data: &SiteData,
) -> AppResult<StaticAssets> {
    let context = build_static_context(site, data)?;

    Ok(StaticAssets {
        robots: render_static_template(path, "robots.txt", &context)?,
        sitemap: render_static_template(path, "sitemap.xml", &context)?,
        manifest: render_static_template(path, "site.webmanifest", &context)?,
        security_txt: render_static_template(path, ".well-known/security.txt", &context)?,
    })
}

fn build_static_context(site: SiteKind, data: &SiteData) -> AppResult<Context> {
    let main_site = data.sites.resolved_main();
    let cv_site = data.sites.resolved_cv();
    let site_data = match site {
        SiteKind::Main => (&main_site.domain, &main_site.title, &main_site.description),
        SiteKind::Cv => (&cv_site.domain, &cv_site.title, &cv_site.description),
    };

    let mut context = Context::new();
    context.insert("domain", site_data.0);
    context.insert("title", site_data.1);
    context.insert("description", site_data.2);
    context.insert("base_url", &format!("https://{}", site_data.0));
    let security_txt_expires = Utc
        .with_ymd_and_hms(Utc::now().year() + 2, 1, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| AppError::msg("invalid security.txt expiration date"))?
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    context.insert("security_txt_expires", &security_txt_expires);

    Ok(context)
}

fn render_static_template(path: &str, template: &str, context: &Context) -> AppResult<String> {
    let full_path = format!("{path}/{template}");
    let contents =
        std::fs::read_to_string(&full_path).with_context(|| format!("reading {full_path}"))?;

    let mut tera = Tera::default();
    tera.autoescape_on(vec![]);
    tera.add_raw_template(template, &contents)
        .with_context(|| format!("adding {full_path} as template"))?;

    tera.render(template, context).with_context(|| format!("rendering {full_path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn development_keeps_runtime_localization_refresh_enabled() {
        let state = app_state_for_environment(Environment::Development);

        assert!(state.should_refresh_localizations());
    }

    #[test]
    fn production_skips_runtime_localization_refresh() {
        let state = app_state_for_environment(Environment::Production);

        assert!(!state.should_refresh_localizations());
    }

    fn app_state_for_environment(environment: Environment) -> AppState {
        let localization_state = Arc::new(LocalizationState::new(
            Localizations::new(),
            Language::new("en"),
            vec![Language::new("en")],
            HashMap::new(),
        ));

        AppState {
            localization_state: Arc::new(RwLock::new(localization_state)),
            data_last_modified: Arc::new(RwLock::new(None)),
            tera: Arc::new(Tera::default()),
            main_static: StaticAssets {
                robots: String::new(),
                sitemap: String::new(),
                manifest: String::new(),
                security_txt: String::new(),
            },
            cv_static: StaticAssets {
                robots: String::new(),
                sitemap: String::new(),
                manifest: String::new(),
                security_txt: String::new(),
            },
            default_site: SiteKind::Main,
            environment,
            asset_version: String::new(),
            generator: String::new(),
            mailer: None,
            show_credits: true,
        }
    }
}
