use std::path::PathBuf;

use reqwest::blocking::Client;
use serde::Deserialize;

use crate::dependency::cache;
use crate::dependency::{GithubReleaseType, UpdateContext, UpdatePolicy};
use crate::workspace::{download, paths};

pub fn resolve(
    username: &str,
    repository: &str,
    asset: &str,
    tag: Option<&String>,
    release_type: &GithubReleaseType,
    update_policy: &UpdatePolicy,
    update: &UpdateContext,
) -> Result<Vec<PathBuf>, (String, u8)> {
    let resolved_tag = match tag {
        Some(tag) => tag.clone(),
        None => resolve_latest_tag(username, repository, release_type)?,
    };

    let filepath = cache::github_cache_path(username, repository, &resolved_tag);
    let path = PathBuf::from(&filepath);

    if update_policy.should_update(update) {
        paths::ensure_parents(&filepath).map_err(|e| (e, 1))?;

        if path.exists() {
            println!("Nothing to do");
            return Ok(vec![path]);
        }

        let asset = release_asset_filename(asset);
        let full_url = format!(
            "https://github.com/{username}/{repository}/releases/download/{resolved_tag}/{asset}"
        );

        download::download(repository.to_string(), full_url, filepath)?;
    } else {
        println!("Not updating");
    }

    Ok(vec![path])
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
) -> Result<String, (String, u8)> {
    let url = format!("https://api.github.com/repos/{username}/{repository}/releases?per_page=100");
    let client = Client::builder()
        .user_agent(download::USER_AGENT)
        .build()
        .map_err(|e| (format!("Could not create GitHub API client: {e}"), 1))?;

    let response = client
        .get(&url)
        .send()
        .map_err(|e| (format!("Could not reach GitHub API at URL {url}: {e}"), 1))?;

    if !response.status().is_success() {
        return Err((
            format!(
                "GitHub API responded with status {} for {username}/{repository}",
                response.status().as_str()
            ),
            1,
        ));
    }

    let text = response.text().map_err(|e| {
        (
            format!("Failed to read GitHub API response for {username}/{repository}: {e}"),
            1,
        )
    })?;
    let releases: Vec<GithubRelease> = serde_json::from_str(&text).map_err(|e| {
        (
            format!("Could not decode GitHub releases for {username}/{repository}: {e}"),
            1,
        )
    })?;

    select_latest_tag(&releases, release_type).ok_or_else(|| {
        (
            missing_release_message(username, repository, release_type),
            1,
        )
    })
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
    use super::*;

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

    #[test]
    fn resolve_with_explicit_tag_and_no_update_returns_cache_path_without_network() {
        let paths = resolve(
            "Owner",
            "Repository",
            "Repository",
            Some(&String::from("v1.0.0")),
            &GithubReleaseType::Release,
            &UpdatePolicy::Never,
            &UpdateContext::ResolveOnly,
        )
        .unwrap();

        assert_eq!(
            paths,
            vec![PathBuf::from(
                ".wisteria/cache/Owner/Repository/v1.0.0/Repository.jar"
            )]
        );
    }
}
