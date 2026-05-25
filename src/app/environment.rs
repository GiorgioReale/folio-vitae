#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Production,
}

impl Environment {
    pub fn detect() -> Self {
        match std::env::var("APP_ENV") {
            Ok(value) => match value.to_ascii_lowercase().as_str() {
                "production" => Environment::Production,
                "development" => Environment::Development,
                _ => Environment::Development,
            },
            Err(_) => Environment::Development,
        }
    }

    pub fn should_minify_assets(self) -> bool {
        matches!(self, Environment::Production)
    }

    pub fn should_minify_html(self) -> bool {
        matches!(self, Environment::Production)
    }
}
