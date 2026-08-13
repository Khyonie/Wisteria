use std::path::PathBuf;

use reqwest::blocking::Client;
use serde::Deserialize;

use crate::dependency::cache;
use crate::dependency::resolver::{ResolveContext, ResolvedArtifact, ResolvedDependency};
use crate::dependency::{GithubReleaseType, UpdatePolicy};
use crate::model::LockfileArtifact;
use crate::workspace::{download, files, paths};

const LOCKFILE_SOURCE: &str = "github";

pub struct GithubResolveRequest<'a> {
    pub name: &'a str,
    pub username: &'a str,
    pub repository: &'a str,
    pub asset: &'a str,
    pub tag: Option<&'a String>,
    pub release_type: &'a GithubReleaseType,
    pub update_policy: &'a UpdatePolicy,
}

pub fn resolve(
    request: GithubResolveRequest<'_>,
    context: &ResolveContext<'_>,
) -> Result<ResolvedDependency, String> {
    if !context.should_update(request.update_policy) {
        if let Some(locked_artifact) = context.locked_artifact() {
            return resolve_locked_artifact(request.name, locked_artifact);
        }

        return resolve_without_update(
            request.name,
            request.username,
            request.repository,
            request.asset,
            request.tag,
        );
    }

    let resolved_tag = match request.tag {
        Some(tag) => tag.clone(),
        None => resolve_latest_tag(request.username, request.repository, request.release_type)?,
    };

    resolve_updated_artifact(
        request.name,
        request.username,
        request.repository,
        request.asset,
        &resolved_tag,
    )
}

fn resolve_without_update(
    name: &str,
    username: &str,
    repository: &str,
    asset: &str,
    tag: Option<&String>,
) -> Result<ResolvedDependency, String> {
    let Some(resolved_tag) = tag else {
        return Err(format!(
            "GitHub dependency \"{name}\" does not have a locked artifact or explicit tag, and no update was requested.\nFix: run `wisteria update {name}` to resolve the release, download it, and write it to wisteria.lock."
        ));
    };

    let filepath = cache::github_cache_path(username, repository, resolved_tag);
    let path = PathBuf::from(&filepath);

    if !path.exists() {
        return Err(format!(
            "GitHub dependency \"{name}\" is not cached at `{filepath}`, and no update was requested.\nFix: run `wisteria fetch {name}` if it is already in wisteria.lock, or `wisteria update {name}` to resolve, lock, and download it."
        ));
    }

    let asset = release_asset_filename(asset);
    let full_url = github_release_url(username, repository, resolved_tag, &asset);
    resolve_cached_artifact(name, path, resolved_tag, full_url, filepath)
}

fn resolve_updated_artifact(
    name: &str,
    username: &str,
    repository: &str,
    asset: &str,
    resolved_tag: &str,
) -> Result<ResolvedDependency, String> {
    let filepath = cache::github_cache_path(username, repository, resolved_tag);
    let path = PathBuf::from(&filepath);
    paths::ensure_parents(&filepath)?;

    let asset = release_asset_filename(asset);
    let full_url = github_release_url(username, repository, resolved_tag, &asset);

    if path.exists() {
        println!("Nothing to do");
        return resolve_cached_artifact(name, path, resolved_tag, full_url, filepath);
    }

    download::download(repository.to_string(), full_url.clone(), filepath.clone())?;
    resolve_cached_artifact(name, path, resolved_tag, full_url, filepath)
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
            "Lockfile artifact for GitHub dependency \"{name}\" does not include a resolved release tag.\nFix: run `wisteria update {name}` to resolve the release and regenerate `{}`.",
            crate::util::consts::LOCKFILE,
        ));
    }

    let path = PathBuf::from(locked_artifact.cache_path());
    if !path.exists() {
        return Err(format!(
            "Locked GitHub dependency \"{name}\" is not cached at `{}`.\nFix: run `wisteria fetch {name}` to download the artifact recorded in `{}`.",
            locked_artifact.cache_path(),
            crate::util::consts::LOCKFILE
        ));
    }

    let hash = files::generate_sha2_for_file(&path)?;
    if hash != locked_artifact.hash() {
        return Err(format!(
            "Cached GitHub dependency \"{name}\" at `{}` does not match the hash in `{}`.\nFix: run `wisteria fetch {name}` to restore the locked artifact, or `wisteria update {name}` if you intended to move to a newer artifact.",
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
    resolved_tag: &str,
    full_url: String,
    filepath: String,
) -> Result<ResolvedDependency, String> {
    let hash = files::generate_sha2_for_file(&path)?;
    let lock = LockfileArtifact::new(
        String::from(name),
        String::from(LOCKFILE_SOURCE),
        Some(String::from(resolved_tag)),
        full_url,
        filepath,
        hash,
    );

    Ok(ResolvedDependency::new(
        String::from(name),
        vec![ResolvedArtifact::new(path, Some(lock))],
    ))
}

fn github_release_url(username: &str, repository: &str, tag: &str, asset: &str) -> String {
    format!("https://github.com/{username}/{repository}/releases/download/{tag}/{asset}")
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    prerelease: bool,
    draft: bool,
}

fn resolve_latest_tag(
    username: &str,
    repository: &str,
    release_type: &GithubReleaseType,
) -> Result<String, String> {
    let url = format!("https://api.github.com/repos/{username}/{repository}/releases?per_page=100");
    let client = Client::builder()
        .user_agent(download::USER_AGENT)
        .build()
        .map_err(|e| format!("Could not create GitHub API client: {e}"))?;

    let response = client
        .get(&url)
        .send()
        .map_err(|e| format!("Could not reach GitHub API at URL {url}: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "GitHub API responded with status {} for {username}/{repository}",
            response.status().as_str()
        ));
    }

    let text = response.text().map_err(|e| {
        format!("Failed to read GitHub API response for {username}/{repository}: {e}")
    })?;
    let releases: Vec<GithubRelease> = serde_json::from_str(&text).map_err(|e| {
        format!("Could not decode GitHub releases for {username}/{repository}: {e}")
    })?;

    select_latest_tag(&releases, release_type)
        .ok_or_else(|| missing_release_message(username, repository, release_type))
}

fn select_latest_tag(
    releases: &[GithubRelease],
    release_type: &GithubReleaseType,
) -> Option<String> {
    releases
        .iter()
        .filter(|release| !release.draft)
        .find(|release| match release_type {
            GithubReleaseType::Release => !release.prerelease,
            GithubReleaseType::Prerelease => release.prerelease,
            GithubReleaseType::Any => true,
        })
        .map(|release| release.tag_name.clone())
}

fn missing_release_message(
    username: &str,
    repository: &str,
    release_type: &GithubReleaseType,
) -> String {
    match release_type {
        GithubReleaseType::Release => {
            format!("No non-prerelease GitHub releases found for {username}/{repository}")
        }
        GithubReleaseType::Prerelease => {
            format!("No prerelease GitHub releases found for {username}/{repository}")
        }
        GithubReleaseType::Any => format!("No GitHub releases found for {username}/{repository}"),
    }
}

fn release_asset_filename(asset: &str) -> String {
    if asset.ends_with(".jar") {
        asset.to_string()
    } else {
        format!("{asset}.jar")
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::workspace::files;
    use crate::{
        dependency::UpdateContext,
        test_support::{TempDir, with_current_dir},
    };

    use super::*;

    const CACHE_PATH: &str = ".wisteria/cache/Owner/Repository/v1.0.0/Repository.jar";
    const FETCH_URL: &str =
        "https://github.com/Owner/Repository/releases/download/v1.0.0/Repository.jar";

    fn release(tag_name: &str, prerelease: bool, draft: bool) -> GithubRelease {
        GithubRelease {
            tag_name: tag_name.to_string(),
            prerelease,
            draft,
        }
    }

    #[test]
    fn selects_latest_non_prerelease_by_default() {
        let releases = vec![
            release("v2.0.0-beta.1", true, false),
            release("v1.9.0", false, false),
        ];

        assert_eq!(
            select_latest_tag(&releases, &GithubReleaseType::Release).as_deref(),
            Some("v1.9.0")
        );
    }

    #[test]
    fn can_select_latest_prerelease() {
        let releases = vec![
            release("v2.0.0-beta.1", true, false),
            release("v1.9.0", false, false),
        ];

        assert_eq!(
            select_latest_tag(&releases, &GithubReleaseType::Prerelease).as_deref(),
            Some("v2.0.0-beta.1")
        );
    }

    #[test]
    fn skips_draft_releases() {
        let releases = vec![
            release("v2.0.0", false, true),
            release("v1.9.0", false, false),
        ];

        assert_eq!(
            select_latest_tag(&releases, &GithubReleaseType::Any).as_deref(),
            Some("v1.9.0")
        );
    }

    #[test]
    fn appends_jar_extension_only_when_missing() {
        assert_eq!(release_asset_filename("library"), "library.jar");
        assert_eq!(release_asset_filename("library.jar"), "library.jar");
    }

    fn create_cached_artifact(contents: &str) {
        let path = PathBuf::from(CACHE_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn resolve_with_explicit_tag_and_no_update_uses_cached_artifact_without_network() {
        let temp = TempDir::new("github-explicit-no-update");
        let tag = String::from("v1.0.0");

        with_current_dir(temp.path(), || {
            create_cached_artifact("cached");

            let resolved = resolve(
                GithubResolveRequest {
                    name: "library",
                    username: "Owner",
                    repository: "Repository",
                    asset: "Repository",
                    tag: Some(&tag),
                    release_type: &GithubReleaseType::Release,
                    update_policy: &UpdatePolicy::Never,
                },
                &ResolveContext::new(UpdateContext::ResolveOnly),
            )
            .unwrap();

            assert_eq!(resolved.name, "library");
            assert_eq!(resolved.artifacts.len(), 1);

            let artifact = &resolved.artifacts[0];
            assert_eq!(artifact.path, PathBuf::from(CACHE_PATH));

            let lock = artifact.lock.as_ref().unwrap();
            assert_eq!(lock.name(), "library");
            assert_eq!(lock.source(), "github");
            assert_eq!(lock.version(), Some("v1.0.0"));
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
    fn resolve_with_lockfile_artifact_and_no_update_does_not_require_tag() {
        let temp = TempDir::new("github-locked-no-update");

        with_current_dir(temp.path(), || {
            create_cached_artifact("locked");
            let hash = files::generate_sha2_for_file(&PathBuf::from(CACHE_PATH)).unwrap();
            let lock = LockfileArtifact::new(
                String::from("library"),
                String::from("github"),
                Some(String::from("v1.0.0")),
                String::from(FETCH_URL),
                String::from(CACHE_PATH),
                hash,
            );

            let resolved = resolve(
                GithubResolveRequest {
                    name: "library",
                    username: "Owner",
                    repository: "Repository",
                    asset: "Repository",
                    tag: None,
                    release_type: &GithubReleaseType::Release,
                    update_policy: &UpdatePolicy::Never,
                },
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
        let temp = TempDir::new("github-locked-hash-mismatch");

        with_current_dir(temp.path(), || {
            create_cached_artifact("actual contents");
            let lock = LockfileArtifact::new(
                String::from("library"),
                String::from("github"),
                Some(String::from("v1.0.0")),
                String::from(FETCH_URL),
                String::from(CACHE_PATH),
                String::from("expected hash"),
            );

            let error = resolve(
                GithubResolveRequest {
                    name: "library",
                    username: "Owner",
                    repository: "Repository",
                    asset: "Repository",
                    tag: None,
                    release_type: &GithubReleaseType::Release,
                    update_policy: &UpdatePolicy::Never,
                },
                &ResolveContext::with_locked_artifact(UpdateContext::ResolveOnly, &lock),
            )
            .unwrap_err();

            assert!(error.contains("does not match the hash"));
        });
    }

    #[test]
    fn resolve_without_update_rejects_implicit_latest_without_lockfile_artifact() {
        let error = resolve(
            GithubResolveRequest {
                name: "library",
                username: "Owner",
                repository: "Repository",
                asset: "Repository",
                tag: None,
                release_type: &GithubReleaseType::Release,
                update_policy: &UpdatePolicy::Never,
            },
            &ResolveContext::new(UpdateContext::ResolveOnly),
        )
        .unwrap_err();

        assert!(error.contains("does not have a locked artifact or explicit tag"));
    }
}
