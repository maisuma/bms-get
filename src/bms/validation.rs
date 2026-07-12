use anyhow::Result;
use std::path::{Path, PathBuf};

use super::analyze::{chart_refs, find_chart};

const MIN_REF_MATCH_PERCENT: usize = 90;

pub fn validate_md5(md5: &str, root: &Path) -> Result<bool> {
    Ok(find_chart(root, md5)?.is_some())
}

pub fn validate_ref(md5: &str, root: &Path) -> Result<bool> {
    let Some(chart) = find_chart(root, md5)? else {
        return Ok(false);
    };

    let refs = chart_refs(&chart)?;
    let root = chart.parent().unwrap_or(&chart);
    let found = refs
        .iter()
        .filter(|file| resolve_ref_path(&root.join(file)).is_some())
        .count();

    Ok(found * 100 >= refs.len() * MIN_REF_MATCH_PERCENT)
}

fn resolve_ref_path(path: &Path) -> Option<PathBuf> {
    if path.exists() {
        return Some(path.to_path_buf());
    }

    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
    {
        let ogg_path = path.with_extension("ogg");
        if ogg_path.exists() {
            return Some(ogg_path);
        }
    }

    None
}
