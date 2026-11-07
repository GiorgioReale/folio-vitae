use std::fs;

use grass::{OutputStyle, from_path};

use crate::{
    app::environment::Environment,
    error::{AppResult, ResultExt},
};

pub(crate) fn compile_styles(environment: Environment) -> AppResult<()> {
    let styles = [
        ("assets/styles/main.scss", "assets/styles/main.css"),
        ("assets/styles/cv.scss", "assets/styles/cv.css"),
    ];

    for (input, output) in styles {
        let css = from_path(
            input,
            &grass::Options::default()
                .style(match environment.should_minify_assets() {
                    true => OutputStyle::Compressed,
                    false => OutputStyle::Expanded,
                })
                .load_path("assets/styles"),
        )
        .with_context(|| format!("compiling {input}"))?;

        let needs_write = match fs::read_to_string(output) {
            Ok(existing) => existing != css,
            Err(_) => true,
        };

        if needs_write {
            fs::write(output, css).with_context(|| format!("writing {output}"))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::environment::Environment;

    #[test]
    fn compile_styles_emits_cross_document_view_transitions() {
        compile_styles(Environment::Development).expect("SCSS should compile");

        let css = fs::read_to_string("assets/styles/main.css").expect("main.css");
        assert!(css.contains("@view-transition"));
        assert!(css.contains("navigation: auto"));
        assert!(css.contains("folio-vt-open"));
        assert!(css.contains("folio-vt-retreat"));
        assert!(css.contains("view-transition-name: folio-nav"));
        assert!(css.contains("view-transition-name: folio-page-title"));
        assert!(css.contains("view-transition-name: folio-page-role"));
        assert!(css.contains("prefers-reduced-motion: reduce"));
        assert!(css.contains("navigation: none"));
    }
}
