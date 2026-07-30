use std::path::PathBuf;

use crate::dependency::cache;
use crate::dependency::{UpdateContext, UpdatePolicy};
use crate::workspace::{download, paths};

pub fn resolve(
    name: &str,
    url: &str,
    update_policy: &UpdatePolicy,
    update: &UpdateContext,
) -> Result<Vec<PathBuf>, String> {
    let filepath = cache::url_cache_path(name);

    if update_policy.should_update(update) {
        paths::ensure_parents(&filepath)?;
        download::download(name.to_string(), url.to_string(), filepath.clone())?;
    } else {
        println!("Not updating");
    }

    Ok(vec![PathBuf::from(filepath)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_without_update_returns_expected_cache_path_without_downloading() {
        let paths = resolve(
            "library",
            "https://example.com/library.jar",
            &UpdatePolicy::Never,
            &UpdateContext::ResolveOnly,
        )
        .unwrap();

        assert_eq!(
            paths,
            vec![PathBuf::from(".wisteria/cache/library/library.jar")]
        );
    }
}
