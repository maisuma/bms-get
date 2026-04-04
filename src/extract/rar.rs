use super::Extractor;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use unrar::Archive;

pub struct RarExtractor;

impl Extractor for RarExtractor {
    fn can_handle(&self, ext: &str) -> bool {
        ext == "rar"
    }

    fn extract(&self, archive_path: &Path, target_dir: &Path) -> Result<()> {
        let mut archive = Archive::new(archive_path).open_for_processing()?;

        while let Some(header) = archive.read_header()? {
            archive = if header.entry().is_file() {
                header.extract_with_base(target_dir)?
            } else {
                header.skip()?
            };
        }

        Ok(())
    }
}
