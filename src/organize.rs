use anyhow::Result;
use log::{debug, warn};
use std::fs;
use std::path::{Path, PathBuf};

use crate::extract::ExtractResult;

const CHART_EXTENSIONS: &[&str] = &["bms", "bme", "bml", "pms"];
const AUDIO_EXTENSIONS: &[&str] = &["wav", "ogg", "mp3"];

pub fn merge_extract(bundle_root: &mut Option<PathBuf>, extracted: &ExtractResult) -> Result<()> {
    let source_root = infer_root(extracted);

    let Some(target_root) = bundle_root.as_ref() else {
        debug!("Bundle root initialized: {}", source_root.display());
        *bundle_root = Some(source_root);
        cleanup_archive(&extracted.archive_path);
        return Ok(());
    };

    if source_root == *target_root {
        cleanup_archive(&extracted.archive_path);
        return Ok(());
    }

    debug!(
        "Merging extracted root {} into {}",
        source_root.display(),
        target_root.display()
    );
    merge_dir_contents(&source_root, target_root)?;
    cleanup_extract_dir(&extracted.target_dir);
    cleanup_archive(&extracted.archive_path);

    Ok(())
}

fn infer_root(extracted: &ExtractResult) -> PathBuf {
    if let Some(root) = shallowest_root(extracted, CHART_EXTENSIONS) {
        return root;
    }

    if let Some(root) = shallowest_root(extracted, AUDIO_EXTENSIONS) {
        return root;
    }

    extracted.target_dir.clone()
}

fn shallowest_root(extracted: &ExtractResult, extensions: &[&str]) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    let mut min_depth: Option<usize> = None;

    for path in &extracted.extracted_paths {
        if !has_extension(path, extensions) {
            continue;
        }

        let root = path.parent().unwrap_or(&extracted.target_dir).to_path_buf();
        let depth = root
            .strip_prefix(&extracted.target_dir)
            .map(component_depth)
            .unwrap_or(usize::MAX);

        match min_depth {
            None => {
                min_depth = Some(depth);
                candidates.push(root);
            }
            Some(current) if depth < current => {
                min_depth = Some(depth);
                candidates.clear();
                candidates.push(root);
            }
            Some(current) if depth == current => {
                candidates.push(root);
            }
            _ => {}
        }
    }

    common_ancestor(candidates)
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            extensions
                .iter()
                .any(|candidate| ext.eq_ignore_ascii_case(candidate))
        })
}

fn component_depth(path: &Path) -> usize {
    path.components().count()
}

fn common_ancestor(paths: Vec<PathBuf>) -> Option<PathBuf> {
    let mut paths = paths.into_iter();
    let mut ancestor = paths.next()?;

    for path in paths {
        while !path.starts_with(&ancestor) {
            if !ancestor.pop() {
                return None;
            }
        }
    }

    Some(ancestor)
}

fn merge_dir_contents(source: &Path, target: &Path) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }

    fs::create_dir_all(target)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());

        merge_entry(&source_path, &target_path)?;
    }

    Ok(())
}

fn merge_entry(source: &Path, target: &Path) -> Result<()> {
    if !target.exists() {
        fs::rename(source, target)?;
        return Ok(());
    }

    if source.is_dir() && target.is_dir() {
        merge_dir_contents(source, target)?;
        return Ok(());
    }

    warn!(
        "Skipping merge because target already exists: {} -> {}",
        source.display(),
        target.display()
    );
    Ok(())
}

fn cleanup_extract_dir(path: &Path) {
    if path.is_dir()
        && let Err(e) = fs::remove_dir_all(path)
    {
        warn!(
            "Failed to remove extract directory: {} - {}",
            path.display(),
            e
        );
    }
}

fn cleanup_archive(archive_path: &Path) {
    if archive_path.is_file()
        && let Err(e) = fs::remove_file(archive_path)
    {
        warn!(
            "Failed to remove archive after extraction: {} - {}",
            archive_path.display(),
            e
        );
    }
}
