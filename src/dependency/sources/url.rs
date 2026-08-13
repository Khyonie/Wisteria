use std::path::PathBuf;

use crate::dependency::UpdatePolicy;
use crate::dependency::cache;
use crate::dependency::resolver::{ResolveContext, ResolvedArtifact, ResolvedDependency};
use crate::model::LockfileArtifact;
use crate::workspace::{download, files, paths};

const LOCKFILE_SOURCE: &str = "url";

pub fn resolve(
    name: &str,
    url: &str,
    update_policy: &UpdatePolicy,
    context: &ResolveContext<'_>,
) -> Result<ResolvedDependency, String> {
    let filepath = cache::url_cache_path(name);
    let path = PathBuf::from(&filepath);

    if context.should_update(update_policy) {
        paths::ensure_parents(&filepath)?;
        download::download(name.to_string(), url.to_string(), filepath.clone())?;
        return resolve_cached_artifact(name, path, url.to_string(), filepath);
    }

    if let Some(locked_artifact) = context.locked_artifact() {
        return resolve_locked_artifact(name, locked_artifact);
    }

    if path.exists() {
        return resolve_cached_artifact(name, path, url.to_string(), filepath);
    }

    Err(format!(
        "URL dependency \"{name}\" is not cached at `{filepath}`, and no update was requested.\nFix: run `wisteria fetch {name}` if it is already in wisteria.lock, or `wisteria update {name}` to resolve, lock, and download it."
    ))
}

fn resolve_locked_artifact(
    name: &str,
    locked_artifact: &LockfileArtifact,
) -> Result<ResolvedDependency, String> {
    if locked_artifact.name() != name {
        return Err(format!(
            "Lockfile artifact for dependency \"{name}\" is named \"{}\".\nFix: run `wisteria sync {name}` if project.toml is current, or `wisteria update {name}` to resolve and download it again.",
            locked_artifact.name(),
        ));
    }

    if locked_artifact.source() != LOCKFILE_SOURCE {
        return Err(format!(
            "Lockfile artifact for dependency \"{name}\" has source \"{}\", expected \"{LOCKFILE_SOURCE}\".\nFix: run `wisteria sync {name}` if project.toml is current, or `wisteria update {name}` to resolve and download it again.",
            locked_artifact.source(),
        ));
    }

    let path = PathBuf::from(locked_artifact.cache_path());
    if !path.exists() {
        return Err(format!(
            "Locked URL dependency \"{name}\" is not cached at `{}`.\nFix: run `wisteria fetch {name}` to download the artifact recorded in `{}`.",
            locked_artifact.cache_path(),
            crate::util::consts::LOCKFILE
        ));
    }

    let hash = files::generate_sha2_for_file(&path)?;
    if hash != locked_artifact.hash() {
        return Err(format!(
            "Cached URL dependency \"{name}\" at `{}` does not match the hash in `{}`.\nFix: run `wisteria fetch {name}` to restore the locked artifact, or `wisteria update {name}` if you intended to move to a newer artifact.",
            locked_artifact.cache_path(),
            crate::util::consts::LOCKFILE
        ));
    }

    Ok(ResolvedDependency::new(
        String::from(name),
        vec![ResolvedArtifact::new(path, Some(locked_artifact.clone()))],
    ))
}

fn resolve_cached_artifact(
    name: &str,
    path: PathBuf,
    fetch_url: String,
    filepath: String,
) -> Result<ResolvedDependency, String> {
    let hash = files::generate_sha2_for_file(&path)?;
    let lock = LockfileArtifact::new(
        String::from(name),
        String::from(LOCKFILE_SOURCE),
        None,
        fetch_url,
        filepath,
        hash,
    );

    Ok(ResolvedDependency::new(
        String::from(name),
        vec![ResolvedArtifact::new(path, Some(lock))],
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::dependency::UpdateContext;
    use crate::test_support::{TempDir, with_current_dir};
    use crate::workspace::files;

    const CACHE_PATH: &str = ".wisteria/cache/library/library.jar";
    const FETCH_URL: &str = "https://example.com/library.jar";

    fn create_cached_artifact(contents: &str) {
        let path = PathBuf::from(CACHE_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn resolve_without_update_uses_cached_artifact() {
        let temp = TempDir::new("url-explicit-no-update");

        with_current_dir(temp.path(), || {
            create_cached_artifact("cached");

            let resolved = resolve(
                "library",
                FETCH_URL,
                &UpdatePolicy::Never,
                &ResolveContext::new(UpdateContext::ResolveOnly),
            )
            .unwrap();

            assert_eq!(resolved.name, "library");
            assert_eq!(resolved.artifacts.len(), 1);

            let artifact = &resolved.artifacts[0];
            assert_eq!(artifact.path, PathBuf::from(CACHE_PATH));

            let lock = artifact.lock.as_ref().unwrap();
            assert_eq!(lock.name(), "library");
            assert_eq!(lock.source(), "url");
            assert_eq!(lock.version(), None);
            assert_eq!(lock.fetch_url(), FETCH_URL);
            assert_eq!(lock.cache_path(), CACHE_PATH);
            assert_eq!(
                lock.hash(),
                files::generate_sha2_for_file(&PathBuf::from(CACHE_PATH))
                    .unwrap()
                    .as_str()
            );
        });
    }

    #[test]
    fn resolve_with_lockfile_artifact_and_no_update_validates_cached_artifact() {
        let temp = TempDir::new("url-locked-no-update");

        with_current_dir(temp.path(), || {
            create_cached_artifact("locked");
            let hash = files::generate_sha2_for_file(&PathBuf::from(CACHE_PATH)).unwrap();
            let lock = LockfileArtifact::new(
                String::from("library"),
                String::from("url"),
                None,
                String::from(FETCH_URL),
                String::from(CACHE_PATH),
                hash,
            );

            let resolved = resolve(
                "library",
                FETCH_URL,
                &UpdatePolicy::Never,
                &ResolveContext::with_locked_artifact(UpdateContext::ResolveOnly, &lock),
            )
            .unwrap();

            assert_eq!(resolved.name, "library");
            assert_eq!(resolved.artifacts.len(), 1);
            assert_eq!(resolved.artifacts[0].path, PathBuf::from(CACHE_PATH));
            assert_eq!(resolved.artifacts[0].lock.as_ref(), Some(&lock));
        });
    }

    #[test]
    fn resolve_with_lockfile_artifact_rejects_hash_mismatch() {
        let temp = TempDir::new("url-locked-hash-mismatch");

        with_current_dir(temp.path(), || {
            create_cached_artifact("actual contents");
            let lock = LockfileArtifact::new(
                String::from("library"),
                String::from("url"),
                None,
                String::from(FETCH_URL),
                String::from(CACHE_PATH),
                String::from("expected hash"),
            );

            let error = resolve(
                "library",
                FETCH_URL,
                &UpdatePolicy::Never,
                &ResolveContext::with_locked_artifact(UpdateContext::ResolveOnly, &lock),
            )
            .unwrap_err();

            assert!(error.contains("does not match the hash"));
        });
    }

    #[test]
    fn resolve_without_update_rejects_missing_cache() {
        let temp = TempDir::new("url-missing-cache");

        with_current_dir(temp.path(), || {
            let error = resolve(
                "library",
                FETCH_URL,
                &UpdatePolicy::Never,
                &ResolveContext::new(UpdateContext::ResolveOnly),
            )
            .unwrap_err();

            assert!(error.contains("is not cached"));
        });
    }
}
