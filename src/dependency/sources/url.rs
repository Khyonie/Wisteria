use std::path::PathBuf;

use crate::dependency::cache;
use crate::dependency::{UpdateContext, UpdatePolicy};
use crate::workspace::{download, paths};

pub fn resolve(
    name: &str,
    url: &str,
    update_policy: &UpdatePolicy,
    update: &UpdateContext,
) -> Result<Vec<PathBuf>, (String, u8)> {
    let filepath = cache::url_cache_path(name);

    if update_policy.should_update(update) {
        paths::ensure_parents(&filepath).map_err(|e| (e, 1))?;
        download::download(name.to_string(), url.to_string(), filepath.clone())?;
    } else {
        println!("Not updating");
    }

    Ok(vec![PathBuf::from(filepath)])
}
