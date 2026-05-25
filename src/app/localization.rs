use std::{collections::HashMap, env, fs, io::ErrorKind, sync::Arc, time::SystemTime};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use strsim::jaro_winkler;

use crate::{
    app::models::{SiteData, expected_fields_for_container},
    config::data_dir,
    error::{AppError, AppResult, ResultExt},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Language {
    code: String,
}

impl Language {
    pub(crate) fn new(code: impl Into<String>) -> Self {
        let normalized = code.into().trim().to_ascii_lowercase();

        Self { code: normalized }
    }

    pub(crate) fn as_code(&self) -> &str {
        self.code.as_str()
    }

    pub(crate) fn default(available: &[Language]) -> AppResult<(Self, bool)> {
        Self::default_with_env(available, || env::var("DEFAULT_LANGUAGE").ok())
    }

    fn default_with_env<F>(available: &[Language], env_lookup: F) -> AppResult<(Self, bool)>
    where
        F: Fn() -> Option<String>,
    {
        let first_language =
            available.first().cloned().ok_or_else(|| AppError::msg("no languages available"))?;

        if available.len() == 1 {
            return Ok((first_language, false));
        }

        let env_default = env_lookup().as_deref().map(|value| value.trim().to_ascii_lowercase());

        if let Some(language) =
            env_default.as_deref().and_then(|code| Language::from_code(code, available))
        {
            return Ok((language, true));
        }

        let fallback = available
            .iter()
            .find(|language| language.as_code() == "en")
            .cloned()
            .unwrap_or(first_language);

        Ok((fallback, false))
    }

    pub(crate) fn from_code(value: &str, available: &[Language]) -> Option<Self> {
        available.iter().find(|language| language.as_code() == value).cloned()
    }

    pub(crate) fn url_prefix(&self, _default_language: &Language, is_multilingual: bool) -> String {
        if !is_multilingual {
            return String::new();
        }

        let code = self.as_code();

        format!("/{code}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn language_codes_are_normalized() {
        let language = Language::new(" IT  ");

        assert_eq!(language.as_code(), "it");
    }

    #[test]
    fn default_language_prefers_environment_variable() {
        let (language, from_env) =
            Language::default_with_env(&[Language::new("en"), Language::new("it")], || {
                Some("IT".to_string())
            })
            .unwrap();

        assert_eq!(language.as_code(), "it");
        assert!(from_env);
    }

    #[test]
    fn default_language_falls_back_to_english() {
        let (language, from_env) =
            Language::default_with_env(&[Language::new("es"), Language::new("en")], || None)
                .unwrap();

        assert_eq!(language.as_code(), "en");
        assert!(!from_env);
    }

    #[test]
    fn url_prefix_respects_multilingual_flag() {
        let english = Language::new("en");
        let italian = Language::new("it");

        assert_eq!(english.url_prefix(&english, false), "");
        assert_eq!(italian.url_prefix(&english, true), "/it");
    }

    #[test]
    fn translations_extracts_nested_values() {
        let translations = Translations::new(json!({
            "dates": {
                "month_names": [" January ", "February"],
                "ongoing": " Ongoing ",
                "year": "year ",
                "years": "years",
                "month": "month ",
                "months": " months "
            },
            "language": {
                "tag": "en-US",
                "meta_language": "English",
                "dc_language": "en",
                "direction": "RTL"
            }
        }));

        assert_eq!(
            translations.months(),
            Some(vec![" January ".to_string(), "February".to_string()])
        );
        assert_eq!(translations.ongoing_label(), Some(" Ongoing ".to_string()));
        assert_eq!(
            translations.duration_labels(),
            Some((
                "year".to_string(),
                "years".to_string(),
                "month".to_string(),
                "months".to_string()
            ))
        );
        assert_eq!(translations.language_tag(), Some("en-US".to_string()));
        assert_eq!(translations.meta_language(), Some("English".to_string()));
        assert_eq!(translations.dc_language(), Some("en".to_string()));
        assert_eq!(translations.language_direction(), Some("rtl".to_string()));
    }
}

#[derive(Clone)]
pub(crate) struct Translations {
    raw: Value,
}

impl Translations {
    pub(crate) fn new(raw: Value) -> Self {
        Self { raw }
    }

    pub(crate) fn get_str(&self, path: &[&str]) -> Option<String> {
        self.navigate(path).and_then(|value| value.as_str().map(|value| value.to_string()))
    }

    pub(crate) fn get_array(&self, path: &[&str]) -> Option<Vec<Value>> {
        self.navigate(path).and_then(|value| value.as_array()).map(|array| array.to_vec())
    }

    pub(crate) fn navigate(&self, path: &[&str]) -> Option<&Value> {
        let mut current = &self.raw;

        for key in path {
            current = current.get(*key)?;
        }

        Some(current)
    }

    pub(crate) fn months(&self) -> Option<Vec<String>> {
        self.get_array(&["dates", "month_names"]).map(|values| {
            values
                .into_iter()
                .filter_map(|value| value.as_str().map(|value| value.to_string()))
                .collect()
        })
    }

    pub(crate) fn ongoing_label(&self) -> Option<String> {
        self.get_str(&["dates", "ongoing"])
    }

    pub(crate) fn duration_labels(&self) -> Option<(String, String, String, String)> {
        Some((
            self.get_str(&["dates", "year"])?.trim().to_string(),
            self.get_str(&["dates", "years"])?.trim().to_string(),
            self.get_str(&["dates", "month"])?.trim().to_string(),
            self.get_str(&["dates", "months"])?.trim().to_string(),
        ))
    }

    pub(crate) fn language_tag(&self) -> Option<String> {
        self.get_str(&["language", "tag"])
    }

    pub(crate) fn meta_language(&self) -> Option<String> {
        self.get_str(&["language", "meta_language"])
    }

    pub(crate) fn dc_language(&self) -> Option<String> {
        self.get_str(&["language", "dc_language"])
    }

    pub(crate) fn language_direction(&self) -> Option<String> {
        self.get_str(&["language", "direction"]).map(|direction| {
            match direction.to_lowercase().as_str() {
                "rtl" => "rtl".to_string(),
                _ => "ltr".to_string(),
            }
        })
    }
}

impl Serialize for Translations {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.raw.serialize(serializer)
    }
}

#[derive(Clone)]
pub(crate) struct LocalizedResources {
    pub(crate) data: Arc<SiteData>,
    pub(crate) translations: Arc<Translations>,
    pub(crate) date_formats: Arc<DateFormats>,
}

pub(crate) type Localizations = HashMap<Language, LocalizedResources>;

#[derive(Deserialize)]
struct LanguageList {
    language_names: HashMap<Language, String>,
}

#[derive(Clone, Deserialize, Default)]
pub(crate) struct DateFormats {
    // The short date pattern is kept for future template usage even though it is not
    // currently read by the renderer.
    #[allow(dead_code)]
    pub(crate) short: String,
    pub(crate) long: String,
}

type DateFormatMap = HashMap<Language, DateFormats>;

fn load_date_formats() -> AppResult<DateFormatMap> {
    let formats_path = "i18n/formats/dates.json";
    let formats_contents =
        std::fs::read_to_string(formats_path).with_context(|| format!("reading {formats_path}"))?;

    let formats: DateFormatMap = parse_json_with_path(&formats_contents, formats_path)?;

    Ok(formats)
}

fn default_date_formats() -> DateFormats {
    DateFormats { short: "YYYY-MM-DD".to_string(), long: "DD MMMM YYYY".to_string() }
}

pub(crate) fn detect_languages_with_last_change() -> AppResult<(Vec<Language>, Option<SystemTime>)>
{
    let mut latest_change: Option<SystemTime> = None;
    let mut languages: Vec<Language> = Vec::new();
    let data_directory = data_dir();

    for entry in fs::read_dir(&data_directory)
        .with_context(|| format!("reading {} directory", data_directory.display()))?
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        if !entry.path().is_file() {
            continue;
        }

        let Some(name) =
            entry.path().file_name().and_then(|name| name.to_str().map(str::to_string))
        else {
            continue;
        };

        if !name.starts_with("data.") || !name.ends_with(".yaml") {
            continue;
        }

        let stripped = name.trim_start_matches("data.").trim_end_matches(".yaml");

        if stripped.is_empty() || stripped == "yaml" || stripped == "yaml.example" {
            continue;
        }

        if let Ok(metadata) = entry.metadata()
            && let Ok(modified) = metadata.modified().or_else(|_| metadata.created())
        {
            latest_change = match latest_change {
                Some(current) if current > modified => Some(current),
                _ => Some(modified),
            };
        }

        languages.push(Language::new(stripped));
    }

    languages.sort_by(|a, b| a.as_code().cmp(b.as_code()));
    languages.dedup_by(|a, b| a.as_code() == b.as_code());

    Ok((languages, latest_change))
}

pub(crate) fn load_localizations(languages: &[Language]) -> AppResult<Localizations> {
    let mut localizations = HashMap::new();
    let date_formats = load_date_formats().unwrap_or_default();
    let data_directory = data_dir();

    for language in languages {
        let data_path =
            data_directory.join(format!("data.{}.yaml", language.as_code())).display().to_string();
        let translations_path = format!("i18n/translations/{}.json", language.as_code());

        let data_contents = read_language_file(&data_path, language, "data")?;
        let translations_contents =
            read_language_file(&translations_path, language, "translations")?;

        let data = parse_site_data(&data_contents, &data_path)?;
        let translations_value: Value =
            parse_json_with_path(&translations_contents, &translations_path)?;

        localizations.insert(
            language.clone(),
            LocalizedResources {
                data: Arc::new(data),
                translations: Arc::new(Translations::new(translations_value)),
                date_formats: Arc::new(
                    date_formats.get(language).cloned().unwrap_or_else(default_date_formats),
                ),
            },
        );
    }

    Ok(localizations)
}

pub(crate) fn load_language_names(languages: &[Language]) -> AppResult<HashMap<Language, String>> {
    let list_path = "i18n/languages.json";
    let list_contents =
        std::fs::read_to_string(list_path).with_context(|| format!("reading {list_path}"))?;

    let list: LanguageList = parse_json_with_path(&list_contents, list_path)?;

    let mut names = HashMap::new();

    for language in languages {
        if let Some(name) = list.language_names.get(language) {
            names.insert(language.clone(), name.clone());
        } else {
            names.insert(language.clone(), language.as_code().to_string());
        }
    }

    Ok(names)
}

fn read_language_file(path: &str, language: &Language, label: &str) -> AppResult<String> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(contents),
        Err(error) if error.kind() == ErrorKind::NotFound => Err(AppError::msg(format!(
            "Unsupported language '{}': missing {label} file at {path}",
            language.as_code()
        ))),
        Err(error) => Err(AppError::from(error).with_context(format!("reading {path}"))),
    }
}

fn parse_site_data(contents: &str, path: &str) -> AppResult<SiteData> {
    let data: SiteData = parse_yaml_with_path(contents, path)?;

    let mut errors = data.validate();
    errors.extend(unknown_site_data_fields(contents)?);

    if errors.is_empty() {
        Ok(data)
    } else {
        let mut message = format!("Invalid configuration in {path}:");
        for error in errors {
            message.push_str(&format!("\n- {error}"));
        }
        Err(AppError::msg(message))
    }
}

fn parse_yaml_with_path<T: DeserializeOwned>(contents: &str, path: &str) -> AppResult<T> {
    let deserializer = serde_yaml::Deserializer::from_str(contents);
    match serde_path_to_error::deserialize(deserializer) {
        Ok(value) => Ok(value),
        Err(error) => Err(AppError::msg(format_yaml_error(path, &error))),
    }
}

fn parse_json_with_path<T: DeserializeOwned>(contents: &str, path: &str) -> AppResult<T> {
    let mut deserializer = serde_json::Deserializer::from_str(contents);
    match serde_path_to_error::deserialize(&mut deserializer) {
        Ok(value) => Ok(value),
        Err(error) => Err(AppError::msg(format_json_error(path, &error))),
    }
}

fn format_yaml_error(path: &str, error: &serde_path_to_error::Error<serde_yaml::Error>) -> String {
    let mut message = format!("Invalid {path}: {}", error.inner());
    if let Some(location) = error.inner().location() {
        message.push_str(&format!(" at line {}, column {}", location.line(), location.column()));
    }
    let path_value = error.path().to_string();
    if !path_value.is_empty() {
        message.push_str(&format!(" (path: {path_value})"));
    }
    message
}

fn format_json_error(path: &str, error: &serde_path_to_error::Error<serde_json::Error>) -> String {
    let inner = error.inner();
    let mut message = format!("Invalid {path}: {inner}");
    message.push_str(&format!(" at line {}, column {}", inner.line(), inner.column()));
    let path_value = error.path().to_string();
    if !path_value.is_empty() {
        message.push_str(&format!(" (path: {path_value})"));
    }
    message
}

fn unknown_site_data_fields(contents: &str) -> AppResult<Vec<String>> {
    let mut unknown_paths = Vec::new();
    let deserializer = serde_yaml::Deserializer::from_str(contents);
    let result: Result<SiteData, serde_yaml::Error> =
        serde_ignored::deserialize(deserializer, |path| {
            unknown_paths.push(path.to_string());
        });

    if let Err(error) = result {
        return Err(AppError::from(error).with_context("parsing data for unknown fields"));
    }

    Ok(unknown_paths.into_iter().filter_map(|path| unknown_field_message(&path)).collect())
}

fn unknown_field_message(path: &str) -> Option<String> {
    let segments = split_path_segments(path);
    let unknown = segments.last()?;
    let container_segments = &segments[..segments.len().saturating_sub(1)];
    let full_label = container_label(&segments);
    let container_key = container_key(container_segments);
    let mut message = full_label.map_or_else(
        || format!("unknown field `{unknown}`"),
        |label| format!("unknown field `{label}`"),
    );

    if let Some(expected) = expected_fields_for_container(container_key.as_deref())
        && let Some(suggestion) = suggest_field(unknown, expected)
    {
        message.push_str(&format!(" (did you mean `{suggestion}`?)"));
    }

    Some(message)
}

fn split_path_segments(path: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = path.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '.' => {
                if !current.is_empty() {
                    segments.push(std::mem::take(&mut current));
                }
            },
            '[' => {
                if !current.is_empty() {
                    segments.push(std::mem::take(&mut current));
                }
                for next in chars.by_ref() {
                    if next == ']' {
                        break;
                    }
                }
            },
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

fn container_key(segments: &[String]) -> Option<String> {
    segments.iter().rev().find(|segment| !is_index_segment(segment)).cloned()
}

fn container_label(segments: &[String]) -> Option<String> {
    if segments.is_empty() {
        return None;
    }

    let mut label = String::new();
    for segment in segments {
        if is_index_segment(segment) {
            label.push('[');
            label.push_str(segment);
            label.push(']');
        } else {
            if !label.is_empty() {
                label.push('.');
            }
            label.push_str(segment);
        }
    }

    Some(label)
}

fn is_index_segment(segment: &str) -> bool {
    !segment.is_empty() && segment.chars().all(|ch| ch.is_ascii_digit())
}

fn suggest_field<'a>(value: &str, candidates: &'a [&'a str]) -> Option<&'a str> {
    let mut best = None;
    let mut best_score = 0.0;

    for candidate in candidates {
        let score = jaro_winkler(value, candidate);
        if score > best_score {
            best_score = score;
            best = Some(*candidate);
        }
    }

    if best_score >= 0.86 { best } else { None }
}
