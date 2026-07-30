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

#[cfg(test)]
mod tests {
    use super::*;
    use toml::Table;

    fn configuration(toml: &str) -> Configuration {
        Configuration::from(
            String::from("main"),
            &toml.parse::<Table>().unwrap(),
            String::from("Demo"),
            String::from("1.0.0"),
        )
        .unwrap()
    }

    #[test]
    fn local_folder_dependencies_are_not_shadable() {
        let dependency = Dependency::LocalFolder {
            path: String::from("lib/"),
            recursive: true,
        };
        let configuration = configuration(r#"shaded = [ "lib" ]"#);

        assert_eq!(dependency.is_shaded("lib", &configuration), None);
    }

    #[test]
    fn non_folder_dependencies_report_whether_they_are_shaded() {
        let dependency = Dependency::LocalFile {
            path: String::from("lib/library.jar"),
            javadoc: None,
        };
        let configuration = configuration(r#"shaded = [ "library" ]"#);

        assert_eq!(dependency.is_shaded("library", &configuration), Some(true));
        assert_eq!(dependency.is_shaded("other", &configuration), Some(false));
    }

    #[test]
    fn dependencies_without_shaded_configuration_return_none() {
        let dependency = Dependency::LocalFile {
            path: String::from("lib/library.jar"),
            javadoc: None,
        };
        let configuration = configuration("");

        assert_eq!(dependency.is_shaded("library", &configuration), None);
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
