use crate::{
    dependency::{UpdatePolicy, cache},
    model::LockfileArtifact,
    util::consts,
};

#[derive(Clone)]
pub enum Dependency {
    LocalFile {
        path: String,
        javadoc: Option<String>,
    },
    LocalFolder {
        path: String,
        recursive: bool,
    },
    LocalRepository {
        repository: String,
        name: String,
        version: String,
        update_policy: UpdatePolicy,
        javadoc: Option<String>,
    },

    FetchFromUrl {
        url: String,
        update_policy: UpdatePolicy,
        javadoc: Option<String>,
    },
    FetchFromMaven {
        url: String,
        group_id: String,
        artifact_id: String,
        version: Option<String>,
        classifier: Option<String>,
        update_policy: UpdatePolicy,
        javadoc: Option<String>,
    },
    FetchFromGithub {
        username: String,
        repository: String,
        asset: String,
        tag: Option<String>,
        release_type: GithubReleaseType,
        update_policy: UpdatePolicy,
        javadoc: Option<String>,
    },

    BuildFromScript {
        run: Vec<String>,
        target: String,
        update_policy: UpdatePolicy,
        javadoc: Option<String>,
    },
}

impl Dependency {
    pub fn type_str(&self) -> &str {
        match self {
            Dependency::LocalFile { .. } => "loadArchive",
            Dependency::LocalFolder { .. } => "loadFolder",
            Dependency::LocalRepository { .. } => "localRepository",
            Dependency::FetchFromUrl { .. } => "fetchFromUrl",
            Dependency::FetchFromMaven { .. } => "fetchFromMaven",
            Dependency::FetchFromGithub { .. } => "fetchFromGithub",
            Dependency::BuildFromScript { .. } => "buildFromScript",
        }
    }

    pub fn javadoc(&self) -> Option<&String> {
        match self {
            Dependency::LocalFile { javadoc, .. } => javadoc.as_ref(),
            Dependency::LocalRepository { javadoc, .. } => javadoc.as_ref(),
            Dependency::FetchFromUrl { javadoc, .. } => javadoc.as_ref(),
            Dependency::FetchFromMaven { javadoc, .. } => javadoc.as_ref(),
            Dependency::FetchFromGithub { javadoc, .. } => javadoc.as_ref(),
            Dependency::BuildFromScript { javadoc, .. } => javadoc.as_ref(),
            _ => None,
        }
    }

    pub fn lockfile_source(&self) -> Option<&'static str> {
        match self {
            Dependency::FetchFromUrl { .. } => Some("url"),
            Dependency::FetchFromMaven { .. } => Some("maven"),
            Dependency::FetchFromGithub { .. } => Some("github"),
            _ => None,
        }
    }

    pub fn matches_lockfile_artifact(&self, artifact: &LockfileArtifact) -> bool {
        let Some(source) = self.lockfile_source() else {
            return false;
        };

        if artifact.source() != source {
            return false;
        }

        match self {
            Dependency::FetchFromUrl { url, .. } => artifact.fetch_url() == url,
            Dependency::FetchFromMaven {
                url,
                group_id,
                artifact_id,
                version,
                classifier,
                ..
            } => maven_dependency_matches_lockfile(
                MavenLockfileMatch {
                    url,
                    group_id,
                    artifact_id,
                    version: version.as_deref(),
                    classifier: classifier.as_ref(),
                },
                artifact,
            ),
            Dependency::FetchFromGithub {
                username,
                repository,
                asset,
                tag,
                ..
            } => github_dependency_matches_lockfile(
                username,
                repository,
                asset,
                tag.as_deref(),
                artifact,
            ),
            _ => false,
        }
    }
}

struct MavenLockfileMatch<'a> {
    url: &'a str,
    group_id: &'a str,
    artifact_id: &'a str,
    version: Option<&'a str>,
    classifier: Option<&'a String>,
}

fn maven_dependency_matches_lockfile(
    request: MavenLockfileMatch<'_>,
    artifact: &LockfileArtifact,
) -> bool {
    if !configured_version_matches_lockfile(request.version, artifact.version()) {
        return false;
    }

    let Some(locked_version) = artifact.version() else {
        return false;
    };

    if locked_version.ends_with("-SNAPSHOT") {
        return maven_snapshot_fetch_url_matches(
            request.url,
            request.group_id,
            request.artifact_id,
            locked_version,
            request.classifier,
            artifact.fetch_url(),
        ) && maven_snapshot_cache_path_matches(
            request.group_id,
            request.artifact_id,
            locked_version,
            request.classifier,
            artifact.cache_path(),
        );
    }

    artifact.fetch_url()
        == maven_artifact_url(
            request.url,
            request.group_id,
            request.artifact_id,
            locked_version,
            locked_version,
            request.classifier,
        )
        && artifact.cache_path()
            == cache::maven_cache_path(
                request.group_id,
                request.artifact_id,
                locked_version,
                None,
                request.classifier,
            )
}

fn maven_snapshot_fetch_url_matches(
    repository_url: &str,
    group_id: &str,
    artifact_id: &str,
    locked_version: &str,
    classifier: Option<&String>,
    fetch_url: &str,
) -> bool {
    let prefix = format!(
        "{}{}/{}/{locked_version}/{artifact_id}-",
        repository_url_with_trailing_slash(repository_url),
        group_id.replace(".", "/"),
        artifact_id.replace(".", "/")
    );
    let suffix = format!("{}.jar", classifier_postfix(classifier));

    fetch_url.starts_with(&prefix) && fetch_url.ends_with(&suffix)
}

fn maven_snapshot_cache_path_matches(
    group_id: &str,
    artifact_id: &str,
    locked_version: &str,
    classifier: Option<&String>,
    cache_path: &str,
) -> bool {
    if cache_path
        == cache::maven_cache_path(group_id, artifact_id, locked_version, None, classifier)
    {
        return true;
    }

    let prefix = format!(
        "{}/{group_id}/{artifact_id}/{locked_version}/{artifact_id}-",
        consts::CACHE_PATH
    );
    let suffix = format!("{}.jar", classifier_postfix(classifier));

    cache_path.starts_with(&prefix) && cache_path.ends_with(&suffix)
}

fn maven_artifact_url(
    repository_url: &str,
    group_id: &str,
    artifact_id: &str,
    version: &str,
    artifact_value: &str,
    classifier: Option<&String>,
) -> String {
    format!(
        "{}{}/{}/{version}/{artifact_id}-{artifact_value}{}.jar",
        repository_url_with_trailing_slash(repository_url),
        group_id.replace(".", "/"),
        artifact_id.replace(".", "/"),
        classifier_postfix(classifier)
    )
}

fn repository_url_with_trailing_slash(repository_url: &str) -> String {
    if repository_url.ends_with('/') {
        repository_url.to_string()
    } else {
        format!("{repository_url}/")
    }
}

fn classifier_postfix(classifier: Option<&String>) -> String {
    classifier
        .map(|classifier| format!("-{classifier}"))
        .unwrap_or_default()
}

fn github_dependency_matches_lockfile(
    username: &str,
    repository: &str,
    asset: &str,
    tag: Option<&str>,
    artifact: &LockfileArtifact,
) -> bool {
    if !configured_version_matches_lockfile(tag, artifact.version()) {
        return false;
    }

    let Some(locked_tag) = artifact.version() else {
        return false;
    };

    let asset = github_release_asset_filename(asset);
    artifact.fetch_url() == github_release_url(username, repository, locked_tag, &asset)
        && artifact.cache_path() == cache::github_cache_path(username, repository, locked_tag)
}

fn github_release_url(username: &str, repository: &str, tag: &str, asset: &str) -> String {
    format!("https://github.com/{username}/{repository}/releases/download/{tag}/{asset}")
}

fn github_release_asset_filename(asset: &str) -> String {
    if asset.ends_with(".jar") {
        asset.to_string()
    } else {
        format!("{asset}.jar")
    }
}

fn configured_version_matches_lockfile(
    configured_version: Option<&str>,
    locked_version: Option<&str>,
) -> bool {
    match configured_version {
        Some("latest" | "release") | None => locked_version.is_some(),
        Some(version) => locked_version == Some(version),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GithubReleaseType {
    Release,
    Prerelease,
    Any,
}

impl GithubReleaseType {
    pub fn load(value: &str) -> Result<Self, String> {
        match value {
            "release" => Ok(Self::Release),
            "prerelease" | "pre-release" => Ok(Self::Prerelease),
            "any" => Ok(Self::Any),
            _ => Err(format!(
                "Unexpected GitHub release type, expected one of [release, prerelease, pre-release, any], found {value}"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn maven_dependency(version: Option<&str>, artifact_id: &str) -> Dependency {
        Dependency::FetchFromMaven {
            url: String::from("https://repo.example"),
            group_id: String::from("com.example"),
            artifact_id: String::from(artifact_id),
            version: version.map(String::from),
            classifier: None,
            update_policy: UpdatePolicy::Never,
            javadoc: None,
        }
    }

    fn maven_artifact(version: &str, fetch_url: &str, cache_path: &str) -> LockfileArtifact {
        LockfileArtifact::new(
            String::from("library"),
            String::from("maven"),
            Some(String::from(version)),
            String::from(fetch_url),
            String::from(cache_path),
            String::from("hash"),
        )
    }

    fn github_dependency(tag: Option<&str>, repository: &str) -> Dependency {
        Dependency::FetchFromGithub {
            username: String::from("Owner"),
            repository: String::from(repository),
            asset: String::from("library"),
            tag: tag.map(String::from),
            release_type: GithubReleaseType::Release,
            update_policy: UpdatePolicy::Never,
            javadoc: None,
        }
    }

    fn github_artifact(version: &str, fetch_url: &str, cache_path: &str) -> LockfileArtifact {
        LockfileArtifact::new(
            String::from("library"),
            String::from("github"),
            Some(String::from(version)),
            String::from(fetch_url),
            String::from(cache_path),
            String::from("hash"),
        )
    }

    #[test]
    fn maven_dependency_matches_lockfile_artifact_for_same_coordinates() {
        let dependency = maven_dependency(Some("latest"), "library");
        let artifact = maven_artifact(
            "1.0.0",
            "https://repo.example/com/example/library/1.0.0/library-1.0.0.jar",
            ".wisteria/cache/com.example/library/1.0.0/library.jar",
        );

        assert!(dependency.matches_lockfile_artifact(&artifact));
    }

    #[test]
    fn maven_dependency_rejects_lockfile_artifact_for_different_coordinates() {
        let dependency = maven_dependency(Some("latest"), "library");
        let artifact = maven_artifact(
            "1.0.0",
            "https://repo.example/com/example/other/1.0.0/other-1.0.0.jar",
            ".wisteria/cache/com.example/other/1.0.0/other.jar",
        );

        assert!(!dependency.matches_lockfile_artifact(&artifact));
    }

    #[test]
    fn maven_dependency_matches_snapshot_lockfile_artifact_with_timestamp_value() {
        let dependency = maven_dependency(Some("1.0-SNAPSHOT"), "library");
        let artifact = maven_artifact(
            "1.0-SNAPSHOT",
            "https://repo.example/com/example/library/1.0-SNAPSHOT/library-1.0-20260813.123456-1.jar",
            ".wisteria/cache/com.example/library/1.0-SNAPSHOT/library-1.0-20260813.123456-1.jar",
        );

        assert!(dependency.matches_lockfile_artifact(&artifact));
    }

    #[test]
    fn github_dependency_matches_lockfile_artifact_for_same_repository() {
        let dependency = github_dependency(None, "Repository");
        let artifact = github_artifact(
            "v1.0.0",
            "https://github.com/Owner/Repository/releases/download/v1.0.0/library.jar",
            ".wisteria/cache/Owner/Repository/v1.0.0/Repository.jar",
        );

        assert!(dependency.matches_lockfile_artifact(&artifact));
    }

    #[test]
    fn github_dependency_rejects_lockfile_artifact_for_different_repository() {
        let dependency = github_dependency(None, "Repository");
        let artifact = github_artifact(
            "v1.0.0",
            "https://github.com/Owner/Other/releases/download/v1.0.0/library.jar",
            ".wisteria/cache/Owner/Other/v1.0.0/Other.jar",
        );

        assert!(!dependency.matches_lockfile_artifact(&artifact));
    }
}
