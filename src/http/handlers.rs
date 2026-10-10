use std::{net::SocketAddr, sync::Arc};

use axum::{
    Form,
    body::Bytes,
    extract::{ConnectInfo, State},
    http::{
        HeaderMap, HeaderValue, StatusCode, Uri,
        header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE, HOST, ORIGIN},
    },
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use tracing::error;

use crate::app::{
    email::CvContactPayload,
    localization::{Language, Translations},
    render::{
        ContactFeedback, CvContactFormContext, render_cv_curriculum, render_cv_homepage,
        render_cv_menu, render_cv_pdf, render_error, render_main_homepage,
    },
    state::{AppState, SiteKind, SitePage},
};

struct ResolvedRequest {
    language: Language,
    site: SiteKind,
    path: String,
}

enum ResolveOutcome {
    Ready(ResolvedRequest),
    Respond(Response),
}

pub(crate) async fn homepage(
    State(state): State<AppState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let resolved = match resolve_request(&state, &headers, &uri, SitePage::Home, None) {
        ResolveOutcome::Ready(resolved) => resolved,
        ResolveOutcome::Respond(response) => return response,
    };

    match resolved.site {
        SiteKind::Main => render_main_homepage(&state, &resolved.language, &resolved.path)
            .map(Html)
            .map(IntoResponse::into_response),
        SiteKind::Cv => render_cv_homepage(&state, &resolved.language, &resolved.path)
            .map(Html)
            .map(IntoResponse::into_response),
    }
    .unwrap_or_else(|status| status.into_response())
}

pub(crate) async fn curriculum(
    State(state): State<AppState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let resolved =
        match resolve_request(&state, &headers, &uri, SitePage::Curriculum, Some(SiteKind::Cv)) {
            ResolveOutcome::Ready(resolved) => resolved,
            ResolveOutcome::Respond(response) => return response,
        };

    render_cv_curriculum(&state, None, &resolved.language, &resolved.path)
        .map(Html)
        .map(IntoResponse::into_response)
        .unwrap_or_else(|status| status.into_response())
}

pub(crate) async fn curriculum_pdf(
    State(state): State<AppState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let resolved = match resolve_request(
        &state,
        &headers,
        &uri,
        SitePage::CurriculumPdf,
        Some(SiteKind::Cv),
    ) {
        ResolveOutcome::Ready(resolved) => resolved,
        ResolveOutcome::Respond(response) => return response,
    };

    if let Some(bytes) = state.cached_cv_pdf(&resolved.language) {
        return pdf_response(&state, &resolved.language, bytes);
    }

    let render_lock = state.pdf_render_lock(&resolved.language);
    let _guard = render_lock.lock().await;

    if let Some(bytes) = state.cached_cv_pdf(&resolved.language) {
        return pdf_response(&state, &resolved.language, bytes);
    }

    let render_state = state.clone();
    let render_language = resolved.language.clone();
    let bytes =
        match tokio::task::spawn_blocking(move || render_cv_pdf(&render_state, &render_language))
            .await
        {
            Ok(Ok(bytes)) => Arc::new(bytes),
            Ok(Err(status)) => return status.into_response(),
            Err(error) => {
                error!(error = %error, "pdf render task failed");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            },
        };
    state.store_cv_pdf(resolved.language.clone(), Arc::clone(&bytes));
    pdf_response(&state, &resolved.language, bytes)
}

fn pdf_response(state: &AppState, language: &Language, bytes: Arc<Vec<u8>>) -> Response {
    let disposition = match state.resources_for(language) {
        Ok(resources) => crate::app::pdf::content_disposition(
            &resources.data.person.first_name,
            &resources.data.person.last_name,
        ),
        Err(error) => {
            error!(error = ?error, "Failed to load the CV name for the PDF filename");
            "attachment; filename=\"cv.pdf\"".to_string()
        },
    };
    let disposition = HeaderValue::from_str(&disposition)
        .unwrap_or_else(|_| HeaderValue::from_static("attachment; filename=\"cv.pdf\""));

    (
        [
            (CONTENT_TYPE, HeaderValue::from_static("application/pdf")),
            (CONTENT_DISPOSITION, disposition),
            (CACHE_CONTROL, HeaderValue::from_static("private, no-cache")),
        ],
        Bytes::from_owner(SharedBytes(bytes)),
    )
        .into_response()
}

struct SharedBytes<T>(T);

impl AsRef<[u8]> for SharedBytes<Arc<Vec<u8>>> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl AsRef<[u8]> for SharedBytes<Arc<str>> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CvContactForm {
    #[serde(rename = "full-name")]
    full_name: String,
    company: String,
    email: String,
    message: String,
}

const FULL_NAME_MIN: usize = 3;
const FULL_NAME_MAX: usize = 120;
const COMPANY_MIN: usize = 2;
const COMPANY_MAX: usize = 120;
const EMAIL_MIN: usize = 5;
const EMAIL_MAX: usize = 254;
const MESSAGE_MIN: usize = 10;
const MESSAGE_MAX: usize = 2_000;

pub(crate) async fn contact_cv(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    uri: Uri,
    Form(form): Form<CvContactForm>,
) -> Response {
    let resolved =
        match resolve_request(&state, &headers, &uri, SitePage::Contact, Some(SiteKind::Cv)) {
            ResolveOutcome::Ready(resolved) => resolved,
            ResolveOutcome::Respond(response) => return response,
        };

    if !request_origin_is_allowed(&headers, &state) {
        return StatusCode::FORBIDDEN.into_response();
    }

    if !state.contact_rate_limiter.check_and_record(addr.ip()) {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }

    let form_data = FormData {
        full_name: normalize_field(&form.full_name, FULL_NAME_MAX),
        company: normalize_field(&form.company, COMPANY_MAX),
        email: normalize_field(&form.email, EMAIL_MAX),
        message: normalize_field(&form.message, MESSAGE_MAX),
    };

    let translations = match state.resources_for(&resolved.language) {
        Ok(resources) => resources.translations,
        Err(error) => {
            error!(error = ?error, "Failed to load translations for contact form");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        },
    };

    if let Some(error_feedback) = validate_form(&form_data, &translations) {
        let context = CvContactFormContext {
            full_name: form_data.full_name.value,
            company: form_data.company.value,
            email: form_data.email.value,
            message: form_data.message.value,
            feedback: Some(error_feedback),
        };

        return render_cv_curriculum(&state, Some(context), &resolved.language, &resolved.path)
            .map(|html| (StatusCode::BAD_REQUEST, Html(html)).into_response())
            .unwrap_or_else(|status| status.into_response());
    }

    let payload = CvContactPayload {
        full_name: &form_data.full_name.value,
        company: &form_data.company.value,
        email: &form_data.email.value,
        message: &form_data.message.value,
    };

    let mailer = match &state.mailer {
        Some(mailer) => mailer,
        None => {
            return render_error(
                &state,
                SiteKind::Cv,
                StatusCode::NOT_FOUND,
                &resolved.path,
                &resolved.language,
            )
            .map(|html| (StatusCode::NOT_FOUND, Html(html)).into_response())
            .unwrap_or_else(|status| status.into_response());
        },
    };

    let to_override = state
        .resources_for(&resolved.language)
        .ok()
        .map(|resources| resources.data.contact.emails.resolved_cv().to_string());

    let (status, feedback) = match mailer.send_cv_contact(payload, to_override.as_deref()).await {
        Ok(_) => (
            StatusCode::OK,
            ContactFeedback {
                status: "success".to_string(),
                message: translations
                    .get_str(&["contact", "feedback_success"])
                    .unwrap_or_else(|| "Message sent successfully.".to_string()),
                field: None,
            },
        ),
        Err(error) => {
            error!(error = ?error, "Failed to send contact email");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                ContactFeedback {
                    status: "error".to_string(),
                    message: translations.get_str(&["contact", "feedback_error"]).unwrap_or_else(
                        || {
                            "There was an error sending the message. Please try again later."
                                .to_string()
                        },
                    ),
                    field: None,
                },
            )
        },
    };

    let context = CvContactFormContext {
        full_name: form_data.full_name.value,
        company: form_data.company.value,
        email: form_data.email.value,
        message: form_data.message.value,
        feedback: Some(feedback),
    };

    render_cv_curriculum(&state, Some(context), &resolved.language, &resolved.path)
        .map(|html| (status, Html(html)).into_response())
        .unwrap_or_else(|render_status| render_status.into_response())
}

pub(crate) async fn menu(State(state): State<AppState>, headers: HeaderMap, uri: Uri) -> Response {
    let resolved = match resolve_request(&state, &headers, &uri, SitePage::Menu, Some(SiteKind::Cv))
    {
        ResolveOutcome::Ready(resolved) => resolved,
        ResolveOutcome::Respond(response) => return response,
    };

    render_cv_menu(&state, &resolved.language, &resolved.path)
        .map(Html)
        .map(IntoResponse::into_response)
        .unwrap_or_else(|status| status.into_response())
}

pub(crate) async fn robots(
    State(state): State<AppState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let (language, _, _) = state.language_from_path(uri.path());

    let Some(site) = site_from_host(&headers, &state) else {
        return site_not_available(&state, uri.path(), &language);
    };
    let body = match site {
        SiteKind::Main => &state.main_static.robots,
        SiteKind::Cv => &state.cv_static.robots,
    };

    (
        [(CONTENT_TYPE, "text/plain; charset=utf-8")],
        Bytes::from_owner(SharedBytes(Arc::clone(body))),
    )
        .into_response()
}

pub(crate) async fn sitemap(
    State(state): State<AppState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let (language, _, _) = state.language_from_path(uri.path());

    let Some(site) = site_from_host(&headers, &state) else {
        return site_not_available(&state, uri.path(), &language);
    };
    let body = match site {
        SiteKind::Main => &state.main_static.sitemap,
        SiteKind::Cv => &state.cv_static.sitemap,
    };

    (
        [(CONTENT_TYPE, "application/xml; charset=utf-8")],
        Bytes::from_owner(SharedBytes(Arc::clone(body))),
    )
        .into_response()
}

pub(crate) async fn manifest(
    State(state): State<AppState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let (language, _, _) = state.language_from_path(uri.path());

    let Some(site) = site_from_host(&headers, &state) else {
        return site_not_available(&state, uri.path(), &language);
    };
    let body = match site {
        SiteKind::Main => &state.main_static.manifest,
        SiteKind::Cv => &state.cv_static.manifest,
    };

    (
        [(CONTENT_TYPE, "application/manifest+json")],
        Bytes::from_owner(SharedBytes(Arc::clone(body))),
    )
        .into_response()
}

pub(crate) async fn security_txt(
    State(state): State<AppState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let (language, _, _) = state.language_from_path(uri.path());

    let Some(site) = site_from_host(&headers, &state) else {
        return site_not_available(&state, uri.path(), &language);
    };
    let body = match site {
        SiteKind::Main => &state.main_static.security_txt,
        SiteKind::Cv => &state.cv_static.security_txt,
    };

    (
        [(CONTENT_TYPE, "text/plain; charset=utf-8")],
        Bytes::from_owner(SharedBytes(Arc::clone(body))),
    )
        .into_response()
}

pub(crate) async fn not_found(
    State(state): State<AppState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Some(redirect) = redirect_missing_language(&state, &uri) {
        return redirect;
    }

    not_found_response(&state, &headers, uri.path())
}

fn resolve_request(
    state: &AppState,
    headers: &HeaderMap,
    uri: &Uri,
    page: SitePage,
    required_site: Option<SiteKind>,
) -> ResolveOutcome {
    if let Some(response) = reject_unknown_page(state, headers, uri, page) {
        return ResolveOutcome::Respond(response);
    }

    if let Some(redirect) = redirect_missing_language(state, uri) {
        return ResolveOutcome::Respond(redirect);
    }

    let (language, _, _) = state.language_from_path(uri.path());

    let Some(site) = site_from_host(headers, state) else {
        return ResolveOutcome::Respond(site_not_available(state, uri.path(), &language));
    };

    if let Some(required_site) = required_site
        && site != required_site
    {
        return ResolveOutcome::Respond(
            render_error(state, SiteKind::Main, StatusCode::NOT_FOUND, uri.path(), &language)
                .map(|html| (StatusCode::NOT_FOUND, Html(html)).into_response())
                .unwrap_or_else(|status| status.into_response()),
        );
    }

    if !state.language_has_site(&language, site) {
        return ResolveOutcome::Respond(
            render_error(state, site, StatusCode::NOT_FOUND, uri.path(), &language)
                .map(|html| (StatusCode::NOT_FOUND, Html(html)).into_response())
                .unwrap_or_else(|status| status.into_response()),
        );
    }

    ResolveOutcome::Ready(ResolvedRequest { language, site, path: uri.path().to_string() })
}

fn reject_unknown_page(
    state: &AppState,
    headers: &HeaderMap,
    uri: &Uri,
    page: SitePage,
) -> Option<Response> {
    if state.path_matches_page(uri.path(), page) {
        return None;
    }

    Some(not_found_response(state, headers, uri.path()))
}

fn not_found_response(state: &AppState, headers: &HeaderMap, path: &str) -> Response {
    let (language, _, _) = state.language_from_path(path);
    let site = site_from_host(headers, state).or_else(|| fallback_site(state));

    match site {
        Some(site) => render_error(state, site, StatusCode::NOT_FOUND, path, &language)
            .map(|html| (StatusCode::NOT_FOUND, Html(html)).into_response())
            .unwrap_or_else(|status| status.into_response()),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

struct NormalizedField {
    value: String,
    len: usize,
}

struct FormData {
    full_name: NormalizedField,
    company: NormalizedField,
    email: NormalizedField,
    message: NormalizedField,
}

fn normalize_field(value: &str, max_len: usize) -> NormalizedField {
    let trimmed = value.trim();
    let len = trimmed.chars().count();
    let value = trimmed.chars().take(max_len).collect();

    NormalizedField { value, len }
}

fn field_feedback(
    translations: &Translations,
    field: &str,
    key: &str,
    fallback: &str,
) -> ContactFeedback {
    ContactFeedback {
        status: "error".to_string(),
        message: translations.get_str(&["validation", key]).unwrap_or_else(|| fallback.to_string()),
        field: Some(field.to_string()),
    }
}

fn validate_form(form: &FormData, translations: &Translations) -> Option<ContactFeedback> {
    if form.full_name.value.is_empty() {
        return Some(field_feedback(
            translations,
            "full-name",
            "full_name_required",
            "Enter your full name.",
        ));
    }

    if !(FULL_NAME_MIN..=FULL_NAME_MAX).contains(&form.full_name.len) {
        return Some(field_feedback(
            translations,
            "full-name",
            "full_name_length",
            "Full name must be between 3 and 120 characters.",
        ));
    }

    if form.company.value.is_empty() {
        return Some(field_feedback(
            translations,
            "company",
            "company_required",
            "Enter the company name.",
        ));
    }

    if !(COMPANY_MIN..=COMPANY_MAX).contains(&form.company.len) {
        return Some(field_feedback(
            translations,
            "company",
            "company_length",
            "Company name must be between 2 and 120 characters.",
        ));
    }

    if form.email.value.is_empty() {
        return Some(field_feedback(
            translations,
            "email",
            "email_required",
            "Enter your email address.",
        ));
    }

    if !(EMAIL_MIN..=EMAIL_MAX).contains(&form.email.len) || !email_is_valid(&form.email.value) {
        return Some(field_feedback(
            translations,
            "email",
            "email",
            "Please enter a valid email address.",
        ));
    }

    if form.message.value.is_empty() {
        return Some(field_feedback(
            translations,
            "message",
            "message_required",
            "Enter a message.",
        ));
    }

    if !(MESSAGE_MIN..=MESSAGE_MAX).contains(&form.message.len) {
        return Some(field_feedback(
            translations,
            "message",
            "message_length",
            "Message must be between 10 and 2000 characters.",
        ));
    }

    None
}

#[cfg(feature = "smtp")]
fn email_is_valid(email: &str) -> bool {
    email.parse::<lettre::message::Mailbox>().is_ok()
}

#[cfg(not(feature = "smtp"))]
fn email_is_valid(email: &str) -> bool {
    email.contains('@') && email.contains('.')
}

fn redirect_missing_language(state: &AppState, uri: &Uri) -> Option<Response> {
    state.redirect_path_for_missing_language(uri.path()).map(|mut target| {
        if let Some(query) = uri.query() {
            target.push('?');
            target.push_str(query);
        }

        Redirect::temporary(&target).into_response()
    })
}

fn site_from_host(headers: &HeaderMap, state: &AppState) -> Option<SiteKind> {
    let snapshot = state.localization_snapshot();
    let host = headers
        .get(HOST)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(':').next())
        .unwrap_or_default();

    if snapshot.cv_available && domain_list_contains(&snapshot.cv_domains, host) {
        return Some(SiteKind::Cv);
    }

    if snapshot.main_available && domain_list_contains(&snapshot.main_domains, host) {
        return Some(SiteKind::Main);
    }

    fallback_site(state)
}

fn domain_list_contains(domains: &[String], host: &str) -> bool {
    !host.is_empty() && domains.iter().any(|domain| domain.eq_ignore_ascii_case(host))
}

fn request_origin_is_allowed(headers: &HeaderMap, state: &AppState) -> bool {
    if let Some(sec_fetch_site) =
        headers.get("sec-fetch-site").and_then(|value| value.to_str().ok())
        && sec_fetch_site.eq_ignore_ascii_case("cross-site")
    {
        return false;
    }

    let Some(origin) = headers.get(ORIGIN) else {
        return false;
    };

    let Some(origin) = origin.to_str().ok() else {
        return false;
    };
    let Some(origin_host) = host_from_origin(origin) else {
        return false;
    };

    let request_host =
        headers.get(HOST).and_then(|value| value.to_str().ok()).and_then(normalize_host_header);

    if request_host.as_deref() == Some(origin_host.as_str()) {
        return true;
    }

    let snapshot = state.localization_snapshot();
    snapshot.main_domains.contains(&origin_host) || snapshot.cv_domains.contains(&origin_host)
}

fn host_from_origin(origin: &str) -> Option<String> {
    let (_, remainder) = origin.split_once("://")?;

    if remainder.is_empty() || remainder.contains('/') || remainder.contains('?') {
        return None;
    }

    normalize_host_header(remainder)
}

fn normalize_host_header(value: &str) -> Option<String> {
    let host = value
        .trim()
        .trim_start_matches('[')
        .split(']')
        .next()
        .unwrap_or_default()
        .split(':')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();

    (!host.is_empty()).then_some(host)
}

fn site_not_available(
    state: &AppState,
    page_url: &str,
    language: &crate::app::localization::Language,
) -> Response {
    if let Some(site) = fallback_site(state) {
        return render_error(state, site, StatusCode::NOT_FOUND, page_url, language)
            .map(|html| (StatusCode::NOT_FOUND, Html(html)).into_response())
            .unwrap_or_else(|status| status.into_response());
    }

    StatusCode::NOT_FOUND.into_response()
}

fn fallback_site(state: &AppState) -> Option<SiteKind> {
    if state.site_available(state.default_site) {
        return Some(state.default_site);
    }

    if state.site_available(SiteKind::Main) {
        return Some(SiteKind::Main);
    }

    if state.site_available(SiteKind::Cv) {
        return Some(SiteKind::Cv);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{
        environment::Environment,
        localization::{Language, Localizations},
        rate_limit::ContactRateLimiter,
        state::{LocalizationState, StaticAssets},
    };
    use std::sync::{Arc, Mutex, RwLock};
    use tera::Tera;

    #[test]
    fn same_host_origin_is_allowed() {
        let headers = headers_with("cv.example.com:36547", "https://cv.example.com");

        assert!(request_origin_is_allowed(&headers, &app_state()));
    }

    #[test]
    fn configured_site_origin_is_allowed() {
        let headers = headers_with("127.0.0.1:36547", "https://cv.example.com");

        assert!(request_origin_is_allowed(&headers, &app_state()));
    }

    #[test]
    fn malformed_origin_is_rejected() {
        let headers = headers_with("cv.example.com", "not-a-valid-origin");

        assert!(!request_origin_is_allowed(&headers, &app_state()));
    }

    #[test]
    fn missing_origin_is_rejected() {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, "cv.example.com".parse().unwrap());

        assert!(!request_origin_is_allowed(&headers, &app_state()));
    }

    #[test]
    fn cross_site_sec_fetch_is_rejected() {
        let mut headers = headers_with("cv.example.com", "https://cv.example.com");
        headers.insert("sec-fetch-site", "cross-site".parse().unwrap());

        assert!(!request_origin_is_allowed(&headers, &app_state()));
    }

    #[test]
    fn mixed_case_host_matches_configured_domain() {
        let headers = headers_with("CV.Example.com:36547", "https://cv.example.com");

        assert!(request_origin_is_allowed(&headers, &app_state()));
        assert_eq!(site_from_host(&headers, &app_state()), Some(SiteKind::Cv));
    }

    #[test]
    fn cross_site_origin_is_rejected() {
        let headers = headers_with("cv.example.com", "https://attacker.example");

        assert!(!request_origin_is_allowed(&headers, &app_state()));
    }

    fn empty_static_assets() -> StaticAssets {
        StaticAssets {
            robots: Arc::from(""),
            sitemap: Arc::from(""),
            manifest: Arc::from(""),
            security_txt: Arc::from(""),
        }
    }

    fn headers_with(host: &str, origin: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, host.parse().unwrap());
        headers.insert(ORIGIN, origin.parse().unwrap());
        headers
    }

    fn app_state() -> AppState {
        let localization_state = Arc::new(LocalizationState {
            localizations: Localizations::new(),
            default_language: Language::new("en"),
            supported_languages: vec![Language::new("en")],
            language_names: std::collections::HashMap::new(),
            main_domains: vec!["main.example.com".to_string()],
            cv_domains: vec!["cv.example.com".to_string()],
            main_available: true,
            cv_available: true,
        });

        AppState {
            localization_state: Arc::new(RwLock::new(localization_state)),
            data_last_modified: Arc::new(RwLock::new(None)),
            localization_checked_at: Arc::new(Mutex::new(None)),
            tera: Arc::new(Tera::default()),
            main_static: Arc::new(empty_static_assets()),
            cv_static: Arc::new(empty_static_assets()),
            default_site: SiteKind::Cv,
            environment: Environment::Production,
            asset_version: Arc::from(""),
            generator: Arc::from(""),
            mailer: None,
            show_credits: true,
            cv_pdf_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            cv_pdf_locks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            html_page_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            contact_rate_limiter: Arc::new(ContactRateLimiter::default()),
        }
    }
}
