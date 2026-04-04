use super::Extractor;
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};

pub struct ZipExtractor;

impl Extractor for ZipExtractor {
    fn can_handle(&self, ext: &str) -> bool {
       ext == "zip"
    }

    fn extract(&self, archive_path: &Path, target_dir: &Path) -> Result<()> {
        let file = fs::File::open(archive_path).context("Failed to open archive")?;
        let mut archive = zip::ZipArchive::new(file).context("Failed to read zip archive")?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => target_dir.join(path),
                None => continue, 
            };

            if file.is_dir() {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p)?;
                    }
                }
                let mut outfile = fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }
        Ok(())
    }
}