use std::collections::HashMap;

use toml::{Table, Value};

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
                        let repository: String = toml_utils::read_string("repository", toml)?;
                        let username = match toml_utils::read_string("username", toml) {
                            Ok(username) => Some(username),
                            Err(_) if !toml.contains_key("username") => None,
                            Err(e) => return Err(e),
                        };
                        let (username, repository) =
                            github_owner_and_repository(username, repository)?;
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

    pub fn load_from_group(group: &str, toml: &Table) -> Result<Dependency, (String, u8)> {
        let dependency_type = dependency_type_for_group(group)?;
        let mut toml = toml.clone();

        if let Some(existing_type) = toml.get("type") {
            match existing_type.as_str() {
                Some(existing_type) if existing_type == dependency_type => {}
                Some(existing_type) => {
                    return Err((
                        format!(
                            "Dependency in group \"{group}\" has mismatched type \"{existing_type}\", expected \"{dependency_type}\""
                        ),
                        32,
                    ))
                }
                None => {
                    return Err((
                        format!(
                            "Mismatched type for dependency type in group \"{group}\", expected a string, found {}",
                            existing_type.type_str()
                        ),
                        32,
                    ))
                }
            }
        } else {
            toml.insert(
                String::from("type"),
                Value::String(dependency_type.to_string()),
            );
        }

        Dependency::load(&toml)
    }
}

pub fn load_dependency_map(
    dependencies_map: Option<&Value>,
) -> Result<HashMap<String, Dependency>, (String, u8)> {
    let Some(dependencies_map) = dependencies_map else {
        return Ok(HashMap::new());
    };

    let Some(dependencies_table) = dependencies_map.as_table() else {
        return Err((
            format!(
                "Mismatched type for \"dependencies\", expected a table, found {}",
                dependencies_map.type_str()
            ),
            16,
        ));
    };

    let mut dependencies: HashMap<String, Dependency> = HashMap::new();

    for (key, value) in dependencies_table {
        let Some(table) = value.as_table() else {
            return Err((
                format!(
                    "Mismatched type for dependency group or dependency \"{key}\", expected a table, found {}",
                    value.type_str()
                ),
                16,
            ));
        };

        if table.contains_key("type") {
            insert_dependency(&mut dependencies, key, Dependency::load(table)?)?;
            continue;
        }

        if dependency_type_for_group(key).is_err() {
            return Err((
                format!(
                    "Unknown dependency group \"{key}\". Expected one of [archive, folder, url, maven, github, local_repository, script]"
                ),
                31,
            ));
        }

        for (dependency_name, dependency_value) in table {
            let Some(dependency_table) = dependency_value.as_table() else {
                return Err((
                    format!(
                        "Mismatched type for dependency \"{dependency_name}\" in group \"{key}\", expected a table, found {}",
                        dependency_value.type_str()
                    ),
                    16,
                ));
            };

            insert_dependency(
                &mut dependencies,
                dependency_name,
                Dependency::load_from_group(key, dependency_table)?,
            )?;
        }
    }

    Ok(dependencies)
}

pub fn migrate_legacy_dependency_table(project_toml: &mut Table) -> Result<bool, (String, u8)> {
    let Some(dependencies_value) = project_toml.get_mut("dependencies") else {
        return Ok(false);
    };

    let Some(dependencies_table) = dependencies_value.as_table_mut() else {
        return Err((
            format!(
                "Mismatched type for \"dependencies\", expected a table, found {}",
                dependencies_value.type_str()
            ),
            16,
        ));
    };

    let old_dependencies = std::mem::take(dependencies_table);
    let mut new_dependencies = Table::new();
    let mut migrated = false;

    for (key, value) in old_dependencies {
        let Some(table) = value.as_table() else {
            new_dependencies.insert(key, value);
            continue;
        };

        let Some(dependency_type_value) = table.get("type") else {
            if dependency_type_for_group(&key).is_ok() {
                merge_dependency_group(&mut new_dependencies, key, table.clone())?;
            } else {
                new_dependencies.insert(key, value);
            }
            continue;
        };

        let Some(dependency_type) = dependency_type_value.as_str() else {
            return Err((
                format!(
                    "Mismatched type for dependency type in \"{key}\", expected a string, found {}",
                    dependency_type_value.type_str()
                ),
                32,
            ));
        };

        let group = group_for_dependency_type(dependency_type)?;
        let mut migrated_dependency = table.clone();
        migrated_dependency.remove("type");
        insert_grouped_dependency(
            &mut new_dependencies,
            group,
            key,
            Value::Table(migrated_dependency),
        )?;
        migrated = true;
    }

    *dependencies_table = new_dependencies;
    Ok(migrated)
}

fn insert_dependency(
    dependencies: &mut HashMap<String, Dependency>,
    name: &str,
    dependency: Dependency,
) -> Result<(), (String, u8)> {
    if dependencies.insert(name.to_string(), dependency).is_some() {
        return Err((format!("Duplicate dependency name \"{name}\""), 33));
    }

    Ok(())
}

fn merge_dependency_group(
    dependencies_table: &mut Table,
    group: String,
    dependency_group: Table,
) -> Result<(), (String, u8)> {
    for (dependency_name, dependency_value) in dependency_group {
        insert_grouped_dependency(
            dependencies_table,
            &group,
            dependency_name,
            dependency_value,
        )?;
    }

    Ok(())
}

fn insert_grouped_dependency(
    dependencies_table: &mut Table,
    group: &str,
    dependency_name: String,
    dependency_value: Value,
) -> Result<(), (String, u8)> {
    if !matches!(dependency_value, Value::Table(_)) {
        return Err((
            format!(
                "Mismatched type for dependency \"{dependency_name}\" in group \"{group}\", expected a table, found {}",
                dependency_value.type_str()
            ),
            16,
        ));
    }

    if !dependencies_table.contains_key(group) {
        dependencies_table.insert(group.to_string(), Value::Table(Table::new()));
    }

    let group_value = dependencies_table.get_mut(group).unwrap();
    let Some(group_table) = group_value.as_table_mut() else {
        return Err((
            format!(
                "Mismatched type for dependency group \"{group}\", expected a table, found {}",
                group_value.type_str()
            ),
            16,
        ));
    };

    if group_table
        .insert(dependency_name.clone(), dependency_value)
        .is_some()
    {
        return Err((
            format!("Duplicate dependency name \"{dependency_name}\""),
            33,
        ));
    }

    Ok(())
}

fn dependency_type_for_group(group: &str) -> Result<&'static str, (String, u8)> {
    match group {
        "archive" | "loadArchive" => Ok("loadArchive"),
        "folder" | "loadFolder" => Ok("loadFolder"),
        "url" | "fetchFromUrl" => Ok("fetchFromUrl"),
        "maven" | "fetchFromMaven" => Ok("fetchFromMaven"),
        "github" | "fetchFromGithub" => Ok("fetchFromGithub"),
        "local_repository" | "localRepository" => Ok("localRepository"),
        "script" | "buildFromScript" => Ok("buildFromScript"),
        _ => Err((format!("Unknown dependency group \"{group}\""), 31)),
    }
}

fn group_for_dependency_type(dependency_type: &str) -> Result<&'static str, (String, u8)> {
    match dependency_type {
        "loadArchive" => Ok("archive"),
        "loadFolder" => Ok("folder"),
        "fetchFromUrl" => Ok("url"),
        "fetchFromMaven" => Ok("maven"),
        "fetchFromGithub" => Ok("github"),
        "localRepository" => Ok("local_repository"),
        "buildFromScript" => Ok("script"),
        _ => Err((format!("Unknown dependency type \"{dependency_type}\""), 31)),
    }
}

fn github_owner_and_repository(
    username: Option<String>,
    repository: String,
) -> Result<(String, String), (String, u8)> {
    if let Some(username) = username {
        return Ok((username, repository));
    }

    let Some((username, repository)) = repository.split_once('/') else {
        return Err((
            String::from("Missing key username and repository is not in owner/repository form"),
            10,
        ));
    };

    if username.is_empty() || repository.is_empty() || repository.contains('/') {
        return Err((
            String::from("GitHub repository shorthand must be in owner/repository form"),
            10,
        ));
    }

    Ok((username.to_string(), repository.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_dependency(toml: &str) -> Dependency {
        Dependency::load(&toml.parse::<Table>().unwrap()).unwrap()
    }

    #[test]
    fn grouped_dependency_map_loads_maven_and_github_dependencies() {
        let toml = r#"
            [maven]
            gson = { group_id = "com.google.code.gson", artifact_id = "gson" }

            [github]
            lilac = { repository = "Khyonie/Lilac" }
            "#;
        let table = toml.parse::<Table>().unwrap();
        let dependencies = load_dependency_map(Some(&Value::Table(table))).unwrap();

        match dependencies.get("gson").unwrap() {
            Dependency::FetchFromMaven {
                group_id,
                artifact_id,
                ..
            } => {
                assert_eq!(group_id, "com.google.code.gson");
                assert_eq!(artifact_id, "gson");
            }
            _ => panic!("expected Maven dependency"),
        }

        match dependencies.get("lilac").unwrap() {
            Dependency::FetchFromGithub {
                username,
                repository,
                ..
            } => {
                assert_eq!(username, "Khyonie");
                assert_eq!(repository, "Lilac");
            }
            _ => panic!("expected GitHub dependency"),
        }
    }

    #[test]
    fn migrates_legacy_flat_dependencies_to_grouped_tables() {
        let mut project = r#"
            [dependencies]
            gson = { type = "fetchFromMaven", group_id = "com.google.code.gson", artifact_id = "gson" }
            lilac = { type = "fetchFromGithub", repository = "Khyonie/Lilac" }
            local = { type = "loadArchive", path = "lib/local.jar" }
            "#
        .parse::<Table>()
        .unwrap();

        assert!(migrate_legacy_dependency_table(&mut project).unwrap());

        let dependencies = project.get("dependencies").unwrap().as_table().unwrap();
        assert!(dependencies
            .get("maven")
            .unwrap()
            .as_table()
            .unwrap()
            .contains_key("gson"));
        assert!(dependencies
            .get("github")
            .unwrap()
            .as_table()
            .unwrap()
            .contains_key("lilac"));
        assert!(dependencies
            .get("archive")
            .unwrap()
            .as_table()
            .unwrap()
            .contains_key("local"));
        assert!(dependencies.get("type").is_none());

        let loaded = load_dependency_map(project.get("dependencies")).unwrap();
        assert!(matches!(
            loaded.get("gson").unwrap(),
            Dependency::FetchFromMaven { .. }
        ));
    }

    #[test]
    fn rejects_duplicate_dependency_names_across_groups() {
        let toml = r#"
            [maven]
            library = { group_id = "com.example", artifact_id = "library" }

            [github]
            library = { repository = "Example/Library" }
            "#;
        let table = toml.parse::<Table>().unwrap();

        let error = match load_dependency_map(Some(&Value::Table(table))) {
            Ok(_) => panic!("expected duplicate dependency name to fail"),
            Err(error) => error,
        };

        assert!(error.0.contains("Duplicate dependency name"));
        assert_eq!(error.1, 33);
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
    fn github_dependency_accepts_repository_shorthand() {
        let dependency = load_dependency(
            r#"
            type = "fetchFromGithub"
            repository = "Example/Library"
            "#,
        );

        match dependency {
            Dependency::FetchFromGithub {
                username,
                repository,
                asset,
                ..
            } => {
                assert_eq!(username, "Example");
                assert_eq!(repository, "Library");
                assert_eq!(asset, "Library");
            }
            _ => panic!("expected GitHub dependency"),
        }
    }

    #[test]
    fn github_dependency_requires_username_or_repository_shorthand() {
        let error = match Dependency::load(
            &r#"
            type = "fetchFromGithub"
            repository = "Library"
            "#
            .parse::<Table>()
            .unwrap(),
        ) {
            Ok(_) => panic!("expected missing username to fail"),
            Err(error) => error,
        };

        assert!(error.0.contains("Missing key username"));
    }

    #[test]
    fn github_dependency_rejects_invalid_repository_shorthand() {
        let error = match Dependency::load(
            &r#"
            type = "fetchFromGithub"
            repository = "Example/Org/Library"
            "#
            .parse::<Table>()
            .unwrap(),
        ) {
            Ok(_) => panic!("expected invalid repository shorthand to fail"),
            Err(error) => error,
        };

        assert!(error.0.contains("owner/repository"));
    }

    #[test]
    fn github_dependency_rejects_malformed_username_with_repository_shorthand() {
        let error = match Dependency::load(
            &r#"
            type = "fetchFromGithub"
            username = 10
            repository = "Example/Library"
            "#
            .parse::<Table>()
            .unwrap(),
        ) {
            Ok(_) => panic!("expected malformed username to fail"),
            Err(error) => error,
        };

        assert!(error.0.contains("Mismatched type for \"username\""));
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

    #[test]
    fn load_folder_dependency_defaults_to_recursive() {
        let dependency = load_dependency(
            r#"
            type = "loadFolder"
            path = "lib/"
            "#,
        );

        match dependency {
            Dependency::LocalFolder { path, recursive } => {
                assert_eq!(path, "lib/");
                assert!(recursive);
            }
            _ => panic!("expected local folder dependency"),
        }
    }

    #[test]
    fn fetch_from_url_loads_update_policy_and_javadoc() {
        let dependency = load_dependency(
            r#"
            type = "fetchFromUrl"
            url = "https://example.com/library.jar"
            update_policy = "Never"
            javadoc = "https://example.com/docs"
            "#,
        );

        match dependency {
            Dependency::FetchFromUrl {
                url,
                update_policy,
                javadoc,
            } => {
                assert_eq!(url, "https://example.com/library.jar");
                assert!(matches!(update_policy, UpdatePolicy::Never));
                assert_eq!(javadoc.as_deref(), Some("https://example.com/docs"));
            }
            _ => panic!("expected URL dependency"),
        }
    }

    #[test]
    fn fetch_from_maven_uses_default_repository_url() {
        let dependency = load_dependency(
            r#"
            type = "fetchFromMaven"
            group_id = "com.example"
            artifact_id = "library"
            "#,
        );

        match dependency {
            Dependency::FetchFromMaven {
                url,
                group_id,
                artifact_id,
                version,
                classifier,
                ..
            } => {
                assert_eq!(url, "https://repo1.maven.org/maven2/");
                assert_eq!(group_id, "com.example");
                assert_eq!(artifact_id, "library");
                assert!(version.is_none());
                assert!(classifier.is_none());
            }
            _ => panic!("expected Maven dependency"),
        }
    }

    #[test]
    fn rejects_unknown_dependency_type() {
        let error = match Dependency::load(
            &r#"
            type = "unknown"
            "#
            .parse::<Table>()
            .unwrap(),
        ) {
            Ok(_) => panic!("expected unknown dependency type to fail"),
            Err(error) => error,
        };

        assert!(error.0.contains("Unknown dependency type"));
        assert_eq!(error.1, 31);
    }
}
