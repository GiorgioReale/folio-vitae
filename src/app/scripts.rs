use std::fs;

use minifier::js;
use walkdir::WalkDir;

use crate::{
    app::environment::Environment,
    error::{AppResult, ResultExt},
};

pub(crate) fn process_scripts(environment: Environment) -> AppResult<()> {
    if !environment.should_minify_assets() {
        return Ok(());
    }

    for entry in WalkDir::new("assets/scripts") {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("js") {
            continue;
        }

        let file_stem = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default();
        if file_stem.ends_with(".min") {
            continue;
        }

        let source = fs::read_to_string(path).with_context(|| format!("reading {path:?}"))?;

        let minified = js::minify(&source);
        let minified = minified.to_string().into_bytes();

        let mut output_path = path.to_path_buf();
        output_path.set_extension("min.js");

        let needs_write = match fs::read(&output_path) {
            Ok(existing) => existing != minified,
            Err(_) => true,
        };

        if needs_write {
            fs::write(&output_path, minified)
                .with_context(|| format!("writing {output_path:?}"))?;
        }
    }

    Ok(())
}
