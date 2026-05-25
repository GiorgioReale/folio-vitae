use axum::{
    Form,
    extract::State,
    http::{
        HeaderMap, StatusCode, Uri,
        header::{CONTENT_TYPE, HOST, ORIGIN},
    },
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use tracing::error;

use crate::app::{
    email::CvContactPayload,
    localization::Translations,
    render::{
        ContactFeedback, CvContactFormContext, render_cv_curriculum, render_cv_homepage,
        render_cv_menu, render_error, render_main_homepage,
    },
    state::{AppState, SiteKind},
};

pub(crate) async fn homepage(
    State(state): State<AppState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Some(redirect) = redirect_missing_language(&state, &uri) {
        return redirect;
    }

    let (language, _, _) = state.language_from_path(uri.path());

    let Some(site) = site_from_host(&headers, &state) else {
        return site_not_available(&state, uri.path(), &language);
    };
    if !state.language_has_site(&language, site) {
        return render_error(&state, site, StatusCode::NOT_FOUND, uri.path(), &language)
            .map(|html| (StatusCode::NOT_FOUND, Html(html)).into_response())
            .unwrap_or_else(|status| status.into_response());
    }
    match site {
        SiteKind::Main => render_main_homepage(&state, &language, uri.path())
            .map(Html)
            .map(IntoResponse::into_response),
        SiteKind::Cv => render_cv_homepage(&state, &language, uri.path())
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
    if let Some(redirect) = redirect_missing_language(&state, &uri) {
        return redirect;
    }

    let (language, _, _) = state.language_from_path(uri.path());

    let Some(site) = site_from_host(&headers, &state) else {
        return site_not_available(&state, uri.path(), &language);
    };

    if site != SiteKind::Cv {
        return render_error(&state, SiteKind::Main, StatusCode::NOT_FOUND, uri.path(), &language)
            .map(|html| (StatusCode::NOT_FOUND, Html(html)).into_response())
            .unwrap_or_else(|status| status.into_response());
    }

    if !state.language_has_site(&language, SiteKind::Cv) {
        return render_error(&state, SiteKind::Cv, StatusCode::NOT_FOUND, uri.path(), &language)
            .map(|html| (StatusCode::NOT_FOUND, Html(html)).into_response())
            .unwrap_or_else(|status| status.into_response());
    }

    render_cv_curriculum(&state, None, &language, uri.path())
        .map(Html)
        .map(IntoResponse::into_response)
        .unwrap_or_else(|status| status.into_response())
}

#[derive(Deserialize)]
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
    headers: HeaderMap,
    uri: Uri,
    Form(form): Form<CvContactForm>,
) -> Response {
    if let Some(redirect) = redirect_missing_language(&state, &uri) {
        return redirect;
    }

    let (language, _, _) = state.language_from_path(uri.path());

    let Some(site) = site_from_host(&headers, &state) else {
        return site_not_available(&state, uri.path(), &language);
    };

    if site != SiteKind::Cv {
        return render_error(&state, SiteKind::Main, StatusCode::NOT_FOUND, uri.path(), &language)
            .map(|html| (StatusCode::NOT_FOUND, Html(html)).into_response())
            .unwrap_or_else(|status| status.into_response());
    }

    if !state.language_has_site(&language, SiteKind::Cv) {
        return render_error(&state, SiteKind::Cv, StatusCode::NOT_FOUND, uri.path(), &language)
            .map(|html| (StatusCode::NOT_FOUND, Html(html)).into_response())
            .unwrap_or_else(|status| status.into_response());
    }

    if !request_origin_is_allowed(&headers, &state) {
        return StatusCode::FORBIDDEN.into_response();
    }

    let form_data = FormData {
        full_name: normalize_field(&form.full_name, FULL_NAME_MAX),
        company: normalize_field(&form.company, COMPANY_MAX),
        email: normalize_field(&form.email, EMAIL_MAX),
        message: normalize_field(&form.message, MESSAGE_MAX),
    };

    let translations = match state.resources_for(&language) {
        Ok(resources) => resources.translations,
        Err(error) => {
            error!(error = ?error, "Failed to load translations for contact form");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        },
    };

    if let Some(error_feedback) = validate_form(&form_data, &translations) {
        let context = CvContactFormContext {
            full_name: form_data.full_name.value.clone(),
            company: form_data.company.value.clone(),
            email: form_data.email.value.clone(),
            message: form_data.message.value.clone(),
            feedback: Some(error_feedback),
        };

        return render_cv_curriculum(&state, Some(context), &language, uri.path())
            .map(|html| (StatusCode::BAD_REQUEST, Html(html)).into_response())
            .unwrap_or_else(|status| status.into_response());
    }

    let payload = CvContactPayload {
        full_name: form_data.full_name.value.clone(),
        company: form_data.company.value.clone(),
        email: form_data.email.value.clone(),
        message: form_data.message.value.clone(),
    };

    let mailer = match &state.mailer {
        Some(mailer) => mailer,
        None => {
            return render_error(
                &state,
                SiteKind::Cv,
                StatusCode::NOT_FOUND,
                uri.path(),
                &language,
            )
            .map(|html| (StatusCode::NOT_FOUND, Html(html)).into_response())
            .unwrap_or_else(|status| status.into_response());
        },
    };

    let (status, feedback) = match mailer.send_cv_contact(payload).await {
        Ok(_) => (
            StatusCode::OK,
            ContactFeedback {
                status: "success".to_string(),
                message: translations
                    .get_str(&["contact", "feedback_success"])
                    .unwrap_or_else(|| "Message sent successfully.".to_string()),
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

    render_cv_curriculum(&state, Some(context), &language, uri.path())
        .map(|html| (status, Html(html)).into_response())
        .unwrap_or_else(|render_status| render_status.into_response())
}

pub(crate) async fn menu(State(state): State<AppState>, headers: HeaderMap, uri: Uri) -> Response {
    if let Some(redirect) = redirect_missing_language(&state, &uri) {
        return redirect;
    }

    let (language, _, _) = state.language_from_path(uri.path());

    let Some(site) = site_from_host(&headers, &state) else {
        return site_not_available(&state, uri.path(), &language);
    };

    if site != SiteKind::Cv {
        return render_error(&state, SiteKind::Main, StatusCode::NOT_FOUND, uri.path(), &language)
            .map(|html| (StatusCode::NOT_FOUND, Html(html)).into_response())
            .unwrap_or_else(|status| status.into_response());
    }

    if !state.language_has_site(&language, SiteKind::Cv) {
        return render_error(&state, SiteKind::Cv, StatusCode::NOT_FOUND, uri.path(), &language)
            .map(|html| (StatusCode::NOT_FOUND, Html(html)).into_response())
            .unwrap_or_else(|status| status.into_response());
    }

    render_cv_menu(&state, &language, uri.path())
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

    ([(CONTENT_TYPE, "text/plain; charset=utf-8")], body.clone()).into_response()
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

    ([(CONTENT_TYPE, "application/xml; charset=utf-8")], body.clone()).into_response()
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

    ([(CONTENT_TYPE, "application/manifest+json")], body.clone()).into_response()
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

    ([(CONTENT_TYPE, "text/plain; charset=utf-8")], body.clone()).into_response()
}

pub(crate) async fn not_found(
    State(state): State<AppState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Some(redirect) = redirect_missing_language(&state, &uri) {
        return redirect;
    }

    let (language, _, _) = state.language_from_path(uri.path());

    let site = site_from_host(&headers, &state).or_else(|| fallback_site(&state));

    match site {
        Some(site) => render_error(&state, site, StatusCode::NOT_FOUND, uri.path(), &language)
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

fn validate_form(form: &FormData, translations: &Translations) -> Option<ContactFeedback> {
    if form.full_name.value.is_empty()
        || form.company.value.is_empty()
        || form.email.value.is_empty()
        || form.message.value.is_empty()
    {
        return Some(ContactFeedback {
            status: "error".to_string(),
            message: translations
                .get_str(&["validation", "required_fields"])
                .unwrap_or_else(|| "Please complete all fields before submitting.".to_string()),
        });
    }

    if !(FULL_NAME_MIN..=FULL_NAME_MAX).contains(&form.full_name.len) {
        return Some(ContactFeedback {
            status: "error".to_string(),
            message: translations
                .get_str(&["validation", "full_name_length"])
                .unwrap_or_else(|| "Full name must be between 3 and 120 characters.".to_string()),
        });
    }

    if !(COMPANY_MIN..=COMPANY_MAX).contains(&form.company.len) {
        return Some(ContactFeedback {
            status: "error".to_string(),
            message: translations.get_str(&["validation", "company_length"]).unwrap_or_else(|| {
                "Company name must be between 2 and 120 characters.".to_string()
            }),
        });
    }

    if !(EMAIL_MIN..=EMAIL_MAX).contains(&form.email.len) || !email_is_valid(&form.email.value) {
        return Some(ContactFeedback {
            status: "error".to_string(),
            message: translations
                .get_str(&["validation", "email"])
                .unwrap_or_else(|| "Please enter a valid email address.".to_string()),
        });
    }

    if !(MESSAGE_MIN..=MESSAGE_MAX).contains(&form.message.len) {
        return Some(ContactFeedback {
            status: "error".to_string(),
            message: translations
                .get_str(&["validation", "message_length"])
                .unwrap_or_else(|| "Message must be between 10 and 2000 characters.".to_string()),
        });
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
        .unwrap_or_default()
        .to_ascii_lowercase()
        .split(':')
        .next()
        .unwrap_or_default()
        .to_string();

    if snapshot.cv_available && snapshot.cv_domains.contains(&host) {
        return Some(SiteKind::Cv);
    }

    if snapshot.main_available && snapshot.main_domains.contains(&host) {
        return Some(SiteKind::Main);
    }

    fallback_site(state)
}

fn request_origin_is_allowed(headers: &HeaderMap, state: &AppState) -> bool {
    let Some(origin) = headers.get(ORIGIN) else {
        return true;
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
        state::{LocalizationState, StaticAssets},
    };
    use std::sync::{Arc, RwLock};
    use tera::Tera;

    #[test]
    fn same_host_origin_is_allowed() {
        let headers = headers_with("cv.example.com:8081", "https://cv.example.com");

        assert!(request_origin_is_allowed(&headers, &app_state()));
    }

    #[test]
    fn configured_site_origin_is_allowed() {
        let headers = headers_with("127.0.0.1:8081", "https://cv.example.com");

        assert!(request_origin_is_allowed(&headers, &app_state()));
    }

    #[test]
    fn malformed_origin_is_rejected() {
        let headers = headers_with("cv.example.com", "not-a-valid-origin");

        assert!(!request_origin_is_allowed(&headers, &app_state()));
    }

    #[test]
    fn cross_site_origin_is_rejected() {
        let headers = headers_with("cv.example.com", "https://attacker.example");

        assert!(!request_origin_is_allowed(&headers, &app_state()));
    }

    fn headers_with(host: &str, origin: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, host.parse().unwrap());
        headers.insert(ORIGIN, origin.parse().unwrap());
        headers
    }

    fn app_state() -> AppState {
        let localizations = Localizations::new();
        let localization_state = Arc::new(LocalizationState::new(
            localizations,
            Language::new("en"),
            vec![Language::new("en")],
            std::collections::HashMap::new(),
        ));

        let mut state = AppState {
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
            default_site: SiteKind::Cv,
            environment: Environment::Production,
            asset_version: String::new(),
            generator: String::new(),
            mailer: None,
            show_credits: true,
        };

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
        state.localization_state = Arc::new(RwLock::new(localization_state));

        state
    }
}
