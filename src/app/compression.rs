use std::{fs, io::Write, path::Path};

use flate2::{Compression, write::GzEncoder};
use walkdir::WalkDir;

use crate::error::{AppResult, ResultExt};

const COMPRESSIBLE_EXTENSIONS: &[&str] =
    &["css", "js", "json", "map", "svg", "txt", "wasm", "webmanifest", "xml"];

pub(crate) fn gzip_assets() -> AppResult<()> {
    for entry in WalkDir::new("assets") {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        if path.extension().and_then(|ext| ext.to_str()) == Some("gz") {
            continue;
        }

        if !is_compressible(path) {
            continue;
        }

        gzip_file(path)?;
    }

    Ok(())
}

fn is_compressible(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| COMPRESSIBLE_EXTENSIONS.contains(&ext))
        .unwrap_or(false)
}

fn gzip_file(path: &Path) -> AppResult<()> {
    let content = fs::read(path).with_context(|| format!("reading {path:?}"))?;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(&content).with_context(|| format!("compressing {path:?}"))?;
    let compressed =
        encoder.finish().with_context(|| format!("finishing compression for {path:?}"))?;

    let gz_path = path.with_file_name(format!(
        "{}.gz",
        path.file_name().and_then(|name| name.to_str()).unwrap_or_default()
    ));

    let needs_write = match fs::read(&gz_path) {
        Ok(existing) => existing != compressed,
        Err(_) => true,
    };

    if needs_write {
        fs::write(&gz_path, compressed).with_context(|| format!("writing {gz_path:?}"))?;
    }

    Ok(())
}
