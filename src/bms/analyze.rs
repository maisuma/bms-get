use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct BmsDir {
    pub root: PathBuf,
    pub files: HashSet<PathBuf>,
    pub charts: Vec<BmsChart>,
}

#[derive(Debug)]
pub struct BmsChart {
    pub root: PathBuf,
    pub path: PathBuf,
    pub md5: String,
    pub refs: HashSet<PathBuf>,
}

pub fn analyze_dir(path: &Path) -> Result<BmsDir> {
    let mut dir = BmsDir {
        root: path.to_path_buf(),
        files: HashSet::new(),
        charts: Vec::new(),
    };
    scan(path, &mut dir)?;
    Ok(dir)
}

fn scan(path: &Path, dir: &mut BmsDir) -> Result<()> {
    if path.is_file() {
        dir.files.insert(path.to_path_buf());

        if is_chart(path) {
            let root = path.parent().unwrap_or(path).to_path_buf();
            dir.charts.push(BmsChart {
                root,
                path: path.to_path_buf(),
                md5: file_md5(path)?,
                refs: chart_refs(path)?,
            });
        }

        return Ok(());
    }

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            scan(&entry?.path(), dir)?;
        }
    }

    Ok(())
}

pub fn chart_refs(path: &Path) -> Result<HashSet<PathBuf>> {
    let text = String::from_utf8_lossy(&fs::read(path)?).into_owned();
    let mut refs = HashSet::new();

    for line in text.lines() {
        let Some((key, file_name)) = split_command(line.trim()) else {
            continue;
        };

        if key.starts_with("#WAV") || key.starts_with("#BMP") {
            refs.insert(PathBuf::from(file_name));
        }
    }

    Ok(refs)
}

fn split_command(line: &str) -> Option<(&str, &str)> {
    let index = line.find(char::is_whitespace)?;
    let value = line[index..].trim();

    if value.is_empty() {
        None
    } else {
        Some((&line[..index], value))
    }
}

fn is_chart(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext, "bms" | "bme" | "bml" | "pms"))
}

fn file_md5(path: &Path) -> Result<String> {
    Ok(format!("{:x}", md5::compute(fs::read(path)?)))
}
