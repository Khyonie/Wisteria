use crate::dependency::UpdatePolicy;
use crate::model::Configuration;

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

    pub fn is_shaded(&self, name: &str, configuration: &Configuration) -> Option<bool> {
        let shaded = configuration.shaded()?;

        match self {
            Dependency::LocalFolder { .. } => None,
            _ => Some(shaded.contains(&String::from(name))),
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
    pub fn load(value: &str) -> Result<Self, (String, u8)> {
        match value {
            "release" => Ok(Self::Release),
            "prerelease" | "pre-release" => Ok(Self::Prerelease),
            "any" => Ok(Self::Any),
            _ => Err((
                format!(
                    "Unexpected GitHub release type, expected one of [release, prerelease, pre-release, any], found {value}"
                ),
                30,
            )),
        }
    }
}
