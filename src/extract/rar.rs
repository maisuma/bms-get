use super::Extractor;
use anyhow::{Result, bail};
use std::path::{Component, Path, PathBuf};
use unrar::Archive;

pub struct RarExtractor;

impl Extractor for RarExtractor {
    fn can_handle(&self, ext: &str) -> bool {
        ext == "rar"
    }

    fn extract(&self, archive_path: &Path, target_dir: &Path) -> Result<Vec<PathBuf>> {
        let mut archive = Archive::new(archive_path).open_for_processing()?;
        let mut extracted_paths = Vec::new();

        while let Some(header) = archive.read_header()? {
            let filename = &header.entry().filename;
            if filename.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            }) {
                bail!("Unsafe RAR entry path: {}", filename.display());
            }

            let entry_path = target_dir.join(filename);
            archive = if header.entry().is_file() {
                header.extract_with_base(target_dir)?
            } else {
                header.skip()?
            };
            extracted_paths.push(entry_path);
        }

        Ok(extracted_paths)
    }
}
