use std::path::Path;
use anyhow::{Context, Result};

pub mod zip;
pub mod rar;

pub trait Extractor: Send + Sync {
    fn can_handle(&self, ext: &str) -> bool;
    fn extract(&self, archive_path: &Path, target_dir: &Path) -> Result<()>;
}

pub fn extract(path: &Path) -> Result<()> {
    let extension = path.extension().and_then(|e| e.to_str()).context("Invalid extension")?.to_lowercase();

    let extractors: Vec<Box<dyn Extractor>> = vec![
        Box::new(zip::ZipExtractor),
        Box::new(rar::RarExtractor),
    ]; 
    
    let extractor = extractors.iter().find(|e| e.can_handle(&extension)).context("No extractor found")?;
    
    let target_dir = path.with_extension("");
    if !target_dir.exists() {
        std::fs::create_dir_all(&target_dir)?;
    }

    extractor.extract(path, target_dir.as_path())?;

    Ok(())
}