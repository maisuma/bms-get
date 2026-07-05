use anyhow::Result;
use log::{debug, warn};
use std::fs;
use std::path::{Path, PathBuf};

use super::analyze::{BmsChart, BmsDir};
use super::validation::resolve_ref_path;

const MIN_MATCH_PERCENT: usize = 80;

/// 追加したBMSディレクトリを既存ディレクトリへ結合する。
/// マージが成功するとsourceはすべて削除される。
pub fn merge_bms_dir(dir: &BmsDir, source: &BmsDir) -> Result<bool> {
    let mut merged = false;

    for chart in &source.charts {
        if let Some(target_dir) = best_target(dir, chart) {
            debug!(
                "Merging BMS chart {} into {}",
                chart.path.display(),
                target_dir.display()
            );
            merge_chart(chart, &target_dir)?;
            merged = true;
        }
    }

    if merged {
        cleanup_source_dir(&source.root);
    }

    Ok(merged)
}

/// 追加した譜面の参照ファイルが最も多く揃う既存ディレクトリを探す。
fn best_target(dir: &BmsDir, chart: &BmsChart) -> Option<PathBuf> {
    if chart.refs.is_empty() {
        return None;
    }

    let mut best = None;
    let mut best_score = 0;

    for existing_chart in &dir.charts {
        let target_dir = &existing_chart.root;
        if target_dir == &chart.root {
            continue;
        }

        let score = score(target_dir, chart);
        if score > best_score {
            best_score = score;
            best = Some(target_dir.to_path_buf());
        }
    }

    if best_score * 100 >= chart.refs.len() * MIN_MATCH_PERCENT {
        best
    } else {
        None
    }
}

/// 追加した譜面の参照ファイルが、指定ディレクトリに何個存在するか数える。
fn score(target_dir: &Path, chart: &BmsChart) -> usize {
    chart
        .refs
        .iter()
        .filter(|file| {
            resolve_ref_path(&target_dir.join(file)).is_some()
                || resolve_ref_path(&chart.root.join(file)).is_some()
        })
        .count()
}

/// 1つの譜面と、その譜面が参照するファイルを移す。
fn merge_chart(chart: &BmsChart, target: &Path) -> Result<()> {
    fs::create_dir_all(target)?;

    if let Some(file_name) = chart.path.file_name() {
        merge_entry(&chart.path, &target.join(file_name))?;
    }

    for file in &chart.refs {
        let Some(source_path) = resolve_ref_path(&chart.root.join(file)) else {
            continue;
        };

        // .wavの参照が.oggとして見つかった場合は、実体の拡張子を保ったまま移す。
        let target_path = match source_path.extension() {
            Some(extension) => target.join(file).with_extension(extension),
            None => target.join(file),
        };
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        merge_entry(&source_path, &target_path)?;
    }

    Ok(())
}

/// ファイルを結合する。
fn merge_entry(source: &Path, target: &Path) -> Result<()> {
    if source.is_dir() {
        warn!("Skipping directory: {}", source.display());
        return Ok(());
    }

    if !target.exists() {
        fs::rename(source, target)?;
        return Ok(());
    }

    warn!(
        "Skipping merge because target already exists: {} -> {}",
        source.display(),
        target.display()
    );
    Ok(())
}

/// 結合後に不要になった結合元ディレクトリを消す。
fn cleanup_source_dir(root: &Path) {
    if root.is_dir()
        && let Err(e) = fs::remove_dir_all(root)
    {
        warn!(
            "Failed to remove source directory: {} - {}",
            root.display(),
            e
        );
    }
}
