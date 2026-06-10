use anyhow::Result;
use std::path::Path;

use super::analyze::analyze_dir;

const MIN_REF_MATCH_PERCENT: usize = 90;

pub fn validate_md5(md5: &str, root: &Path) -> Result<bool> {
    let dir = analyze_dir(root)?;

    Ok(dir
        .charts
        .iter()
        .any(|chart| chart.md5.eq_ignore_ascii_case(md5)))
}

pub fn validate_ref(md5: &str, root: &Path) -> Result<bool> {
    let dir = analyze_dir(root)?;

    for chart in &dir.charts {
        if !chart.md5.eq_ignore_ascii_case(md5) {
            continue;
        }

        let found = chart
            .refs
            .iter()
            .filter(|file| ref_exists(&chart.root.join(file)))
            .count();

        return Ok(found * 100 >= chart.refs.len() * MIN_REF_MATCH_PERCENT);
    }

    Ok(false)
}

fn ref_exists(path: &Path) -> bool {
    if path.exists() {
        return true;
    }

    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
    {
        return path.with_extension("ogg").exists();
    }

    false
}
