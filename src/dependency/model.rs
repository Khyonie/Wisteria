use crate::dependency::UpdatePolicy;

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
