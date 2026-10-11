use std::{
    collections::HashMap,
    fmt::Write as _,
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant, SystemTime},
};

use chrono::{Datelike, SecondsFormat, TimeZone, Utc};
use serde::Serialize;
use tera::{Context, Tera};
use tracing::{error, warn};

use crate::{
    app::{
        email::Mailer,
        environment::Environment,
        localization::{
            Language, Localizations, LocalizedResources, detect_languages_with_last_change,
            load_language_names, load_localizations, match_accept_language,
        },
        models::SiteData,
        rate_limit::ContactRateLimiter,
    },
    error::{AppError, AppResult, ResultExt},
};

const LOCALIZATION_RECHECK_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct AppState {
    pub(crate) localization_state: Arc<RwLock<Arc<LocalizationState>>>,
    pub(crate) data_last_modified: Arc<RwLock<Option<SystemTime>>>,
    pub(crate) localization_checked_at: Arc<Mutex<Option<Instant>>>,
    pub(crate) tera: Arc<Tera>,
    pub(crate) main_static: Arc<StaticAssets>,
    pub(crate) cv_static: Arc<StaticAssets>,
    pub(crate) default_site: SiteKind,
    pub(crate) environment: Environment,
    pub(crate) asset_version: Arc<str>,
    pub(crate) generator: Arc<str>,
    pub(crate) mailer: Option<Arc<Mailer>>,
    pub(crate) show_credits: bool,
    pub(crate) cv_pdf_cache: Arc<RwLock<HashMap<Language, Arc<Vec<u8>>>>>,
    pub(crate) cv_pdf_locks: Arc<Mutex<HashMap<Language, Arc<tokio::sync::Mutex<()>>>>>,
    pub(crate) html_page_cache: Arc<RwLock<HashMap<HtmlCacheKey, Arc<str>>>>,
    pub(crate) contact_rate_limiter: Arc<ContactRateLimiter>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct HtmlCacheKey {
    pub site: SiteKind,
    pub language: Language,
    pub template: &'static str,
    pub request_path: String,
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
    pub robots: Arc<str>,
    pub sitemap: Arc<str>,
    pub manifest: Arc<str>,
    pub security_txt: Arc<str>,
}

#[derive(Serialize)]
pub(crate) struct LanguageLink {
    pub code: String,
    pub label: String,
    pub url: String,
    pub is_current: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SiteKind {
    Main,
    Cv,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SitePage {
    Home,
    Curriculum,
    CurriculumPdf,
    Menu,
    Languages,
    Contact,
}

impl SitePage {
    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Home => "",
            Self::Curriculum => "curriculum",
            Self::CurriculumPdf => "curriculum.pdf",
            Self::Menu => "menu",
            Self::Languages => "languages",
            Self::Contact => "contact",
        }
    }
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
        language_from_path_in_snapshot(path, &self.localization_snapshot())
    }

    pub(crate) fn language_has_site(&self, language: &Language, site: SiteKind) -> bool {
        let snapshot = self.localization_snapshot();

        language_has_site_in_snapshot(language, site, &snapshot)
    }

    pub(crate) fn site_available(&self, site: SiteKind) -> bool {
        let snapshot = self.localization_snapshot();

        site_available_in_snapshot(site, &snapshot)
    }

    pub(crate) fn path_matches_page(&self, path: &str, page: SitePage) -> bool {
        let snapshot = self.localization_snapshot();

        path_matches_page(path, page, &snapshot.supported_languages)
    }

    pub(crate) fn redirect_path_for_missing_language(
        &self,
        path: &str,
        accept_language: Option<&str>,
        site: Option<SiteKind>,
    ) -> Option<String> {
        let snapshot = self.localization_snapshot();
        let clean = path.trim_matches('/');

        if snapshot.supported_languages.len() == 1 {
            let language = snapshot.supported_languages.first()?;
            let (prefix, suffix) = split_language_prefix(clean);

            if prefix == Some(language.as_code()) && is_known_page_suffix(suffix) {
                return Some(page_path(suffix));
            }

            return None;
        }

        let preferred = self.preferred_language(accept_language, site);

        if clean.is_empty() {
            return Some(format!("/{}", preferred.as_code()));
        }

        let (prefix, _) = split_language_prefix(clean);
        if prefix.is_some_and(|prefix| language_code_is_supported(prefix, &snapshot)) {
            return None;
        }

        is_known_page_suffix(clean).then(|| format!("/{}/{}", preferred.as_code(), clean))
    }

    /// Prefer browser `Accept-Language`, then `DEFAULT_LANGUAGE` / configured default,
    /// then the existing English-or-first fallback already stored as `default_language`.
    pub(crate) fn preferred_language(
        &self,
        accept_language: Option<&str>,
        site: Option<SiteKind>,
    ) -> Language {
        let snapshot = self.localization_snapshot();
        preferred_language_in_snapshot(&snapshot, accept_language, site)
    }

    pub(crate) fn localization_snapshot(&self) -> Arc<LocalizationState> {
        if self.should_refresh_localizations()
            && let Err(error) = self.try_refresh_localizations()
        {
            error!(error = ?error, "failed to refresh localization data");
        }

        self.localization_state.read().unwrap_or_else(|error| error.into_inner()).clone()
    }

    fn should_refresh_localizations(&self) -> bool {
        matches!(self.environment, Environment::Development)
    }

    fn localization_check_is_fresh(&self) -> bool {
        self.localization_checked_at
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .is_some_and(|checked_at| checked_at.elapsed() < LOCALIZATION_RECHECK_INTERVAL)
    }

    fn mark_localization_checked(&self) {
        *self.localization_checked_at.lock().unwrap_or_else(|error| error.into_inner()) =
            Some(Instant::now());
    }

    fn try_refresh_localizations(&self) -> AppResult<()> {
        if self.localization_check_is_fresh() {
            return Ok(());
        }

        let (available_languages, latest_change) = detect_languages_with_last_change()?;

        {
            let current_state =
                self.localization_state.read().unwrap_or_else(|error| error.into_inner());

            let languages_unchanged = available_languages == current_state.supported_languages;
            let timestamp_unchanged = latest_change
                == *self.data_last_modified.read().unwrap_or_else(|error| error.into_inner());

            if languages_unchanged && timestamp_unchanged {
                self.mark_localization_checked();
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
        self.cv_pdf_cache.write().unwrap_or_else(|error| error.into_inner()).clear();
        self.html_page_cache.write().unwrap_or_else(|error| error.into_inner()).clear();
        self.mark_localization_checked();

        Ok(())
    }

    pub(crate) fn cached_cv_pdf(&self, language: &Language) -> Option<Arc<Vec<u8>>> {
        let _ = self.localization_snapshot();
        self.cv_pdf_cache.read().unwrap_or_else(|error| error.into_inner()).get(language).cloned()
    }

    pub(crate) fn store_cv_pdf(&self, language: Language, bytes: Arc<Vec<u8>>) {
        self.cv_pdf_cache
            .write()
            .unwrap_or_else(|error| error.into_inner())
            .insert(language, bytes);
    }

    pub(crate) fn pdf_render_lock(&self, language: &Language) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = self.cv_pdf_locks.lock().unwrap_or_else(|error| error.into_inner());
        locks
            .entry(language.clone())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    pub(crate) fn cached_html_page(&self, key: &HtmlCacheKey) -> Option<Arc<str>> {
        if !matches!(self.environment, Environment::Production) {
            return None;
        }

        self.html_page_cache.read().unwrap_or_else(|error| error.into_inner()).get(key).cloned()
    }

    pub(crate) fn store_html_page(&self, key: HtmlCacheKey, html: Arc<str>) {
        if !matches!(self.environment, Environment::Production) {
            return;
        }

        self.html_page_cache.write().unwrap_or_else(|error| error.into_inner()).insert(key, html);
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

    pub(crate) fn language_has_site(&self, language: &Language, site: SiteKind) -> bool {
        language_has_site_in_snapshot(language, site, self)
    }

    pub(crate) fn language_links(
        &self,
        current_language: &Language,
        path_suffix: &str,
        site: SiteKind,
    ) -> Vec<LanguageLink> {
        language_links_in_snapshot(self, current_language, path_suffix, site)
    }
}

pub(crate) fn language_from_path_in_snapshot(
    path: &str,
    snapshot: &LocalizationState,
) -> (Language, String, String) {
    let clean_path = path.trim_matches('/');

    if snapshot.supported_languages.len() == 1 {
        let language = snapshot.supported_languages.first().cloned().unwrap_or_else(|| {
            warn!("supported languages missing; defaulting to English");
            Language::new("en")
        });

        let mut segments = clean_path.split('/');

        if let Some(first_segment) = segments.next()
            && first_segment == language.as_code()
        {
            return (language, rooted_segments(segments), String::new());
        }

        return (language, rooted_path(clean_path), String::new());
    }

    if clean_path.is_empty() {
        return (snapshot.default_language.clone(), "/".to_string(), String::new());
    }

    let mut segments = clean_path.split('/');
    if let Some(first_segment) = segments.next()
        && let Some(language) = Language::from_code(first_segment, &snapshot.supported_languages)
    {
        return (language, rooted_segments(segments), format!("/{first_segment}"));
    }

    (snapshot.default_language.clone(), rooted_path(clean_path), String::new())
}

fn preferred_language_in_snapshot(
    snapshot: &LocalizationState,
    accept_language: Option<&str>,
    site: Option<SiteKind>,
) -> Language {
    let available: Vec<Language> = snapshot
        .supported_languages
        .iter()
        .filter(|language| match site {
            Some(site) => language_has_site_in_snapshot(language, site, snapshot),
            None => true,
        })
        .cloned()
        .collect();

    if let Some(header) = accept_language
        && let Some(matched) = match_accept_language(header, &available)
    {
        return matched;
    }

    if available.iter().any(|language| language == &snapshot.default_language) {
        return snapshot.default_language.clone();
    }

    available.first().cloned().unwrap_or_else(|| snapshot.default_language.clone())
}

fn language_links_in_snapshot(
    snapshot: &LocalizationState,
    current_language: &Language,
    path_suffix: &str,
    site: SiteKind,
) -> Vec<LanguageLink> {
    let available_languages: Vec<_> = snapshot
        .supported_languages
        .iter()
        .filter(|language| language_has_site_in_snapshot(language, site, snapshot))
        .collect();

    if available_languages.len() <= 1 {
        return Vec::new();
    }

    available_languages
        .into_iter()
        .map(|language| {
            let mut url = language.url_prefix(&snapshot.default_language, true);
            url.push_str(path_suffix);

            LanguageLink {
                code: language.as_code().to_string(),
                label: snapshot
                    .language_names
                    .get(language)
                    .cloned()
                    .unwrap_or_else(|| language.as_code().to_string()),
                url,
                is_current: language == current_language,
            }
        })
        .collect()
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
    localization: &LocalizationState,
) -> AppResult<StaticAssets> {
    let context = build_static_context(site, data, localization)?;
    let mut tera = Tera::default();
    tera.autoescape_on(Vec::<&str>::new());

    Ok(StaticAssets {
        robots: render_static_template(&mut tera, path, "robots.txt", &context)?,
        sitemap: Arc::from(render_sitemap(site, data, localization)?),
        manifest: render_static_template(&mut tera, path, "site.webmanifest", &context)?,
        security_txt: render_static_template(
            &mut tera,
            path,
            ".well-known/security.txt",
            &context,
        )?,
    })
}

fn build_static_context(
    site: SiteKind,
    data: &SiteData,
    localization: &LocalizationState,
) -> AppResult<Context> {
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
    context.insert("lang", localization.default_language.as_code());
    context.insert("base_url", &format!("https://{}", site_data.0));
    let security_txt_expires = Utc
        .with_ymd_and_hms(Utc::now().year().saturating_add(2), 1, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| AppError::msg("invalid security.txt expiration date"))?
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    context.insert("security_txt_expires", &security_txt_expires);

    Ok(context)
}

fn render_sitemap(
    site: SiteKind,
    data: &SiteData,
    localization: &LocalizationState,
) -> AppResult<String> {
    let domain = match site {
        SiteKind::Main => data.sites.resolved_main().domain.as_str(),
        SiteKind::Cv => data.sites.resolved_cv().domain.as_str(),
    };
    let base_url = format!("https://{domain}");
    let pages: &[(&str, &str)] = match site {
        SiteKind::Main => &[("", "1.00")],
        SiteKind::Cv => &[("", "1.00"), ("curriculum", "0.90")],
    };

    let languages: Vec<&Language> = localization
        .supported_languages
        .iter()
        .filter(|language| language_has_site_in_snapshot(language, site, localization))
        .collect();

    let multilingual = languages.len() > 1;
    // Header + roughly 200 bytes per URL entry (with hreflang alternates when multilingual).
    let estimated_entries = pages.len().saturating_mul(languages.len().max(1));
    let mut output = String::with_capacity(160 + estimated_entries.saturating_mul(220));
    output.push_str(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">
"#,
    );

    for (page, priority) in pages {
        for language in &languages {
            let path = language_prefixed_path(
                language,
                page,
                &localization.default_language,
                multilingual,
            );
            let _ = writeln!(
                output,
                "  <url>\n    <loc>{base_url}{path}</loc>\n    <priority>{priority}</priority>"
            );

            if multilingual {
                for alternate in &languages {
                    let alternate_path = language_prefixed_path(
                        alternate,
                        page,
                        &localization.default_language,
                        true,
                    );
                    let _ = writeln!(
                        output,
                        "    <xhtml:link rel=\"alternate\" hreflang=\"{}\" href=\"{base_url}{alternate_path}\" />",
                        alternate.as_code()
                    );
                }
                let default_path = language_prefixed_path(
                    &localization.default_language,
                    page,
                    &localization.default_language,
                    true,
                );
                let _ = writeln!(
                    output,
                    "    <xhtml:link rel=\"alternate\" hreflang=\"x-default\" href=\"{base_url}{default_path}\" />"
                );
            }

            output.push_str("  </url>\n");
        }
    }

    output.push_str("</urlset>\n");
    Ok(output)
}

fn language_prefixed_path(
    language: &Language,
    page: &str,
    default_language: &Language,
    multilingual: bool,
) -> String {
    let prefix = if multilingual {
        format!("/{}", language.as_code())
    } else if language == default_language {
        String::new()
    } else {
        format!("/{}", language.as_code())
    };

    if page.is_empty() {
        if prefix.is_empty() { "/".to_string() } else { format!("{prefix}/") }
    } else if prefix.is_empty() {
        format!("/{page}")
    } else {
        format!("{prefix}/{page}")
    }
}

fn render_static_template(
    tera: &mut Tera,
    path: &str,
    template: &str,
    context: &Context,
) -> AppResult<Arc<str>> {
    let full_path = format!("{path}/{template}");
    let contents =
        std::fs::read_to_string(&full_path).with_context(|| format!("reading {full_path}"))?;

    tera.add_raw_template(template, &contents)
        .with_context(|| format!("adding {full_path} as template"))?;

    tera.render(template, context).with_context(|| format!("rendering {full_path}")).map(Arc::from)
}

fn path_matches_page(path: &str, page: SitePage, languages: &[Language]) -> bool {
    let clean = path.trim_matches('/');
    let slug = page.slug();

    if clean == slug {
        return true;
    }

    let Some((prefix, rest)) = clean.split_once('/') else {
        return page == SitePage::Home
            && languages.iter().any(|language| language.as_code() == clean);
    };

    languages.iter().any(|language| language.as_code() == prefix)
        && rest.trim_end_matches('/') == slug
}

fn language_code_is_supported(code: &str, snapshot: &LocalizationState) -> bool {
    snapshot.supported_languages.iter().any(|language| language.as_code() == code)
}

fn is_known_page_suffix(suffix: &str) -> bool {
    [
        SitePage::Home,
        SitePage::Curriculum,
        SitePage::CurriculumPdf,
        SitePage::Menu,
        SitePage::Languages,
        SitePage::Contact,
    ]
    .into_iter()
    .any(|page| page.slug() == suffix)
}

fn split_language_prefix(clean: &str) -> (Option<&str>, &str) {
    if clean.is_empty() {
        return (None, "");
    }

    match clean.split_once('/') {
        Some((prefix, rest)) => (Some(prefix), rest.trim_end_matches('/')),
        None => (Some(clean), ""),
    }
}

fn page_path(suffix: &str) -> String {
    if suffix.is_empty() { "/".to_string() } else { format!("/{suffix}") }
}

fn rooted_path(path: &str) -> String {
    if path.is_empty() {
        return "/".to_string();
    }

    let mut rooted = String::with_capacity(path.len() + 1);
    rooted.push('/');
    rooted.push_str(path);
    rooted
}

fn rooted_segments<'a>(segments: impl Iterator<Item = &'a str>) -> String {
    let mut rooted = String::from("/");
    let mut has_segment = false;

    for segment in segments {
        if has_segment {
            rooted.push('/');
        }

        rooted.push_str(segment);
        has_segment = true;
    }

    rooted
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

    #[test]
    fn unknown_single_segment_is_not_the_homepage() {
        let state = app_state_with_languages(&["it"]);

        assert!(!state.path_matches_page("/asdas", SitePage::Home));
        assert!(!state.path_matches_page("/asdas/", SitePage::Home));
        assert!(state.redirect_path_for_missing_language("/asdas", None, None).is_none());
    }

    #[test]
    fn unknown_language_prefix_does_not_match_a_real_page() {
        let state = app_state_with_languages(&["it"]);

        assert!(!state.path_matches_page("/en/curriculum", SitePage::Curriculum));
        assert!(!state.path_matches_page("/foo/menu", SitePage::Menu));
        assert!(state.redirect_path_for_missing_language("/it/asdas", None, None).is_none());
    }

    #[test]
    fn single_language_prefix_redirects_only_known_pages() {
        let state = app_state_with_languages(&["it"]);

        assert_eq!(
            state.redirect_path_for_missing_language("/it", None, None),
            Some("/".to_string())
        );
        assert_eq!(
            state.redirect_path_for_missing_language("/it/curriculum", None, None),
            Some("/curriculum".to_string())
        );
        assert!(state.path_matches_page("/", SitePage::Home));
        assert!(state.path_matches_page("/curriculum", SitePage::Curriculum));
        assert!(state.path_matches_page("/it/menu/", SitePage::Menu));
        assert!(state.path_matches_page("/it/languages/", SitePage::Languages));
        assert!(state.path_matches_page("/languages/", SitePage::Languages));
    }

    #[test]
    fn multilingual_unknown_path_is_not_rewritten_to_the_homepage() {
        let state = app_state_with_languages(&["en", "it"]);

        assert!(state.redirect_path_for_missing_language("/asdas", None, None).is_none());
        assert!(state.redirect_path_for_missing_language("/zz/curriculum", None, None).is_none());
        assert_eq!(
            state.redirect_path_for_missing_language("/", None, None),
            Some("/en".to_string())
        );
        assert_eq!(
            state.redirect_path_for_missing_language("/curriculum", None, None),
            Some("/en/curriculum".to_string())
        );
        assert!(state.redirect_path_for_missing_language("/en/curriculum", None, None).is_none());
        assert!(state.path_matches_page("/en", SitePage::Home));
        assert!(state.path_matches_page("/it/curriculum", SitePage::Curriculum));
        assert!(state.path_matches_page("/en/languages", SitePage::Languages));
        assert_eq!(
            state.redirect_path_for_missing_language("/languages", None, None),
            Some("/en/languages".to_string())
        );
        assert_eq!(
            state.redirect_path_for_missing_language("/", Some("it-IT,it;q=0.9,en;q=0.8"), None),
            Some("/it".to_string())
        );
        assert_eq!(
            state.redirect_path_for_missing_language("/curriculum", Some("fr-FR,fr;q=0.9"), None),
            Some("/en/curriculum".to_string())
        );
    }

    fn empty_static_assets() -> StaticAssets {
        StaticAssets {
            robots: Arc::from(""),
            sitemap: Arc::from(""),
            manifest: Arc::from(""),
            security_txt: Arc::from(""),
        }
    }

    fn app_state_with_languages(codes: &[&str]) -> AppState {
        let supported_languages = codes.iter().copied().map(Language::new).collect::<Vec<_>>();
        let default_language =
            supported_languages.first().cloned().unwrap_or_else(|| Language::new("en"));
        let localization_state = Arc::new(LocalizationState::new(
            Localizations::new(),
            default_language,
            supported_languages,
            HashMap::new(),
        ));

        AppState {
            localization_state: Arc::new(RwLock::new(localization_state)),
            data_last_modified: Arc::new(RwLock::new(None)),
            localization_checked_at: Arc::new(Mutex::new(None)),
            tera: Arc::new(Tera::default()),
            main_static: Arc::new(empty_static_assets()),
            cv_static: Arc::new(empty_static_assets()),
            default_site: SiteKind::Main,
            environment: Environment::Production,
            asset_version: Arc::from(""),
            generator: Arc::from(""),
            mailer: None,
            show_credits: true,
            cv_pdf_cache: Arc::new(RwLock::new(HashMap::new())),
            cv_pdf_locks: Arc::new(Mutex::new(HashMap::new())),
            html_page_cache: Arc::new(RwLock::new(HashMap::new())),
            contact_rate_limiter: Arc::new(ContactRateLimiter::default()),
        }
    }

    fn app_state_for_environment(environment: Environment) -> AppState {
        let mut state = app_state_with_languages(&["en"]);
        state.environment = environment;
        state
    }
}
