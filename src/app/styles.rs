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
