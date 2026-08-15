use std::path::PathBuf;

use reqwest::blocking::Client;

use crate::dependency::UpdatePolicy;
use crate::dependency::cache;
use crate::dependency::resolver::{ResolveContext, ResolvedArtifact, ResolvedDependency};
use crate::maven::repository::{self, ArtifactVersion};
use crate::model::LockfileArtifact;
use crate::workspace::{download, files, paths};

const LOCKFILE_SOURCE: &str = "maven";

pub struct MavenResolveRequest<'a> {
    pub name: &'a str,
    pub url: &'a str,
    pub group_id: &'a str,
    pub artifact_id: &'a str,
    pub version: Option<&'a String>,
    pub classifier: Option<&'a String>,
    pub update_policy: &'a UpdatePolicy,
}

pub fn resolve(
    request: MavenResolveRequest<'_>,
    context: &ResolveContext<'_>,
) -> Result<ResolvedDependency, String> {
    if !context.should_update(request.update_policy) {
        if let Some(locked_artifact) = context.locked_artifact() {
            return resolve_locked_artifact(request.name, locked_artifact);
        }

        return resolve_without_update(request);
    }

    let target_version = match request.version {
        Some(version) => match version.as_str() {
            "latest" => ArtifactVersion::Latest,
            "release" => ArtifactVersion::Release,
            _ => ArtifactVersion::Version {
                version: version.to_string(),
            },
        },
        None => ArtifactVersion::Latest,
    };

    let client: Client = Client::builder()
        .user_agent(download::USER_AGENT)
        .build()
        .map_err(|e| format!("Could not create Maven repository client: {e}"))?;

    let version = match repository::get_version(
        &client,
        request.url,
        request.group_id,
        request.artifact_id,
        request.classifier,
        &target_version,
    ) {
        Ok(t) => t,
        Err(e) => {
            return Err(format!(
                "Failed to get Maven repository version information: {e}"
            ));
        }
    };

    let filepath = cache::maven_cache_path(
        request.group_id,
        request.artifact_id,
        &version.0,
        version.1.as_deref(),
        request.classifier,
    );
    let path: PathBuf = PathBuf::from(&filepath);

    paths::ensure_parents(&filepath)?;

    let target_url = match repository::get_artifact(
        &client,
        request.url,
        request.group_id,
        request.artifact_id,
        request.classifier,
        &target_version,
    ) {
        Ok(t) => t,
        Err(e) => return Err(format!("Failed to get Maven repository artifact: {e}")),
    };

    if path.exists() {
        return resolve_cached_artifact(
            request.name,
            path,
            version.0.as_str(),
            target_url,
            filepath,
        );
    }

    download::download_silent(
        request.artifact_id.to_string(),
        target_url.clone(),
        filepath.clone(),
    )?;
    resolve_cached_artifact(request.name, path, version.0.as_str(), target_url, filepath)
}

fn resolve_without_update(request: MavenResolveRequest<'_>) -> Result<ResolvedDependency, String> {
    let Some(version) = explicit_static_version(request.version) else {
        return Err(format!(
            "Maven dependency \"{}\" does not have a locked artifact or explicit non-SNAPSHOT version, and no update was requested.\nFix: run `wisteria update {}` to resolve the Maven version, download it, and write it to wisteria.lock.",
            request.name, request.name
        ));
    };

    let filepath = cache::maven_cache_path(
        request.group_id,
        request.artifact_id,
        version,
        None,
        request.classifier,
    );
    let path = PathBuf::from(&filepath);

    if !path.exists() {
        return Err(format!(
            "Maven dependency \"{}\" is not cached at `{filepath}`, and no update was requested.\nFix: run `wisteria fetch {}` if it is already in wisteria.lock, or `wisteria update {}` to resolve, lock, and download it.",
            request.name, request.name, request.name
        ));
    }

    let target_url = maven_artifact_url(
        request.url,
        request.group_id,
        request.artifact_id,
        version,
        version,
        request.classifier,
    );

    resolve_cached_artifact(request.name, path, version, target_url, filepath)
}

fn explicit_static_version(version: Option<&String>) -> Option<&str> {
    let version = version?;
    match version.as_str() {
        "latest" | "release" => None,
        version if version.ends_with("-SNAPSHOT") => None,
        version => Some(version),
    }
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

    if locked_artifact.version().is_none() {
        return Err(format!(
            "Lockfile artifact for Maven dependency \"{name}\" does not include a resolved artifact version.\nFix: run `wisteria update {name}` to resolve the version and regenerate `{}`.",
            crate::util::consts::LOCKFILE,
        ));
    }

    let path = PathBuf::from(locked_artifact.cache_path());
    if !path.exists() {
        return Err(format!(
            "Locked Maven dependency \"{name}\" is not cached at `{}`.\nFix: run `wisteria fetch {name}` to download the artifact recorded in `{}`.",
            locked_artifact.cache_path(),
            crate::util::consts::LOCKFILE
        ));
    }

    let hash = files::generate_sha2_for_file(&path)?;
    if hash != locked_artifact.hash() {
        return Err(format!(
            "Cached Maven dependency \"{name}\" at `{}` does not match the hash in `{}`.\nFix: run `wisteria fetch {name}` to restore the locked artifact, or `wisteria update {name}` if you intended to move to a newer artifact.",
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
    version: &str,
    fetch_url: String,
    filepath: String,
) -> Result<ResolvedDependency, String> {
    let hash = files::generate_sha2_for_file(&path)?;
    let lock = LockfileArtifact::new(
        String::from(name),
        String::from(LOCKFILE_SOURCE),
        Some(String::from(version)),
        fetch_url,
        filepath,
        hash,
    );

    Ok(ResolvedDependency::new(
        String::from(name),
        vec![ResolvedArtifact::new(path, Some(lock))],
    ))
}

fn maven_artifact_url(
    repository_url: &str,
    group_id: &str,
    artifact_id: &str,
    version: &str,
    artifact_value: &str,
    classifier: Option<&String>,
) -> String {
    let url_postfix = if repository_url.ends_with('/') {
        ""
    } else {
        "/"
    };
    let classifier_postfix = classifier
        .map(|classifier| format!("-{classifier}"))
        .unwrap_or_default();

    format!(
        "{repository_url}{url_postfix}{}/{}/{version}/{artifact_id}-{artifact_value}{classifier_postfix}.jar",
        group_id.replace(".", "/"),
        artifact_id.replace(".", "/")
    )
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::*;
    use crate::dependency::UpdateContext;
    use crate::test_support::{TempDir, with_current_dir};
    use crate::workspace::files;

    const CACHE_PATH: &str = ".wisteria/cache/com.example/library/1.0.0/library.jar";
    const FETCH_URL: &str = "https://repo.example/com/example/library/1.0.0/library-1.0.0.jar";

    fn request<'a>(
        version: Option<&'a String>,
        update_policy: &'a UpdatePolicy,
    ) -> MavenResolveRequest<'a> {
        MavenResolveRequest {
            name: "library",
            url: "https://repo.example",
            group_id: "com.example",
            artifact_id: "library",
            version,
            classifier: None,
            update_policy,
        }
    }

    fn create_cached_artifact(contents: &str) {
        let path = PathBuf::from(CACHE_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn resolve_without_update_uses_explicit_cached_artifact() {
        let temp = TempDir::new("maven-explicit-no-update");
        let version = String::from("1.0.0");

        with_current_dir(temp.path(), || {
            create_cached_artifact("cached");

            let resolved = resolve(
                request(Some(&version), &UpdatePolicy::Never),
                &ResolveContext::new(UpdateContext::ResolveOnly),
            )
            .unwrap();

            assert_eq!(resolved.name, "library");
            assert_eq!(resolved.artifacts.len(), 1);

            let artifact = &resolved.artifacts[0];
            assert_eq!(artifact.path, PathBuf::from(CACHE_PATH));

            let lock = artifact.lock.as_ref().unwrap();
            assert_eq!(lock.name(), "library");
            assert_eq!(lock.source(), "maven");
            assert_eq!(lock.version(), Some("1.0.0"));
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
        let temp = TempDir::new("maven-locked-no-update");

        with_current_dir(temp.path(), || {
            create_cached_artifact("locked");
            let hash = files::generate_sha2_for_file(&PathBuf::from(CACHE_PATH)).unwrap();
            let lock = LockfileArtifact::new(
                String::from("library"),
                String::from("maven"),
                Some(String::from("1.0.0")),
                String::from(FETCH_URL),
                String::from(CACHE_PATH),
                hash,
            );

            let resolved = resolve(
                request(None, &UpdatePolicy::Never),
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
        let temp = TempDir::new("maven-locked-hash-mismatch");

        with_current_dir(temp.path(), || {
            create_cached_artifact("actual contents");
            let lock = LockfileArtifact::new(
                String::from("library"),
                String::from("maven"),
                Some(String::from("1.0.0")),
                String::from(FETCH_URL),
                String::from(CACHE_PATH),
                String::from("expected hash"),
            );

            let error = resolve(
                request(None, &UpdatePolicy::Never),
                &ResolveContext::with_locked_artifact(UpdateContext::ResolveOnly, &lock),
            )
            .unwrap_err();

            assert!(error.contains("does not match the hash"));
        });
    }

    #[test]
    fn resolve_without_update_rejects_dynamic_version_without_lockfile_artifact() {
        let error = resolve(
            request(None, &UpdatePolicy::Never),
            &ResolveContext::new(UpdateContext::ResolveOnly),
        )
        .unwrap_err();

        assert!(error.contains("does not have a locked artifact"));
    }

    #[test]
    fn resolve_without_update_rejects_snapshot_without_lockfile_artifact() {
        let version = String::from("1.0-SNAPSHOT");
        let error = resolve(
            request(Some(&version), &UpdatePolicy::Never),
            &ResolveContext::new(UpdateContext::ResolveOnly),
        )
        .unwrap_err();

        assert!(error.contains("explicit non-SNAPSHOT version"));
    }

    #[test]
    fn builds_maven_artifact_url_with_classifier() {
        let classifier = String::from("shaded");

        assert_eq!(
            maven_artifact_url(
                "https://repo.example/",
                "com.example",
                "library",
                "1.0.0",
                "1.0.0",
                Some(&classifier),
            ),
            "https://repo.example/com/example/library/1.0.0/library-1.0.0-shaded.jar"
        );
    }
}
