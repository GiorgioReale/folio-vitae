use std::{
    fs,
    path::Path,
    time::SystemTime,
};

use walkdir::WalkDir;

use crate::error::{AppError, AppResult, ResultExt};

pub(crate) fn latest_modification<P: AsRef<Path>>(paths: &[P]) -> AppResult<Option<SystemTime>> {
    let mut latest: Option<SystemTime> = None;

    for path in paths {
        let path = path.as_ref();
        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        if metadata.is_file() {
            record_modified(&mut latest, path, &metadata)?;
            continue;
        }

        if !metadata.is_dir() {
            continue;
        }

        for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }

            let file_metadata = entry.metadata().map_err(|error| {
                AppError::msg(format!("reading metadata for {}: {error}", entry.path().display()))
            })?;
            record_modified(&mut latest, entry.path(), &file_metadata)?;
        }
    }

    Ok(latest)
}

fn record_modified(
    latest: &mut Option<SystemTime>,
    path: &Path,
    metadata: &fs::Metadata,
) -> AppResult<()> {
    let modified = metadata
        .modified()
        .or_else(|_| metadata.created())
        .with_context(|| format!("getting modification time for {path:?}"))?;

    *latest = match latest {
        Some(current) if *current > modified => Some(*current),
        _ => Some(modified),
    };

    Ok(())
}
