use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub mod rar;
pub mod zip;

pub trait Extractor: Send + Sync {
    fn can_handle(&self, ext: &str) -> bool;
    fn extract(&self, archive_path: &Path, target_dir: &Path) -> Result<Vec<PathBuf>>;
}

const EXTRACTORS: &[&dyn Extractor] = &[&zip::ZipExtractor, &rar::RarExtractor];

#[derive(Debug)]
pub struct ExtractResult {
    pub archive_path: PathBuf,
    pub target_dir: PathBuf,
    pub extracted_paths: Vec<PathBuf>,
}

pub fn extract(path: &Path) -> Result<ExtractResult> {
    let target_dir = path.with_extension("");
    extract_to(path, &target_dir)
}

fn extract_to(path: &Path, target_dir: &Path) -> Result<ExtractResult> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .context("Invalid extension")?
        .to_lowercase();

    let extractor = EXTRACTORS
        .iter()
        .find(|e| e.can_handle(&extension))
        .context("No extractor found")?;

    if !target_dir.exists() {
        std::fs::create_dir_all(target_dir)?;
    }

    let extracted_paths = extractor.extract(path, target_dir)?;

    Ok(ExtractResult {
        archive_path: path.to_path_buf(),
        target_dir: target_dir.to_path_buf(),
        extracted_paths,
    })
}
