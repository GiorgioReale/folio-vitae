use std::{
    borrow::Cow,
    sync::{Arc, OnceLock},
};

use chrono::{Datelike, NaiveDate, Utc};
use http::StatusCode;
use minifier::html;
use serde::Serialize;
use simpleicons_rs;
use tera::Context;
use tracing::error;

use crate::app::{
    localization::{Language, Translations},
    state::{AppState, HtmlCacheKey, SiteKind},
};

pub(crate) fn render_main_homepage(
    state: &AppState,
    language: &Language,
    request_path: &str,
) -> Result<Arc<str>, StatusCode> {
    let resources =
        load_resources(state, language, "failed to load resources for the main homepage")?;
    let mut context = base_context(state, &resources, language, request_path, SiteKind::Main);
    context.insert("page_url", request_path);
    context.insert("robots", "index,follow");
    render_template(
        state,
        "main/pages/homepage.html",
        context,
        Some(cache_key(SiteKind::Main, language, "main/pages/homepage.html", request_path)),
    )
}

pub(crate) fn render_cv_homepage(
    state: &AppState,
    language: &Language,
    request_path: &str,
) -> Result<Arc<str>, StatusCode> {
    let resources =
        load_resources(state, language, "failed to load resources for the CV homepage")?;
    let mut context = base_context(state, &resources, language, request_path, SiteKind::Cv);
    context.insert("page_url", request_path);
    context.insert("robots", "index,follow");
    render_template(
        state,
        "cv/pages/homepage.html",
        context,
        Some(cache_key(SiteKind::Cv, language, "cv/pages/homepage.html", request_path)),
    )
}

pub(crate) fn render_cv_curriculum(
    state: &AppState,
    contact_form: Option<CvContactFormContext>,
    language: &Language,
    request_path: &str,
) -> Result<Arc<str>, StatusCode> {
    let resources = load_resources(state, language, "failed to load resources for the curriculum")?;
    let mut context = base_context(state, &resources, language, request_path, SiteKind::Cv);
    let translations = resources.translations.as_ref();
    let page_title = translated_or(translations, &["cv_curriculum", "page_title"], "Curriculum");

    context.insert("page_title", page_title.as_ref());
    context.insert("page_url", request_path);
    context.insert("robots", "index,follow");

    let has_contact_feedback = contact_form.as_ref().is_some_and(|form| form.feedback.is_some());
    let contact_form_context = contact_form.unwrap_or_default();
    context.insert("contact_form", &contact_form_context);
    context.insert("contact_form_enabled", &state.mailer.is_some());

    let cache = if has_contact_feedback {
        None
    } else {
        Some(cache_key(SiteKind::Cv, language, "cv/pages/curriculum.html", request_path))
    };

    render_template(state, "cv/pages/curriculum.html", context, cache)
}

pub(crate) fn render_cv_menu(
    state: &AppState,
    language: &Language,
    request_path: &str,
) -> Result<Arc<str>, StatusCode> {
    let resources = load_resources(state, language, "failed to load resources for menu")?;
    let mut context = base_context(state, &resources, language, request_path, SiteKind::Cv);
    let translations = resources.translations.as_ref();
    let profile = &resources.data.sites.cv.profile;
    let page_title = translated_or(translations, &["cv_menu", "title"], "Menu");

    context.insert("page_title", page_title.as_ref());
    context.insert("page_url", request_path);
    context.insert("robots", "noindex,nofollow");
    context.insert("has_about_section", &!profile.summary.trim().is_empty());
    context.insert("has_experience_section", &!profile.work_experiences.is_empty());
    context.insert("has_education_section", &!profile.educations.is_empty());
    context.insert("has_skills_section", &!profile.skills.is_empty());
    context.insert("has_courses_section", &!profile.courses_certifications.is_empty());
    context.insert("has_other_projects_section", &!profile.other_projects.is_empty());
    context.insert("has_download_section", &true);
    context.insert("has_contact_section", &state.mailer.is_some());

    render_template(
        state,
        "cv/pages/menu.html",
        context,
        Some(cache_key(SiteKind::Cv, language, "cv/pages/menu.html", request_path)),
    )
}

pub(crate) fn render_languages_page(
    state: &AppState,
    language: &Language,
    request_path: &str,
    site: SiteKind,
) -> Result<Arc<str>, StatusCode> {
    let resources = load_resources(state, language, "failed to load resources for languages")?;
    let mut context = base_context(state, &resources, language, request_path, site);
    let translations = resources.translations.as_ref();
    let page_title = translated_or(translations, &["nav", "languages"], "Languages");
    let snapshot = state.localization_snapshot();
    let home_path = {
        let prefix =
            language.url_prefix(&snapshot.default_language, snapshot.supported_languages.len() > 1);
        if prefix.is_empty() { "/".to_string() } else { format!("{prefix}/") }
    };
    let language_choices = snapshot.language_links(language, &home_path, site);

    context.insert("page_title", page_title.as_ref());
    context.insert("page_url", request_path);
    context.insert("robots", "noindex,nofollow");
    context.insert("languages", &language_choices);

    let template = match site {
        SiteKind::Cv => "cv/pages/languages.html",
        SiteKind::Main => "main/pages/languages.html",
    };

    render_template(
        state,
        template,
        context,
        Some(cache_key(site, language, template, request_path)),
    )
}

pub(crate) fn render_error(
    state: &AppState,
    site: SiteKind,
    status: StatusCode,
    page_url: &str,
    language: &Language,
) -> Result<Arc<str>, StatusCode> {
    let resources = load_resources(state, language, "failed to load resources for error page")?;
    let mut context = base_context(state, &resources, language, page_url, site);
    context.insert("page_url", page_url);
    context.insert("robots", "noindex,nofollow");

    let site_value = match site {
        SiteKind::Main => "main",
        SiteKind::Cv => "cv",
    };

    context.insert("site", site_value);

    let translations = resources.translations.as_ref();
    let error = error_details(status, translations);
    let title_prefix = translated_or(translations, &["errors", "title"], "Error");
    let page_title = format!("{title_prefix} {}", error.code);

    context.insert("page_title", page_title.as_str());
    let error_code = error.code.to_string();
    context.insert("error_code", &error_code);
    context.insert("error_code_first", &error_code.chars().next().unwrap_or('0').to_string());
    context.insert("error_code_middle", &error_code.chars().nth(1).unwrap_or('0').to_string());
    context.insert("error_code_last", &error_code.chars().nth(2).unwrap_or('0').to_string());
    context.insert("error_message", &error.message);

    render_template(state, "errors/error.html", context, None)
}

fn load_resources(
    state: &AppState,
    language: &Language,
    reason: &'static str,
) -> Result<crate::app::localization::LocalizedResources, StatusCode> {
    state.resources_for(language).map_err(|error| {
        error!(error = ?error, "{reason}");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

fn translated_or<'a>(
    translations: &'a Translations,
    path: &[&str],
    fallback: &'a str,
) -> Cow<'a, str> {
    match translations.navigate(path).and_then(serde_json::Value::as_str) {
        Some(value) => Cow::Borrowed(value),
        None => Cow::Borrowed(fallback),
    }
}

fn base_context(
    state: &AppState,
    resources: &crate::app::localization::LocalizedResources,
    language: &Language,
    request_path: &str,
    site: SiteKind,
) -> Context {
    let mut context = Context::new();
    let data = resources.data.as_ref();
    let translations = resources.translations.as_ref();
    let dates = RenderDates::new(translations, resources.date_formats.as_ref());
    let person = &data.person;
    context.insert("first_name", &person.first_name);
    context.insert("last_name", &person.last_name);
    context.insert("role", &person.role);
    context.insert("date_of_birth", &dates.display(&person.date_of_birth));
    context.insert("position", &person.position);

    let snapshot = state.localization_snapshot();
    let sites = &data.sites;
    let has_website_main = snapshot.language_has_site(language, SiteKind::Main);
    let has_website_cv = snapshot.language_has_site(language, SiteKind::Cv);

    context.insert("website_main", sites.resolved_main());
    context.insert("website_cv", sites.resolved_cv());
    context.insert("has_website_main", &has_website_main);
    context.insert("has_website_cv", &has_website_cv);
    context.insert("websites", &sites.additional);

    let contact = &data.contact;
    context.insert("email_main", &contact.emails.main);
    context.insert("email_cv", contact.emails.resolved_cv());
    context.insert("emails", &contact.emails.other);
    context.insert("socials", &format_socials(&contact.socials));

    let profile = &data.sites.cv.profile;
    context.insert("work_experiences", &format_work_experiences(&profile.work_experiences, &dates));
    context.insert("other_projects", &format_projects(&profile.other_projects, &dates));
    context.insert("educations", &format_educations(&profile.educations, &dates));
    context.insert("skills", &format_skills(&profile.skills));
    context.insert(
        "courses_certifications",
        &format_courses_certifications(&profile.courses_certifications, &dates),
    );
    context.insert("description", &profile.summary);
    context.insert("current_year", &Utc::now().year());
    context.insert("minify_assets", &state.environment.should_minify_assets());
    context.insert("asset_version", state.asset_version.as_ref());
    context.insert("generator", state.generator.as_ref());
    context.insert("show_credits", &state.show_credits);
    let (_, path_suffix, language_prefix) =
        crate::app::state::language_from_path_in_snapshot(request_path, &snapshot);
    context.insert("language_prefix", &language_prefix);
    let language_links = snapshot.language_links(language, &path_suffix, site);
    let default_code = snapshot.default_language.as_code();
    let hreflang_default_url = language_links
        .iter()
        .find(|link| link.code == default_code)
        .map(|link| link.url.clone())
        .or_else(|| language_links.first().map(|link| link.url.clone()))
        .unwrap_or_else(|| request_path.to_string());
    context.insert("languages", &language_links);
    context.insert("hreflang_default_url", &hreflang_default_url);
    context.insert("translations", translations);
    context.insert("language_tag", translations.language_tag().unwrap_or(language.as_code()));
    let meta_language_fallback = language.as_code().to_uppercase();
    context.insert(
        "meta_language",
        translations.meta_language().unwrap_or(meta_language_fallback.as_str()),
    );
    context.insert("dc_language", translations.dc_language().unwrap_or(language.as_code()));
    context.insert("language_direction", translations.language_direction().unwrap_or("ltr"));
    let full_name = format!("{} {}", person.first_name, person.last_name);
    let social_card_alt = translations
        .get_str(&["accessibility", "social_card_alt"])
        .unwrap_or_else(|| format!("Social card for {full_name}"))
        .replace("{name}", &full_name);
    context.insert("social_card_alt", &social_card_alt);
    context
}

fn cache_key(
    site: SiteKind,
    language: &Language,
    template: &'static str,
    request_path: &str,
) -> HtmlCacheKey {
    HtmlCacheKey {
        site,
        language: language.clone(),
        template,
        request_path: request_path.to_string(),
    }
}

fn render_template(
    state: &AppState,
    template: &str,
    context: Context,
    cache: Option<HtmlCacheKey>,
) -> Result<Arc<str>, StatusCode> {
    if let Some(key) = cache.as_ref()
        && let Some(cached) = state.cached_html_page(key)
    {
        return Ok(cached);
    }

    let rendered = state.tera.render(template, &context).map_err(|error| {
        error!(error = ?error, template, "failed to render template");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let rendered = if state.environment.should_minify_html() {
        Arc::<str>::from(html::minify(&rendered))
    } else {
        Arc::<str>::from(rendered)
    };

    if let Some(key) = cache {
        state.store_html_page(key, Arc::clone(&rendered));
    }

    Ok(rendered)
}

struct ErrorDetails {
    code: u16,
    message: String,
}

fn error_details(status: StatusCode, translations: &Translations) -> ErrorDetails {
    let code_key = status.as_u16().to_string();
    let message = translations
        .navigate(&["errors", "status_messages", code_key.as_str()])
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| {
            if status.is_server_error() {
                translations
                    .get_str(&["errors", "server_error"])
                    .unwrap_or_else(|| "Server error".to_string())
            } else {
                translations
                    .get_str(&["errors", "client_error"])
                    .unwrap_or_else(|| "Request error".to_string())
            }
        });

    ErrorDetails { code: status.as_u16(), message }
}

#[derive(Serialize)]
struct FormattedWorkExperience {
    company: String,
    location: String,
    start_date: String,
    end_date: String,
    duration: String,
    roles: Vec<FormattedWorkRole>,
}

#[derive(Serialize)]
struct FormattedWorkRole {
    title: String,
    start_date: String,
    end_date: String,
    duration: String,
    description: String,
}

#[derive(Serialize)]
struct FormattedProject {
    title: String,
    start_date: String,
    end_date: String,
    duration: String,
    description: String,
    link: String,
}

#[derive(Serialize)]
struct FormattedEducation {
    institute: String,
    educational_qualification: String,
    course_of_study: String,
    date_start: String,
    date_end: String,
    duration: String,
    vote: Option<String>,
    description: String,
}

#[derive(Serialize)]
struct FormattedCourseCertification {
    title: String,
    released_by: String,
    released_on: String,
    link: String,
}

#[derive(Serialize)]
struct SocialWithIcon {
    social_name: String,
    social_full_name: String,
    link: String,
    profile_nickname: String,
    icon_svg: Option<&'static str>,
}

#[derive(Serialize)]
struct SkillWithIcon {
    name: String,
    icon: String,
    icon_svg: Option<&'static str>,
}

#[derive(Serialize, Default, Clone)]
pub(crate) struct CvContactFormContext {
    pub full_name: String,
    pub company: String,
    pub email: String,
    pub message: String,
    pub feedback: Option<ContactFeedback>,
}

#[derive(Serialize, Clone)]
pub(crate) struct ContactFeedback {
    pub status: String,
    pub message: String,
    pub field: Option<String>,
}

struct RenderDates<'a> {
    formats: &'a crate::app::localization::DateFormats,
    months: Vec<String>,
    ongoing: String,
    singular_year: String,
    plural_year: String,
    singular_month: String,
    plural_month: String,
    and_word: String,
}

impl<'a> RenderDates<'a> {
    fn new(
        translations: &'a Translations,
        formats: &'a crate::app::localization::DateFormats,
    ) -> Self {
        let (singular_year, plural_year, singular_month, plural_month) = match translations
            .duration_labels()
        {
            Some((year, years, month, months)) => {
                (year.to_string(), years.to_string(), month.to_string(), months.to_string())
            },
            None => {
                ("year".to_string(), "years".to_string(), "month".to_string(), "months".to_string())
            },
        };

        Self {
            formats,
            months: translations.months().unwrap_or_else(default_months),
            ongoing: translations.ongoing_label().unwrap_or_else(|| "Present".to_string()),
            singular_year,
            plural_year,
            singular_month,
            plural_month,
            and_word: translations.get_str(&["dates", "and"]).unwrap_or_else(|| "and".to_string()),
        }
    }

    fn display(&self, date: &NaiveDate) -> String {
        format_date_with_pattern(date, &self.months, &self.formats.long, true)
    }

    fn month_year(&self, date: &NaiveDate) -> String {
        format_date_with_pattern(date, &self.months, &self.formats.long, false)
    }

    fn end(&self, end_date: Option<&NaiveDate>) -> String {
        let Some(end_date) = end_date else {
            return self.ongoing.clone();
        };

        if is_ongoing_date(end_date) {
            return self.ongoing.clone();
        }

        self.month_year(end_date)
    }

    fn duration(&self, start_date: &NaiveDate, end_date: Option<&NaiveDate>) -> Option<String> {
        let end = match end_date.filter(|date| !is_ongoing_date(date)) {
            Some(date) => *date,
            None => Utc::now().date_naive(),
        };

        if end < *start_date {
            return None;
        }

        let mut total_months = end.year().saturating_sub(start_date.year()).saturating_mul(12)
            + (end.month() as i32 - start_date.month() as i32)
            + 1;

        if end.day() < start_date.day() {
            total_months = total_months.saturating_sub(1);
        }

        if total_months <= 0 {
            return None;
        }

        let years = total_months / 12;
        let months = total_months % 12;
        let year_label =
            if years == 1 { self.singular_year.as_str() } else { self.plural_year.as_str() };
        let month_label =
            if months == 1 { self.singular_month.as_str() } else { self.plural_month.as_str() };

        Some(match (years, months) {
            (0, month_count) => format!("{month_count} {month_label}"),
            (year_count, 0) => format!("{year_count} {year_label}"),
            (year_count, month_count) => {
                format!(
                    "{year_count} {year_label} {and_word} {month_count} {month_label}",
                    and_word = self.and_word
                )
            },
        })
    }
}

fn format_work_experiences(
    work_experiences: &[crate::app::models::WorkExperience],
    dates: &RenderDates<'_>,
) -> Vec<FormattedWorkExperience> {
    work_experiences
        .iter()
        .map(|experience| {
            let roles = experience
                .roles
                .iter()
                .map(|role| FormattedWorkRole {
                    title: role.title.clone(),
                    start_date: dates.month_year(&role.start_date),
                    end_date: dates.end(role.end_date.as_ref()),
                    duration: dates
                        .duration(&role.start_date, role.end_date.as_ref())
                        .unwrap_or_default(),
                    description: role.description.clone(),
                })
                .collect();

            let (start_date, end_date, duration) =
                summarize_work_experience(&experience.roles, dates);

            FormattedWorkExperience {
                company: experience.company.name.clone(),
                location: experience.company.location.clone(),
                start_date,
                end_date,
                duration,
                roles,
            }
        })
        .collect()
}

fn summarize_work_experience(
    roles: &[crate::app::models::WorkRole],
    dates: &RenderDates<'_>,
) -> (String, String, String) {
    if roles.is_empty() {
        return (String::new(), String::new(), String::new());
    }

    let earliest_start =
        roles.iter().map(|role| role.start_date).min().unwrap_or_else(|| Utc::now().date_naive());

    let mut latest_end: Option<NaiveDate> = None;
    let mut ongoing = false;

    for role in roles {
        match role.end_date.as_ref() {
            Some(end) if is_ongoing_date(end) => ongoing = true,
            Some(end) => {
                if latest_end.map(|current| end > &current).unwrap_or(true) {
                    latest_end = Some(*end);
                }
            },
            None => ongoing = true,
        }
    }

    let end_date_value = if ongoing { None } else { latest_end.as_ref() };

    let start_date = dates.month_year(&earliest_start);
    let end_date = if ongoing { dates.end(None) } else { dates.end(end_date_value) };
    let duration = dates.duration(&earliest_start, end_date_value).unwrap_or_default();

    (start_date, end_date, duration)
}

fn format_projects(
    projects: &[crate::app::models::Project],
    dates: &RenderDates<'_>,
) -> Vec<FormattedProject> {
    projects
        .iter()
        .map(|project| FormattedProject {
            title: project.title.clone(),
            start_date: dates.month_year(&project.start_date),
            end_date: dates.end(project.end_date.as_ref()),
            duration: dates
                .duration(&project.start_date, project.end_date.as_ref())
                .unwrap_or_default(),
            description: project.description.clone(),
            link: project.link.clone(),
        })
        .collect()
}

fn format_educations(
    educations: &[crate::app::models::Education],
    dates: &RenderDates<'_>,
) -> Vec<FormattedEducation> {
    educations
        .iter()
        .map(|education| FormattedEducation {
            institute: education.institute.clone(),
            educational_qualification: education.educational_qualification.clone(),
            course_of_study: education.course_of_study.clone(),
            date_start: dates.month_year(&education.date_start),
            date_end: dates.end(education.date_end.as_ref()),
            duration: dates
                .duration(&education.date_start, education.date_end.as_ref())
                .unwrap_or_default(),
            vote: education.vote.clone(),
            description: education.description.clone(),
        })
        .collect()
}

fn format_courses_certifications(
    courses_certifications: &[crate::app::models::CourseCertification],
    dates: &RenderDates<'_>,
) -> Vec<FormattedCourseCertification> {
    courses_certifications
        .iter()
        .map(|course| FormattedCourseCertification {
            title: course.title.clone(),
            released_by: course.released_by.clone(),
            released_on: dates.display(&course.released_on),
            link: course.link.clone(),
        })
        .collect()
}

fn format_socials(socials: &[crate::app::models::Social]) -> Vec<SocialWithIcon> {
    socials
        .iter()
        .map(|social| SocialWithIcon {
            social_name: social.social_name.clone(),
            social_full_name: social.social_full_name.clone(),
            link: social.link.clone(),
            profile_nickname: social.profile_nickname.clone(),
            icon_svg: simple_icon_svg(&social.social_name),
        })
        .collect()
}

fn format_skills(skills: &[crate::app::models::Skill]) -> Vec<SkillWithIcon> {
    skills
        .iter()
        .map(|skill| SkillWithIcon {
            name: skill.name.clone(),
            icon: skill.icon.clone(),
            icon_svg: simple_icon_svg(&skill.icon),
        })
        .collect()
}

fn simple_icon_svg(slug: &str) -> Option<&'static str> {
    if slug.eq_ignore_ascii_case("linkedin") {
        return None;
    }

    simpleicons_rs::slug(slug).map(|icon| icon.svg)
}

fn is_ongoing_date(date: &NaiveDate) -> bool {
    date.year() == 9999 && date.month() == 12 && date.day() == 31
}

fn default_months() -> Vec<String> {
    vec![
        "January".to_string(),
        "February".to_string(),
        "March".to_string(),
        "April".to_string(),
        "May".to_string(),
        "June".to_string(),
        "July".to_string(),
        "August".to_string(),
        "September".to_string(),
        "October".to_string(),
        "November".to_string(),
        "December".to_string(),
    ]
}

fn format_date_with_pattern(
    parsed_date: &NaiveDate,
    months: &[String],
    pattern: &str,
    include_day: bool,
) -> String {
    let year = parsed_date.year().to_string();
    let month_index = parsed_date.month0() as usize;
    let month_name =
        months.get(month_index).cloned().unwrap_or_else(|| format!("{:02}", parsed_date.month()));
    let month_number = format!("{:02}", parsed_date.month());
    let day = format!("{:02}", parsed_date.day());

    let mut formatted = pattern.to_string();
    formatted = formatted.replace("YYYY", &year);
    formatted = formatted.replace("MMMM", &month_name);
    formatted = formatted.replace("MM", &month_number);

    if include_day {
        formatted = formatted.replace("DD", &day);
    } else {
        formatted = formatted.replace("DD", "");
    }

    normalize_formatted_date(&formatted)
}

fn is_date_edge(character: char) -> bool {
    matches!(character, '-' | '/' | '.' | ',' | '\\') || character.is_whitespace()
}

fn collapse_repeated(input: &str, target: char) -> String {
    let mut collapsed = String::with_capacity(input.len());
    let mut previous_was_target = false;

    for character in input.chars() {
        if character == target {
            if previous_was_target {
                continue;
            }
            previous_was_target = true;
        } else {
            previous_was_target = false;
        }

        collapsed.push(character);
    }

    collapsed
}

fn collapse_date_punctuation(input: &str) -> String {
    let mut collapsed = String::with_capacity(input.len());
    let mut previous = None;

    for character in input.chars() {
        if matches!(character, '-' | '/' | '.' | ',') && previous == Some(character) {
            continue;
        }

        collapsed.push(character);
        previous = Some(character);
    }

    collapsed
}

fn normalize_formatted_date(formatted: &str) -> String {
    let collapsed_spaces = collapse_repeated(formatted, ' ');
    let trimmed = collapsed_spaces.trim_matches(is_date_edge);
    collapse_date_punctuation(trimmed).trim_matches(is_date_edge).to_string()
}

pub(crate) fn render_cv_pdf(state: &AppState, language: &Language) -> Result<Vec<u8>, StatusCode> {
    let content = cv_pdf_content(state, language)?;
    crate::app::pdf::render(&content).map_err(|error| {
        error!(error = ?error, "failed to render CV PDF");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

fn cv_pdf_content(
    state: &AppState,
    language: &Language,
) -> Result<crate::app::pdf::CvPdfContent, StatusCode> {
    let resources = state.resources_for(language).map_err(|error| {
        error!(error = ?error, "failed to load resources for CV PDF");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let data = resources.data.as_ref();
    let translations = resources.translations.as_ref();
    let dates = RenderDates::new(translations, resources.date_formats.as_ref());
    let person = &data.person;
    let profile = &data.sites.cv.profile;

    let label = |key: &str, fallback: &str| {
        translations.get_str(&["cv_curriculum", key]).unwrap_or_else(|| fallback.to_string())
    };

    let mut facts = Vec::new();
    if !person.position.locality.trim().is_empty() {
        facts.push(crate::app::pdf::PdfFact {
            label: label("city_label", "City:"),
            value: person.position.locality.clone(),
            href: None,
        });
    }
    facts.push(crate::app::pdf::PdfFact {
        label: label("birth_date_label", "Birth date:"),
        value: dates.display(&person.date_of_birth),
        href: None,
    });
    let website_domain = if state.language_has_site(language, SiteKind::Main) {
        Some(data.sites.resolved_main().domain.as_str())
    } else if state.language_has_site(language, SiteKind::Cv) {
        Some(data.sites.resolved_cv().domain.as_str())
    } else {
        None
    };
    if let Some(domain) = website_domain.filter(|domain| !domain.trim().is_empty()) {
        facts.push(crate::app::pdf::PdfFact {
            label: label("website_label", "Website:"),
            value: domain.to_string(),
            href: Some(format!("https://{domain}/")),
        });
    }
    let email = data.contact.emails.resolved_cv();
    if !email.trim().is_empty() {
        facts.push(crate::app::pdf::PdfFact {
            label: label("email_label", "E-Mail:"),
            value: email.to_string(),
            href: Some(format!("mailto:{email}")),
        });
    }

    let mut sections = Vec::new();
    if !profile.summary.trim().is_empty() {
        sections.push(crate::app::pdf::PdfSection {
            heading: label("about_heading", "About Me"),
            blocks: vec![crate::app::pdf::PdfBlock::Paragraph(profile.summary.clone())],
        });
    }

    let experiences = format_work_experiences(&profile.work_experiences, &dates);
    if !experiences.is_empty() {
        sections.push(crate::app::pdf::PdfSection {
            heading: label("experience_heading", "Experience"),
            blocks: experiences
                .into_iter()
                .map(|experience| {
                    crate::app::pdf::PdfBlock::Group(crate::app::pdf::PdfGroup {
                        title: experience.company,
                        subtitle: experience.location,
                        entries: experience
                            .roles
                            .into_iter()
                            .map(|role| crate::app::pdf::PdfEntry {
                                title: role.title,
                                subtitle: String::new(),
                                meta: dated_meta(&role.start_date, &role.end_date, &role.duration),
                                body: role.description,
                                notes: Vec::new(),
                            })
                            .collect(),
                    })
                })
                .collect(),
        });
    }

    if !profile.skills.is_empty() {
        sections.push(crate::app::pdf::PdfSection {
            heading: label("skills_heading", "Skills"),
            blocks: vec![crate::app::pdf::PdfBlock::Skills(
                profile
                    .skills
                    .iter()
                    .map(|skill| crate::app::pdf::PdfSkill { icon: skill.icon.clone() })
                    .collect(),
            )],
        });
    }

    let projects = format_projects(&profile.other_projects, &dates);
    if !projects.is_empty() {
        let project_link = label("project_link", "Link");
        sections.push(crate::app::pdf::PdfSection {
            heading: label("other_projects_heading", "Other Projects"),
            blocks: projects
                .into_iter()
                .map(|project| {
                    let mut notes = Vec::new();
                    if !project.link.trim().is_empty() {
                        notes.push(format!("{project_link}: {}", project.link));
                    }
                    crate::app::pdf::PdfBlock::Entry(crate::app::pdf::PdfEntry {
                        title: project.title,
                        subtitle: String::new(),
                        meta: dated_meta(&project.start_date, &project.end_date, &project.duration),
                        body: project.description,
                        notes,
                    })
                })
                .collect(),
        });
    }

    let educations = format_educations(&profile.educations, &dates);
    if !educations.is_empty() {
        let vote_label = label("vote", "Vote");
        sections.push(crate::app::pdf::PdfSection {
            heading: label("education_heading", "Education"),
            blocks: educations
                .into_iter()
                .map(|education| {
                    let mut notes = Vec::new();
                    if let Some(vote) = education.vote.filter(|vote| !vote.trim().is_empty()) {
                        notes.push(format!("{vote_label}: {vote}"));
                    }
                    crate::app::pdf::PdfBlock::Entry(crate::app::pdf::PdfEntry {
                        title: education.institute,
                        subtitle: [education.course_of_study, education.educational_qualification]
                            .into_iter()
                            .filter(|part| !part.trim().is_empty())
                            .collect::<Vec<_>>()
                            .join(" — "),
                        meta: dated_meta(
                            &education.date_start,
                            &education.date_end,
                            &education.duration,
                        ),
                        body: education.description,
                        notes,
                    })
                })
                .collect(),
        });
    }

    let courses = format_courses_certifications(&profile.courses_certifications, &dates);
    if !courses.is_empty() {
        let released_by = label("course_released_by", "Released by");
        let released_on = label("course_released_on", "Released on");
        let certificate = label("certificate_link", "Certificate");
        sections.push(crate::app::pdf::PdfSection {
            heading: label("courses_heading", "Courses & Certifications"),
            blocks: courses
                .into_iter()
                .map(|course| {
                    let mut notes = Vec::new();
                    if !course.link.trim().is_empty() {
                        notes.push(format!("{certificate}: {}", course.link));
                    }
                    crate::app::pdf::PdfBlock::Entry(crate::app::pdf::PdfEntry {
                        title: course.title,
                        subtitle: format!("{released_by}: {}", course.released_by),
                        meta: format!("{released_on} {}", course.released_on),
                        body: String::new(),
                        notes,
                    })
                })
                .collect(),
        });
    }

    let photo = cached_profile_photo();

    Ok(crate::app::pdf::CvPdfContent {
        title: format!("{} {}", person.first_name.trim(), person.last_name.trim())
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
        first_name: person.first_name.clone(),
        last_name: person.last_name.clone(),
        role: person.role.clone(),
        photo,
        facts,
        sections,
        legal_notice: label("legal_notice", ""),
    })
}

fn dated_meta(start: &str, end: &str, duration: &str) -> String {
    let mut meta = format!("{start} - {end}");
    if !duration.trim().is_empty() {
        meta.push_str(&format!(" ({duration})"));
    }
    meta
}

fn cached_profile_photo() -> Option<Vec<u8>> {
    static PROFILE_PHOTO: OnceLock<Option<Vec<u8>>> = OnceLock::new();

    PROFILE_PHOTO
        .get_or_init(|| std::fs::read("assets/images/cv/profile-pictures/profile-picture.png").ok())
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::{environment::Environment, localization::Language, state::SiteKind},
        prepare_application_state,
    };

    #[test]
    fn date_normalization_matches_the_previous_cleanup_rules() {
        for sample in [
            "18  April  1992",
            "  --18//04..1992,, ",
            "DD  MMMM   YYYY",
            "----",
            "a - - b",
            "18---04",
            "end\\",
        ] {
            assert_eq!(normalize_formatted_date(sample), legacy_date_cleanup(sample));
        }
    }

    #[test]
    fn key_pages_render_with_tera_v2() {
        if !std::path::Path::new("data").is_dir() {
            // CI and fresh checkouts do not ship a local `data/` directory.
            return;
        }

        let prepared = prepare_application_state(Environment::Development)
            .expect("application state should prepare for template smoke tests");
        let language = Language::new("it");

        let main_home = render_main_homepage(&prepared.main_state, &language, "/")
            .expect("main homepage should render");
        assert!(main_home.contains("skip-link"));
        assert!(main_home.contains("main-content"));

        let cv_home = render_cv_homepage(&prepared.cv_state, &language, "/")
            .expect("cv homepage should render");
        assert!(cv_home.contains("skip-link"));

        let curriculum = render_cv_curriculum(&prepared.cv_state, None, &language, "/curriculum")
            .expect("curriculum should render");
        assert!(curriculum.contains("main-content"));

        let menu =
            render_cv_menu(&prepared.cv_state, &language, "/menu").expect("menu should render");
        assert!(menu.contains("menu-title"));

        let languages =
            render_languages_page(&prepared.cv_state, &language, "/it/languages", SiteKind::Cv)
                .expect("languages page should render");
        assert!(languages.contains("languages-title"));
        assert!(languages.contains("data-language-option"));
        assert!(languages.contains("/en/") || languages.contains("href=\"/en"));

        let error = render_error(
            &prepared.cv_state,
            SiteKind::Cv,
            StatusCode::NOT_FOUND,
            "/missing",
            &language,
        )
        .expect("error page should render");
        assert!(error.contains("error-code"));
    }

    fn legacy_date_cleanup(formatted: &str) -> String {
        let mut formatted = formatted.to_string();

        while formatted.contains("  ") {
            formatted = formatted.replace("  ", " ");
        }

        let mut cleaned = formatted.trim_matches(is_date_edge).to_string();

        for duplicate in ["--", "//", "..", ",,"] {
            let Some(first_char) = duplicate.chars().next() else {
                continue;
            };

            while cleaned.contains(duplicate) {
                cleaned = cleaned.replace(duplicate, &first_char.to_string());
            }
        }

        cleaned.trim_matches(is_date_edge).to_string()
    }
}
