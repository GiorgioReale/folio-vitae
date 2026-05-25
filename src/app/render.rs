use chrono::{Datelike, NaiveDate, Utc};
use http::StatusCode;
use minifier::html;
use serde::Serialize;
use simpleicons_rs;
use tera::Context;

use crate::app::{
    localization::{Language, Translations},
    state::{AppState, SiteKind},
};

pub(crate) fn render_main_homepage(
    state: &AppState,
    language: &Language,
    request_path: &str,
) -> Result<String, StatusCode> {
    let mut context = base_context(state, language, request_path, SiteKind::Main)?;
    context.insert("page_url", request_path);
    context.insert("robots", "index,follow");
    render_template(state, "main/pages/homepage.html", context)
}

pub(crate) fn render_cv_homepage(
    state: &AppState,
    language: &Language,
    request_path: &str,
) -> Result<String, StatusCode> {
    let mut context = base_context(state, language, request_path, SiteKind::Cv)?;
    context.insert("page_url", request_path);
    context.insert("robots", "index,follow");
    render_template(state, "cv/pages/homepage.html", context)
}

pub(crate) fn render_cv_curriculum(
    state: &AppState,
    contact_form: Option<CvContactFormContext>,
    language: &Language,
    request_path: &str,
) -> Result<String, StatusCode> {
    let mut context = base_context(state, language, request_path, SiteKind::Cv)?;

    let translations = state
        .resources_for(language)
        .map_err(|error| {
            eprintln!("Failed to load translations: {error:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .translations;
    let page_title = translations
        .get_str(&["cv_curriculum", "page_title"])
        .unwrap_or_else(|| "Curriculum".to_string());

    context.insert("page_title", &page_title);
    context.insert("page_url", request_path);
    context.insert("robots", "index,follow");

    let contact_form_context = contact_form.unwrap_or_default();
    context.insert("contact_form", &contact_form_context);
    context.insert("contact_form_enabled", &state.mailer.is_some());

    if let Some(age) = calculate_age(
        &state
            .resources_for(language)
            .map_err(|error| {
                eprintln!("Failed to load resources for age calculation: {error:#?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .data
            .person
            .date_of_birth,
    ) {
        context.insert("age", &age);
    }

    render_template(state, "cv/pages/curriculum.html", context)
}

pub(crate) fn render_cv_menu(
    state: &AppState,
    language: &Language,
    request_path: &str,
) -> Result<String, StatusCode> {
    let mut context = base_context(state, language, request_path, SiteKind::Cv)?;
    let resources = state.resources_for(language).map_err(|error| {
        eprintln!("Failed to load resources for menu: {error:#?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let translations = resources.translations;
    let profile = &resources.data.sites.cv.profile;

    let page_title =
        translations.get_str(&["cv_menu", "title"]).unwrap_or_else(|| "Menu".to_string());

    context.insert("page_title", &page_title);
    context.insert("page_url", request_path);
    context.insert("robots", "noindex,nofollow");
    context.insert("has_about_section", &!profile.summary.trim().is_empty());
    context.insert("has_experience_section", &!profile.work_experiences.is_empty());
    context.insert("has_education_section", &!profile.educations.is_empty());
    context.insert("has_skills_section", &!profile.skills.is_empty());
    context.insert("has_courses_section", &!profile.courses_certifications.is_empty());
    context.insert("has_contact_section", &state.mailer.is_some());

    render_template(state, "cv/pages/menu.html", context)
}

pub(crate) fn render_error(
    state: &AppState,
    site: SiteKind,
    status: StatusCode,
    page_url: &str,
    language: &Language,
) -> Result<String, StatusCode> {
    let mut context = base_context(state, language, page_url, site)?;
    context.insert("page_url", page_url);
    context.insert("robots", "noindex,nofollow");

    let site_value = match site {
        SiteKind::Main => "main",
        SiteKind::Cv => "cv",
    };

    context.insert("site", site_value);

    let translations = state
        .resources_for(language)
        .map_err(|error| {
            eprintln!("Failed to load translations for error page: {error:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .translations;
    let error = error_details(status, translations.as_ref());
    let title_prefix =
        translations.get_str(&["errors", "title"]).unwrap_or_else(|| "Error".to_string());
    let page_title = format!("{title_prefix} {}", error.code);

    context.insert("page_title", &page_title);
    let error_code = error.code.to_string();
    context.insert("error_code", &error_code);
    context.insert("error_code_first", &error_code.chars().next().unwrap_or('0').to_string());
    context.insert("error_code_middle", &error_code.chars().nth(1).unwrap_or('0').to_string());
    context.insert("error_code_last", &error_code.chars().nth(2).unwrap_or('0').to_string());
    context.insert("error_message", &error.message);

    render_template(state, "errors/error.html", context)
}

fn base_context(
    state: &AppState,
    language: &Language,
    request_path: &str,
    site: SiteKind,
) -> Result<Context, StatusCode> {
    let mut context = Context::new();
    let resources = state.resources_for(language).map_err(|error| {
        eprintln!("Failed to load resources for context: {error:#?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let data = resources.data.as_ref();
    let translations = resources.translations.as_ref();
    let date_formats = resources.date_formats.as_ref();
    let person = &data.person;
    context.insert("first_name", &person.first_name);
    context.insert("last_name", &person.last_name);
    context.insert("role", &person.role);
    context.insert(
        "date_of_birth",
        &format_display_date(&person.date_of_birth, translations, date_formats),
    );
    context.insert("position", &person.position);

    let sites = &data.sites;
    let website_main = sites.resolved_main();
    let website_cv = sites.resolved_cv();
    let has_website_main = state.language_has_site(language, SiteKind::Main);
    let has_website_cv = state.language_has_site(language, SiteKind::Cv);

    context.insert("website_main", &website_main);
    context.insert("website_cv", &website_cv);
    context.insert("has_website_main", &has_website_main);
    context.insert("has_website_cv", &has_website_cv);
    context.insert("websites", &sites.additional);

    let contact = &data.contact;
    let email_main = &contact.emails.main;
    let email_cv = contact.emails.resolved_cv();

    context.insert("email_main", email_main);
    context.insert("email_cv", &email_cv);
    context.insert("emails", &contact.emails.other);
    context.insert("socials", &format_socials(&contact.socials));

    let profile = &data.sites.cv.profile;
    context.insert(
        "work_experiences",
        &format_work_experiences(&profile.work_experiences, translations, date_formats),
    );
    context.insert(
        "other_projects",
        &format_projects(&profile.other_projects, translations, date_formats),
    );
    context
        .insert("educations", &format_educations(&profile.educations, translations, date_formats));
    context.insert("skills", &format_skills(&profile.skills));
    context.insert(
        "courses_certifications",
        &format_courses_certifications(&profile.courses_certifications, translations, date_formats),
    );
    context.insert("description", &profile.summary);
    context.insert("current_year", &Utc::now().year());
    context.insert("minify_assets", &state.environment.should_minify_assets());
    context.insert("asset_version", &state.asset_version);
    context.insert("generator", &state.generator);
    context.insert("show_credits", &state.show_credits);
    let (_, _, language_prefix) = state.language_from_path(request_path);
    context.insert("language_prefix", &language_prefix);
    context.insert("languages", &state.language_links(language, request_path, site));
    context.insert("translations", translations);
    context.insert(
        "language_tag",
        &translations.language_tag().unwrap_or_else(|| language.as_code().to_string()),
    );
    context.insert(
        "meta_language",
        &translations.meta_language().unwrap_or_else(|| language.as_code().to_uppercase()),
    );
    context.insert(
        "dc_language",
        &translations.dc_language().unwrap_or_else(|| language.as_code().to_string()),
    );
    context.insert(
        "language_direction",
        &translations.language_direction().unwrap_or_else(|| "ltr".to_string()),
    );
    Ok(context)
}

fn render_template(
    state: &AppState,
    template: &str,
    context: Context,
) -> Result<String, StatusCode> {
    let rendered = state.tera.render(template, &context).map_err(|error| {
        eprintln!("Failed to render template {template}: {error:#?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if !state.environment.should_minify_html() {
        return Ok(rendered);
    }

    Ok(html::minify(&rendered))
}

struct ErrorDetails {
    code: u16,
    message: String,
}

fn error_details(status: StatusCode, translations: &Translations) -> ErrorDetails {
    let status_messages = translations
        .navigate(&["errors", "status_messages"])
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();
    let code_key = status.as_u16().to_string();

    let default_client_error = translations
        .get_str(&["errors", "client_error"])
        .unwrap_or_else(|| "Request error".to_string());
    let default_server_error = translations
        .get_str(&["errors", "server_error"])
        .unwrap_or_else(|| "Server error".to_string());

    let message = status_messages
        .get(&code_key)
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .unwrap_or_else(|| {
            if status.is_server_error() {
                default_server_error.clone()
            } else {
                default_client_error.clone()
            }
        });

    ErrorDetails { code: status.as_u16(), message }
}

fn calculate_age(date_of_birth: &NaiveDate) -> Option<i32> {
    let today = Utc::now().date_naive();

    let mut age = today.year() - date_of_birth.year();

    let birth_day_of_year = date_of_birth.ordinal();
    let today_day_of_year = today.ordinal();
    if today_day_of_year < birth_day_of_year {
        age -= 1;
    }

    Some(age)
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
    icon_svg: Option<String>,
}

#[derive(Serialize)]
struct SkillWithIcon {
    name: String,
    icon: String,
    icon_svg: Option<String>,
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
}

fn format_work_experiences(
    work_experiences: &[crate::app::models::WorkExperience],
    translations: &Translations,
    date_formats: &crate::app::localization::DateFormats,
) -> Vec<FormattedWorkExperience> {
    work_experiences
        .iter()
        .map(|experience| {
            let roles = experience
                .roles
                .iter()
                .map(|role| FormattedWorkRole {
                    title: role.title.clone(),
                    start_date: format_month_year_date(
                        &role.start_date,
                        translations,
                        date_formats,
                    ),
                    end_date: format_end_date(role.end_date.as_ref(), translations, date_formats),
                    duration: format_duration(
                        &role.start_date,
                        role.end_date.as_ref(),
                        translations,
                    )
                    .unwrap_or_default(),
                    description: role.description.clone(),
                })
                .collect();

            let (start_date, end_date, duration) =
                summarize_work_experience(&experience.roles, translations, date_formats);

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
    translations: &Translations,
    date_formats: &crate::app::localization::DateFormats,
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

    let start_date = format_month_year_date(&earliest_start, translations, date_formats);
    let end_date = if ongoing {
        format_end_date(None, translations, date_formats)
    } else {
        format_end_date(end_date_value, translations, date_formats)
    };

    let duration =
        format_duration(&earliest_start, end_date_value, translations).unwrap_or_default();

    (start_date, end_date, duration)
}

fn format_projects(
    projects: &[crate::app::models::Project],
    translations: &Translations,
    date_formats: &crate::app::localization::DateFormats,
) -> Vec<FormattedProject> {
    projects
        .iter()
        .map(|project| FormattedProject {
            title: project.title.clone(),
            start_date: format_month_year_date(&project.start_date, translations, date_formats),
            end_date: format_end_date(project.end_date.as_ref(), translations, date_formats),
            duration: format_duration(&project.start_date, project.end_date.as_ref(), translations)
                .unwrap_or_default(),
            description: project.description.clone(),
            link: project.link.clone(),
        })
        .collect()
}

fn format_educations(
    educations: &[crate::app::models::Education],
    translations: &Translations,
    date_formats: &crate::app::localization::DateFormats,
) -> Vec<FormattedEducation> {
    educations
        .iter()
        .map(|education| FormattedEducation {
            institute: education.institute.clone(),
            educational_qualification: education.educational_qualification.clone(),
            course_of_study: education.course_of_study.clone(),
            date_start: format_month_year_date(&education.date_start, translations, date_formats),
            date_end: format_end_date(education.date_end.as_ref(), translations, date_formats),
            duration: format_duration(
                &education.date_start,
                education.date_end.as_ref(),
                translations,
            )
            .unwrap_or_default(),
            vote: education.vote.clone(),
            description: education.description.clone(),
        })
        .collect()
}

fn format_courses_certifications(
    courses_certifications: &[crate::app::models::CourseCertification],
    translations: &Translations,
    date_formats: &crate::app::localization::DateFormats,
) -> Vec<FormattedCourseCertification> {
    courses_certifications
        .iter()
        .map(|course| FormattedCourseCertification {
            title: course.title.clone(),
            released_by: course.released_by.clone(),
            released_on: format_display_date(&course.released_on, translations, date_formats),
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

fn simple_icon_svg(slug: &str) -> Option<String> {
    if slug.eq_ignore_ascii_case("linkedin") {
        return None;
    }

    simpleicons_rs::slug(slug).map(|icon| icon.svg.to_string())
}

fn format_end_date(
    end_date: Option<&NaiveDate>,
    translations: &Translations,
    date_formats: &crate::app::localization::DateFormats,
) -> String {
    let Some(end_date) = end_date else {
        return translations.ongoing_label().unwrap_or_else(|| "Present".to_string());
    };

    if is_ongoing_date(end_date) {
        return translations.ongoing_label().unwrap_or_else(|| "Present".to_string());
    }

    format_month_year_date(end_date, translations, date_formats)
}

fn is_ongoing_date(date: &NaiveDate) -> bool {
    date.year() == 9999 && date.month() == 12 && date.day() == 31
}

fn format_display_date(
    date: &NaiveDate,
    translations: &Translations,
    date_formats: &crate::app::localization::DateFormats,
) -> String {
    let months = translations.months().unwrap_or_else(default_months);

    format_date_with_pattern(date, &months, &date_formats.long, true)
}

fn format_month_year_date(
    date: &NaiveDate,
    translations: &Translations,
    date_formats: &crate::app::localization::DateFormats,
) -> String {
    let months = translations.months().unwrap_or_else(default_months);

    format_date_with_pattern(date, &months, &date_formats.long, false)
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

    while formatted.contains("  ") {
        formatted = formatted.replace("  ", " ");
    }

    let mut cleaned = formatted
        .trim_matches(|c: char| matches!(c, '-' | '/' | '.' | ',' | '\\') || c.is_whitespace())
        .to_string();

    for duplicate in ["--", "//", "..", ",,"] {
        if let Some(first_char) = duplicate.chars().next() {
            while cleaned.contains(duplicate) {
                cleaned = cleaned.replace(duplicate, &first_char.to_string());
            }
        }
    }

    cleaned
        .trim_matches(|c: char| matches!(c, '-' | '/' | '.' | ',' | '\\') || c.is_whitespace())
        .to_string()
}

fn format_duration(
    start_date: &NaiveDate,
    end_date: Option<&NaiveDate>,
    translations: &Translations,
) -> Option<String> {
    let end = match end_date.filter(|date| !is_ongoing_date(date)) {
        Some(date) => *date,
        None => Utc::now().date_naive(),
    };

    if end < *start_date {
        return None;
    }

    let mut total_months = (end.year() - start_date.year()) * 12
        + (end.month() as i32 - start_date.month() as i32)
        + 1;

    if end.day() < start_date.day() {
        total_months -= 1;
    }

    if total_months <= 0 {
        return None;
    }

    let years = total_months / 12;
    let months = total_months % 12;

    let (singular_year, plural_year, singular_month, plural_month) =
        translations.duration_labels().unwrap_or_else(|| {
            ("year".to_string(), "years".to_string(), "month".to_string(), "months".to_string())
        });

    let year_label = if years == 1 { singular_year.as_str() } else { plural_year.as_str() };
    let month_label = if months == 1 { singular_month.as_str() } else { plural_month.as_str() };

    Some(match (years, months) {
        (0, m) => format!("{m} {month_label}"),
        (y, 0) => format!("{y} {year_label}"),
        (y, m) => format!("{y} {year_label} e {m} {month_label}"),
    })
}
