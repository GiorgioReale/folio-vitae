use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct SiteData {
    pub person: Person,
    pub sites: Sites,
    pub contact: Contact,
}

#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct Person {
    pub first_name: String,
    pub last_name: String,
    pub role: String,
    #[serde(with = "date_format")]
    pub date_of_birth: NaiveDate,
    pub position: Position,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub(crate) struct Position {
    #[serde(default)]
    pub locale: String,
    #[serde(default)]
    pub locality: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub postal_code: String,
    #[serde(default)]
    pub country_name: String,
    #[serde(default)]
    pub latitude: String,
    #[serde(default)]
    pub longitude: String,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub(crate) struct Sites {
    #[serde(default)]
    pub main: Website,
    #[serde(default)]
    pub cv: CvWebsite,
    #[serde(default)]
    pub additional: Vec<Website>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub(crate) struct Website {
    pub domain: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub(crate) struct CvWebsite {
    pub domain: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub profile: Profile,
}

#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct Contact {
    pub emails: ContactEmails,
    #[serde(default)]
    pub socials: Vec<Social>,
}

#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct ContactEmails {
    pub main: String,
    #[serde(default)]
    pub cv: String,
    #[serde(default)]
    pub other: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct Social {
    pub social_name: String,
    pub social_full_name: String,
    pub link: String,
    pub profile_nickname: String,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub(crate) struct Profile {
    pub summary: String,
    #[serde(default)]
    pub work_experiences: Vec<WorkExperience>,
    #[serde(default)]
    pub other_projects: Vec<Project>,
    #[serde(default)]
    pub educations: Vec<Education>,
    #[serde(default)]
    pub skills: Vec<Skill>,
    #[serde(default)]
    pub courses_certifications: Vec<CourseCertification>,
}

#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct WorkExperience {
    pub company: WorkCompany,
    #[serde(default)]
    pub roles: Vec<WorkRole>,
}

#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct WorkCompany {
    pub name: String,
    pub location: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct WorkRole {
    pub title: String,
    #[serde(with = "date_format")]
    pub start_date: NaiveDate,
    #[serde(default, with = "optional_date_format")]
    pub end_date: Option<NaiveDate>,
    #[serde(default)]
    pub description: String,
}

#[derive(Serialize, Clone)]
pub(crate) struct Project {
    pub title: String,
    #[serde(with = "date_format")]
    pub start_date: NaiveDate,
    #[serde(default, with = "optional_date_format")]
    pub end_date: Option<NaiveDate>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub link: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct Education {
    pub institute: String,
    #[serde(default)]
    pub educational_qualification: String,
    #[serde(default)]
    pub course_of_study: String,
    #[serde(with = "date_format")]
    pub date_start: NaiveDate,
    #[serde(default, with = "optional_date_format")]
    pub date_end: Option<NaiveDate>,
    #[serde(default)]
    pub vote: Option<String>,
    #[serde(default)]
    pub description: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct Skill {
    pub name: String,
    pub icon: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct CourseCertification {
    pub title: String,
    pub released_by: String,
    #[serde(with = "date_format")]
    pub released_on: NaiveDate,
    #[serde(default)]
    pub link: String,
}

mod date_format {
    use chrono::NaiveDate;
    use serde::{Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%d";

    pub fn serialize<S>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&date.format(FORMAT).to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        NaiveDate::parse_from_str(&value, FORMAT).map_err(serde::de::Error::custom)
    }
}

#[derive(Deserialize)]
struct ProjectFields {
    title: String,
    #[serde(with = "date_format")]
    start_date: NaiveDate,
    #[serde(default, with = "optional_date_format")]
    end_date: Option<NaiveDate>,
    #[serde(default)]
    description: String,
    #[serde(default)]
    link: Option<String>,
    #[serde(default)]
    link_title: Option<String>,
}

impl<'de> Deserialize<'de> for Project {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let fields = ProjectFields::deserialize(deserializer)?;
        let link = fields.link.or(fields.link_title).unwrap_or_default();

        Ok(Project {
            title: fields.title,
            start_date: fields.start_date,
            end_date: fields.end_date,
            description: fields.description,
            link,
        })
    }
}

mod optional_date_format {
    use chrono::NaiveDate;
    use serde::{Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%d";

    pub fn serialize<S>(date: &Option<NaiveDate>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(value) => serializer.serialize_some(&value.format(FORMAT).to_string()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Option::<String>::deserialize(deserializer)?;
        match value {
            Some(content) if !content.is_empty() => NaiveDate::parse_from_str(&content, FORMAT)
                .map(Some)
                .map_err(serde::de::Error::custom),
            _ => Ok(None),
        }
    }
}

impl From<Website> for CvWebsite {
    fn from(value: Website) -> Self {
        Self {
            domain: value.domain,
            title: value.title,
            description: value.description,
            keywords: value.keywords,
            profile: Profile::default(),
        }
    }
}

impl From<CvWebsite> for Website {
    fn from(value: CvWebsite) -> Self {
        Self {
            domain: value.domain,
            title: value.title,
            description: value.description,
            keywords: value.keywords,
        }
    }
}

impl Sites {
    pub(crate) fn resolved_main(&self) -> Website {
        self.main.clone()
    }

    pub(crate) fn resolved_cv(&self) -> CvWebsite {
        self.cv.clone()
    }

    pub(crate) fn has_main(&self) -> bool {
        !self.main.domain.trim().is_empty()
    }

    pub(crate) fn has_cv(&self) -> bool {
        !self.cv.domain.trim().is_empty()
    }
}

impl ContactEmails {
    pub(crate) fn resolved_cv(&self) -> String {
        if self.cv.is_empty() {
            return self.main.clone();
        }

        self.cv.clone()
    }
}

impl SiteData {
    pub(crate) fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        push_if_empty(&mut errors, &self.person.first_name, "person.first_name");
        push_if_empty(&mut errors, &self.person.last_name, "person.last_name");
        push_if_empty(&mut errors, &self.person.role, "person.role");
        push_if_empty(&mut errors, &self.contact.emails.main, "contact.emails.main");

        if !self.sites.has_main() && !self.sites.has_cv() {
            errors.push(
                "at least one site must be configured: set `sites.main.domain` or `sites.cv.domain`"
                    .to_string(),
            );
        }

        for (index, site) in self.sites.additional.iter().enumerate() {
            if site.domain.trim().is_empty() {
                errors.push(format!("missing value for `sites.additional[{index}].domain`"));
            }
        }

        for (index, social) in self.contact.socials.iter().enumerate() {
            push_if_empty(
                &mut errors,
                &social.social_name,
                &format!("contact.socials[{index}].social_name"),
            );
            push_if_empty(
                &mut errors,
                &social.social_full_name,
                &format!("contact.socials[{index}].social_full_name"),
            );
            push_if_empty(&mut errors, &social.link, &format!("contact.socials[{index}].link"));
            push_if_empty(
                &mut errors,
                &social.profile_nickname,
                &format!("contact.socials[{index}].profile_nickname"),
            );
        }

        errors
    }
}

pub(crate) fn expected_fields_for_container(
    container: Option<&str>,
) -> Option<&'static [&'static str]> {
    match container {
        None => Some(&["person", "sites", "contact"]),
        Some("person") => Some(&["first_name", "last_name", "role", "date_of_birth", "position"]),
        Some("position") => Some(&[
            "locale",
            "locality",
            "region",
            "postal_code",
            "country_name",
            "latitude",
            "longitude",
        ]),
        Some("sites") => Some(&["main", "cv", "additional"]),
        Some("main") => Some(&["domain", "title", "description", "keywords"]),
        Some("cv") => Some(&["domain", "title", "description", "keywords", "profile"]),
        Some("additional") => Some(&["domain", "title", "description", "keywords"]),
        Some("contact") => Some(&["emails", "socials"]),
        Some("emails") => Some(&["main", "cv", "other"]),
        Some("socials") => Some(&["social_name", "social_full_name", "link", "profile_nickname"]),
        Some("profile") => Some(&[
            "summary",
            "work_experiences",
            "other_projects",
            "educations",
            "skills",
            "courses_certifications",
        ]),
        Some("work_experiences") => Some(&["company", "roles"]),
        Some("company") => Some(&["name", "location"]),
        Some("roles") => Some(&["title", "start_date", "end_date", "description"]),
        Some("other_projects") => Some(&["title", "start_date", "end_date", "description", "link"]),
        Some("educations") => Some(&[
            "institute",
            "educational_qualification",
            "course_of_study",
            "date_start",
            "date_end",
            "vote",
            "description",
        ]),
        Some("skills") => Some(&["name", "icon"]),
        Some("courses_certifications") => Some(&["title", "released_by", "released_on", "link"]),
        _ => None,
    }
}

fn push_if_empty(errors: &mut Vec<String>, value: &str, path: &str) {
    if value.trim().is_empty() {
        errors.push(format!("missing value for `{path}`"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_data() -> SiteData {
        SiteData {
            person: Person {
                first_name: "Alex".to_string(),
                last_name: "Carter".to_string(),
                role: "Developer".to_string(),
                date_of_birth: NaiveDate::from_ymd_opt(1992, 4, 18).unwrap(),
                position: Position::default(),
            },
            sites: Sites::default(),
            contact: Contact {
                emails: ContactEmails {
                    main: "hello@example.com".to_string(),
                    cv: String::new(),
                    other: Vec::new(),
                },
                socials: Vec::new(),
            },
        }
    }

    #[test]
    fn validation_allows_main_only() {
        let mut data = sample_data();
        data.sites.main.domain = "example.com".to_string();

        assert!(data.validate().is_empty());
    }

    #[test]
    fn validation_allows_cv_only() {
        let mut data = sample_data();
        data.sites.cv.domain = "cv.example.com".to_string();

        assert!(data.validate().is_empty());
    }

    #[test]
    fn validation_rejects_missing_main_and_cv() {
        let data = sample_data();
        let errors = data.validate();

        assert!(errors.iter().any(|error| error.contains("at least one site must be configured")));
    }
}
