use anyhow::{Context, Result};
use log::warn;
use std::fs;
use std::path::{Path, PathBuf};

pub mod chart;
pub mod rar;
pub mod zip;

pub trait Extractor: Send + Sync {
    fn can_handle(&self, ext: &str) -> bool;
    fn extract(&self, archive_path: &Path, target_dir: &Path) -> Result<Vec<PathBuf>>;

    fn extract_to(&self, path: &Path) -> Result<ExtractResult> {
        let target_dir = path.with_extension("");

        if !target_dir.exists() {
            fs::create_dir_all(&target_dir)?;
        }

        let extracted_paths = self.extract(path, &target_dir)?;
        cleanup_archive(path);

        Ok(ExtractResult {
            archive_path: path.to_path_buf(),
            target_dir,
            extracted_paths,
        })
    }
}

const EXTRACTORS: &[&dyn Extractor] = &[
    &zip::ZipExtractor,
    &rar::RarExtractor,
    &chart::ChartExtractor,
];

#[derive(Debug)]
pub struct ExtractResult {
    pub archive_path: PathBuf,
    pub target_dir: PathBuf,
    pub extracted_paths: Vec<PathBuf>,
}

pub fn find_extractor(path: &Path) -> Result<&'static dyn Extractor> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .context("Invalid extension")?
        .to_lowercase();

    EXTRACTORS
        .iter()
        .copied()
        .find(|e| e.can_handle(&extension))
        .context("No extractor found")
}

pub fn extract(path: &Path) -> Result<ExtractResult> {
    find_extractor(path)?.extract_to(path)
}

fn cleanup_archive(path: &Path) {
    if path.is_file()
        && let Err(e) = fs::remove_file(path)
    {
        warn!("Failed to remove archive: {} - {}", path.display(), e);
    }
}
