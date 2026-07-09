use toml::Table;

use crate::config::toml_utils;
use crate::dependency::{Dependency, GithubReleaseType, UpdatePolicy};

impl Dependency {
    pub fn load(toml: &Table) -> Result<Dependency, (String, u8)> {
        match toml.get("type") {
            Some(val) if val.is_str() => {
                let dependency_type = val.as_str().unwrap();

                if dependency_type == "loadFolder" {
                    let path: String = toml_utils::read_string("path", toml)?;
                    let recursive: bool =
                        toml_utils::read_boolean("recursive", toml).unwrap_or(true);

                    return Ok(Dependency::LocalFolder { path, recursive });
                }

                let update_policy = UpdatePolicy::load(
                    &toml_utils::read_string("update_policy", toml)
                        .unwrap_or(String::from("SwitchOrUpdate")),
                )?;
                let javadoc: Option<String> = toml_utils::read_string("javadoc", toml).ok();

                match dependency_type {
                    "loadArchive" => {
                        let path: String = toml_utils::read_string("path", toml)?;

                        Ok(Dependency::LocalFile { path, javadoc })
                    }
                    "localRepository" => {
                        let repository: String = toml_utils::read_string("repository", toml)?;
                        let name: String = toml_utils::read_string("name", toml)?;
                        let version: String = toml_utils::read_string("version", toml)?;

                        Ok(Dependency::LocalRepository {
                            repository,
                            name,
                            version,
                            update_policy,
                            javadoc,
                        })
                    }
                    "fetchFromUrl" => {
                        let url: String = toml_utils::read_string("url", toml)?;

                        Ok(Dependency::FetchFromUrl {
                            url,
                            update_policy,
                            javadoc,
                        })
                    }
                    "fetchFromMaven" => {
                        let url: String = toml_utils::read_string("url", toml)
                            .unwrap_or(String::from("https://repo1.maven.org/maven2/"));
                        let group_id: String = toml_utils::read_string("group_id", toml)?;
                        let artifact_id: String = toml_utils::read_string("artifact_id", toml)?;
                        let version = toml_utils::read_string("version", toml).ok();
                        let classifier: Option<String> =
                            toml_utils::read_string("classifier", toml).ok();

                        Ok(Dependency::FetchFromMaven {
                            url,
                            group_id,
                            artifact_id,
                            version,
                            classifier,
                            update_policy,
                            javadoc,
                        })
                    }
                    "fetchFromGithub" => {
                        let username: String = toml_utils::read_string("username", toml)?;
                        let repository: String = toml_utils::read_string("repository", toml)?;
                        let tag: Option<String> = toml_utils::read_string("tag", toml).ok();
                        let release_type = GithubReleaseType::load(
                            &toml_utils::read_string("release_type", toml)
                                .unwrap_or(String::from("release")),
                        )?;

                        let asset: String = toml_utils::read_string("asset", toml)
                            .unwrap_or(repository.to_string());

                        Ok(Dependency::FetchFromGithub {
                            username,
                            repository,
                            asset,
                            tag,
                            release_type,
                            update_policy,
                            javadoc,
                        })
                    }
                    "buildFromScript" => {
                        let run: Vec<String> = toml_utils::read_string_array("run", toml)?;
                        let target: String = toml_utils::read_string("target", toml)?;

                        Ok(Dependency::BuildFromScript {
                            run,
                            target,
                            update_policy,
                            javadoc,
                        })
                    }
                    _ => Err((format!("Unknown dependency type \"{dependency_type}\""), 31)),
                }
            }
            Some(val) => Err((
                format!(
                    "Unexpected input for dependency type, expected a string, found {}",
                    val.type_str()
                ),
                32,
            )),
            None => Err((
                String::from("Dependency must explicitly define its type"),
                32,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_dependency(toml: &str) -> Dependency {
        Dependency::load(&toml.parse::<Table>().unwrap()).unwrap()
    }

    #[test]
    fn github_dependency_allows_explicit_tag() {
        let dependency = load_dependency(
            r#"
            type = "fetchFromGithub"
            username = "Example"
            repository = "Library"
            tag = "v1.2.3"
            "#,
        );

        match dependency {
            Dependency::FetchFromGithub {
                tag, release_type, ..
            } => {
                assert_eq!(tag.as_deref(), Some("v1.2.3"));
                assert_eq!(release_type, GithubReleaseType::Release);
            }
            _ => panic!("expected GitHub dependency"),
        }
    }

    #[test]
    fn github_dependency_defaults_to_latest_release() {
        let dependency = load_dependency(
            r#"
            type = "fetchFromGithub"
            username = "Example"
            repository = "Library"
            "#,
        );

        match dependency {
            Dependency::FetchFromGithub {
                tag, release_type, ..
            } => {
                assert!(tag.is_none());
                assert_eq!(release_type, GithubReleaseType::Release);
            }
            _ => panic!("expected GitHub dependency"),
        }
    }

    #[test]
    fn github_dependency_accepts_prerelease_selector() {
        let dependency = load_dependency(
            r#"
            type = "fetchFromGithub"
            username = "Example"
            repository = "Library"
            release_type = "prerelease"
            "#,
        );

        match dependency {
            Dependency::FetchFromGithub { release_type, .. } => {
                assert_eq!(release_type, GithubReleaseType::Prerelease);
            }
            _ => panic!("expected GitHub dependency"),
        }
    }

    #[test]
    fn github_dependency_rejects_invalid_release_type() {
        let error = match Dependency::load(
            &r#"
            type = "fetchFromGithub"
            username = "Example"
            repository = "Library"
            release_type = "nightly"
            "#
            .parse::<Table>()
            .unwrap(),
        ) {
            Ok(_) => panic!("expected invalid release_type to fail"),
            Err(error) => error,
        };

        assert!(error.0.contains("Unexpected GitHub release type"));
    }
}
