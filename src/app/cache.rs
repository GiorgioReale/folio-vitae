use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::error::{AppResult, ResultExt};

const CACHE_PATH: &str = ".folio-cache.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct InputSnapshot {
    pub data: Option<u64>,
    pub assets: Option<u64>,
    pub templates: Option<u64>,
    pub images: Option<u64>,
    pub minify_assets: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuildCache {
    pub assets_last_modified: Option<u64>,
    pub templates_last_modified: Option<u64>,
    pub images_last_modified: Option<u64>,
    pub data_last_modified: Option<u64>,
    pub minify_assets: bool,
    pub last_build_inputs: Option<InputSnapshot>,
}

impl BuildCache {
    pub(crate) fn load() -> AppResult<Self> {
        let contents = match fs::read_to_string(CACHE_PATH) {
            Ok(contents) => contents,
            Err(_) => return Ok(Self::default()),
        };

        serde_json::from_str(&contents).with_context(|| format!("parsing {CACHE_PATH}"))
    }

    pub(crate) fn save(&self) -> AppResult<()> {
        let serialized = serde_json::to_string_pretty(self)
            .with_context(|| format!("serializing {CACHE_PATH}"))?;

        if let Ok(existing) = fs::read_to_string(CACHE_PATH)
            && existing == serialized
        {
            return Ok(());
        }

        fs::write(CACHE_PATH, serialized).with_context(|| format!("writing {CACHE_PATH}"))
    }

    pub(crate) fn should_refresh_assets(
        &self,
        assets_last_modified: Option<SystemTime>,
        minify_assets: bool,
    ) -> bool {
        self.assets_last_modified != system_time_to_timestamp(assets_last_modified)
            || self.minify_assets != minify_assets
    }

    pub(crate) fn should_refresh_images(&self, images_last_modified: Option<SystemTime>) -> bool {
        self.images_last_modified != system_time_to_timestamp(images_last_modified)
    }

    pub(crate) fn record_assets(
        &mut self,
        assets_last_modified: Option<SystemTime>,
        minify_assets: bool,
    ) {
        self.assets_last_modified = system_time_to_timestamp(assets_last_modified);
        self.minify_assets = minify_assets;
    }

    pub(crate) fn record_templates(&mut self, templates_last_modified: Option<SystemTime>) {
        self.templates_last_modified = system_time_to_timestamp(templates_last_modified);
    }

    pub(crate) fn record_images(&mut self, images_last_modified: Option<SystemTime>) {
        self.images_last_modified = system_time_to_timestamp(images_last_modified);
    }

    pub(crate) fn record_data(&mut self, data_last_modified: Option<SystemTime>) {
        self.data_last_modified = system_time_to_timestamp(data_last_modified);
    }
}

pub(crate) fn latest_modification<P: AsRef<Path>>(paths: &[P]) -> AppResult<Option<SystemTime>> {
    let mut latest: Option<SystemTime> = None;

    for path in paths {
        let path = path.as_ref();

        if !path.exists() {
            continue;
        }

        if path.is_file() {
            update_latest(&mut latest, path)?;
            continue;
        }

        for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }

            update_latest(&mut latest, entry.path())?;
        }
    }

    Ok(latest)
}

fn update_latest(latest: &mut Option<SystemTime>, path: &Path) -> AppResult<()> {
    let metadata = fs::metadata(path).with_context(|| format!("reading metadata for {path:?}"))?;
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

pub(crate) fn system_time_to_timestamp(time: Option<SystemTime>) -> Option<u64> {
    time.and_then(|time| time.duration_since(UNIX_EPOCH).ok().map(|duration| duration.as_secs()))
}
