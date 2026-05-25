use std::{fs, path::Path};

use include_dir::{Dir, include_dir};

use crate::error::{AppError, AppResult, ResultExt};

const EMBEDDED_TEMPLATES: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates");
const EMBEDDED_STATIC: Dir = include_dir!("$CARGO_MANIFEST_DIR/static");
const EMBEDDED_ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");
const EMBEDDED_I18N: Dir = include_dir!("$CARGO_MANIFEST_DIR/i18n");
const EMBEDDED_ASSETS_VERSION: &str = env!("CARGO_PKG_VERSION");

const INIT_DATA_IT: &str = include_str!("../data_example/data.it.yaml");
const INIT_DATA_EN: &str = include_str!("../data_example/data.en.yaml");
const INIT_ENV: &str = include_str!("../data_example/.env.example");

pub fn ensure_runtime_assets() -> AppResult<()> {
    let version_path = Path::new(".folio-assets-version");
    let current_version =
        fs::read_to_string(version_path).ok().map(|value| value.trim().to_owned());
    let should_update = current_version.as_deref() != Some(EMBEDDED_ASSETS_VERSION);
    let force = should_update;

    write_dir(&EMBEDDED_ASSETS, Path::new("assets"), force)?;
    write_dir(&EMBEDDED_TEMPLATES, Path::new("templates"), force)?;
    write_dir(&EMBEDDED_STATIC, Path::new("static"), force)?;
    write_dir(&EMBEDDED_I18N, Path::new("i18n"), force)?;

    if should_update {
        fs::write(version_path, EMBEDDED_ASSETS_VERSION)
            .with_context(|| format!("writing {version_path:?}"))?;
    }

    Ok(())
}

pub fn init_project_files(force: bool) -> AppResult<()> {
    fs::create_dir_all("data").with_context(|| "creating data directory")?;

    write_file(Path::new("data/data.it.yaml"), INIT_DATA_IT, force)?;
    write_file(Path::new("data/data.en.yaml"), INIT_DATA_EN, force)?;
    write_file(Path::new(".env.example"), INIT_ENV, force)?;

    Ok(())
}

fn write_dir(dir: &Dir, target: &Path, force: bool) -> AppResult<()> {
    let base_path = dir.path();

    for entry in dir.dirs() {
        let relative_path = entry.path().strip_prefix(base_path).unwrap_or_else(|_| entry.path());

        let destination = target.join(relative_path);
        fs::create_dir_all(&destination)
            .with_context(|| format!("creating directory {destination:?}"))?;
        write_dir(entry, &destination, force)?;
    }

    for file in dir.files() {
        let relative_path = file.path().strip_prefix(base_path).unwrap_or_else(|_| file.path());

        let destination = target.join(relative_path);
        if destination.exists() && !force {
            continue;
        }

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating directory {parent:?}"))?;
        }

        let contents = file
            .contents_utf8()
            .map_or_else(|| file.contents().to_vec(), |value| value.as_bytes().to_vec());

        fs::write(&destination, contents)
            .with_context(|| format!("writing file {destination:?}"))?;
    }

    Ok(())
}

fn write_file(path: &Path, contents: &str, force: bool) -> AppResult<()> {
    if path.exists() && !force {
        return Err(AppError::msg(format!(
            "{path:?} already exists. Use --force to overwrite existing files."
        )));
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating directory {parent:?}"))?;
    }

    fs::write(path, contents).with_context(|| format!("writing {path:?}"))
}
