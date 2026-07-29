use std::{collections::HashMap, fs::read_to_string};

use toml::Table;

use crate::{
    cli::args::StartupFlags,
    config::toml_utils,
    dependency::{load_dependency_map, migrate_legacy_dependency_table, Dependency},
    model::Configuration,
    util::consts,
    workspace::nature::Nature,
};

/// Collection of identifying information for a project.
#[derive(Clone)]
pub struct ProjectInfo {
    name: String,
    description: String,
    authors: Vec<String>,
    version: String,
    license: Vec<String>,
    homepage: Option<String>,
    sourcepage: Option<String>,
    natures: Vec<Nature>,
    configurations: HashMap<String, Configuration>,
}

#[derive(Clone)]
pub struct Project {
    info: ProjectInfo,
    dependencies: HashMap<String, Dependency>,
}

impl Project {
    pub fn from(project_file: Option<String>) -> Result<Self, (String, u8)> {
        Self::from_with_flags(project_file, StartupFlags::default())
    }

    pub fn from_with_flags(
        project_file: Option<String>,
        flags: StartupFlags,
    ) -> Result<Self, (String, u8)> {
        let project_file = project_file.unwrap_or(String::from(consts::PROJECT_FILE));
        let project_toml_string = read_to_string(&project_file).map_err(|e| {
            (
                format!(
                    "Could not read project file \"{project_file}\": {e}.\nFix: run this command from a Wisteria project folder, create a project with `wisteria create <name>`, or pass a file with `--project <project.toml>`."
                ),
                1,
            )
        })?;

        let mut project_toml: Table = project_toml_string
            .parse::<Table>()
            .map_err(|e| {
                (
                    format!(
                        "Could not parse \"{project_file}\" as TOML: {e}\nFix: check for missing quotes, unfinished arrays/tables, duplicate table headers, or malformed inline tables."
                    ),
                    1,
                )
            })?;
        migrate_legacy_dependency_table(&mut project_toml)?;

        let toml = read_project_table(&project_toml)?;
        let configuration_map = project_toml.get("configuration");
        let dependencies_map = project_toml.get("dependencies");

        let name: String = read_project_string("name", toml)?;
        let version: String = read_project_string("version", toml)?;
        let info = ProjectInfo {
            name: name.clone(),
            description: read_project_string("description", toml)?,
            authors: read_project_string_array("authors", toml)?.unwrap_or_default(),
            version: version.clone(),
            license: read_project_string_array("license", toml)?.unwrap_or_default(),
            homepage: read_project_optional_string("homepage", toml)?,
            sourcepage: read_project_optional_string("sourcepage", toml)?,
            natures: read_project_natures(toml)?,
            configurations: match configuration_map {
                Some(v) if v.is_table() => {
                    let v = v.as_table().unwrap();
                    let mut configurations: HashMap<String, Configuration> = HashMap::new();

                    for key in v.keys() {
                        match v.get(key)
                        {
                            Some(config) if config.is_table() => {
                                let mut configuration = Configuration::from(key.clone(), config.as_table().unwrap(), name.clone(), version.clone())
                                    .map_err(|error| contextual_configuration_load_error(key, error))?;
                                configuration.apply_implicit(flags.clone());
                                configurations.insert(key.clone(), configuration)
                            }
                            Some(v) => return Err((format!("Invalid [configuration.{key}]: expected a table, found {}.\nFix: define configurations as tables, for example `[configuration.{key}]` followed by keys like `sources` and `targets`.", v.type_str()), 16)),
                            None => None
                        };
                    }

                    let mut updated_configurations: HashMap<String, Configuration> = HashMap::new();
                    for (config_name, configuration) in configurations.iter() {
                        if let Some(target) = configuration.inherits() {
                            if config_name.eq(target) {
                                return Err((format!("Configuration \"{config_name}\" cannot inherit from itself.\nFix: remove `inherit = \"{target}\"` from [configuration.{config_name}], or point it at a different configuration."), 40));
                            }

                            let target = match configurations.get(target)
                            {
                                Some(c) => c,
                                None => return Err((format!("No such configuration \"{target}\" to be inherited by \"{config_name}\".\nFix: create `[configuration.{target}]`, or change `inherit` in [configuration.{config_name}] to one of: {}.", configuration_names(&configurations)), 41))
                            };

                            let mut inheritor: Configuration = configuration.clone();
                            inheritor.inherit_from(target);
                            inheritor.apply_implicit(flags.clone());
                            updated_configurations.insert(config_name.clone(), inheritor);
                        }
                    }

                    for (k, v) in updated_configurations {
                        configurations.insert(k, v);
                    }

                    configurations
                }
                Some(v) => {
                    return Err((
                        format!(
                            "Invalid [configuration] section: expected a table of configuration tables, found {}.\nFix: define configurations like `[configuration.main]`, not `configuration = ...`.",
                            v.type_str()
                        ),
                        16,
                    ))
                }
                None => HashMap::new(),
            },
        };

        let dependencies: HashMap<String, Dependency> = load_dependency_map(dependencies_map)?;

        Ok(Project { info, dependencies })
    }

    pub fn info(&self) -> &ProjectInfo {
        &self.info
    }

    pub fn dependencies(&self) -> &HashMap<String, Dependency> {
        &self.dependencies
    }

    pub fn print_info(&self) {
        println!(
            "╒══[ Information for project \"{}\" ]═════════════",
            self.info.name
        );
        println!("│\tDescription      {}", self.info.description);

        match self.info.authors.len() {
            0 => {}
            _ => println!(
                "│\tAuthors          {}",
                toml_utils::string_vec_to_string(&self.info.authors)
            ),
        }

        println!("│\tVersion          {}", self.info.version);

        match self.info.license.len() {
            0 => {}
            _ => println!(
                "│\tLicenses         {}",
                toml_utils::string_vec_to_string(&self.info.license)
            ),
        }

        if let Some(s) = &self.info.homepage {
            println!("│\tWebsite          {s}")
        }

        if let Some(s) = &self.info.sourcepage {
            println!("│\tSource           {s}")
        }

        println!("│\tConfigurations   {}", self.info.configurations.len());
        println!("│\tDependencies     {}", self.dependencies.len());

        if !self.dependencies.is_empty() {
            println!("╞ Dependencies:");
            for (name, dependency) in &self.dependencies {
                println!("│\t{:<16} ({})", name, dependency.type_str())
            }
        }
        println!("│");

        for c in self.info.configurations.values() {
            c.print_info()
        }
        println!("│\t*Depending on the configuration, Wisteria may automatically provide tasks such as \"build\".")
    }
}

fn read_project_table(project_toml: &Table) -> Result<&Table, (String, u8)> {
    match project_toml.get("project") {
        Some(value) if value.is_table() => Ok(value.as_table().unwrap()),
        Some(value) => Err((
            format!(
                "Invalid [project] section: expected a table, found {}.\nFix: define project metadata with a `[project]` header, then keys like `name`, `version`, and `description` underneath it.",
                value.type_str()
            ),
            16,
        )),
        None => Err((
            String::from(
                "Missing [project] section in project.toml.\nFix: add a `[project]` table with at least `name`, `version`, and `description`.",
            ),
            10,
        )),
    }
}

fn read_project_string(key: &str, toml: &Table) -> Result<String, (String, u8)> {
    toml_utils::read_string(key, toml).map_err(|error| contextual_project_error(key, error))
}

fn read_project_optional_string(key: &str, toml: &Table) -> Result<Option<String>, (String, u8)> {
    toml_utils::read_optional_string(key, toml)
        .map_err(|error| contextual_project_error(key, error))
}

fn read_project_string_array(
    key: &str,
    toml: &Table,
) -> Result<Option<Vec<String>>, (String, u8)> {
    toml_utils::read_optional_string_array(key, toml)
        .map_err(|error| contextual_project_error(key, error))
}

fn read_project_natures(toml: &Table) -> Result<Vec<Nature>, (String, u8)> {
    let Some(natures) = read_project_string_array("natures", toml)? else {
        return Ok(Vec::new());
    };

    let mut parsed = Vec::new();
    for nature in natures {
        match nature.as_str() {
            "eclipse" => parsed.push(Nature::Eclipse),
            "maven" => parsed.push(Nature::Maven),
            _ => {
                return Err((
                    format!(
                        "Invalid [project].natures entry \"{nature}\".\nFix: supported natures are `eclipse` and `maven`; remove the value or use `natures = [ \"eclipse\", \"maven\" ]`."
                    ),
                    31,
                ))
            }
        }
    }

    Ok(parsed)
}

fn contextual_project_error(key: &str, error: (String, u8)) -> (String, u8) {
    (
        format!("Invalid [project].{key}: {}", error.0),
        error.1,
    )
}

fn contextual_configuration_load_error(
    configuration_name: &str,
    error: (String, u8),
) -> (String, u8) {
    (
        format!(
            "Could not load [configuration.{configuration_name}] from project.toml.\n{}",
            error.0
        ),
        error.1,
    )
}

fn configuration_names(configurations: &HashMap<String, Configuration>) -> String {
    if configurations.is_empty() {
        return String::from("none are currently defined");
    }

    let mut names: Vec<&str> = configurations.keys().map(String::as_str).collect();
    names.sort_unstable();
    names.join(", ")
}

#[allow(dead_code)]
impl ProjectInfo {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn authors(&self) -> &[String] {
        &self.authors
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn license(&self) -> &[String] {
        &self.license
    }

    pub fn homepage(&self) -> Option<&String> {
        self.homepage.as_ref()
    }

    pub fn sourcepage(&self) -> Option<&String> {
        self.sourcepage.as_ref()
    }

    pub fn natures(&self) -> &Vec<Nature> {
        self.natures.as_ref()
    }

    pub fn configurations(&self) -> &HashMap<String, Configuration> {
        &self.configurations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependency::Dependency;
    use crate::test_support::TempDir;
    use std::fs;

    fn write_project(temp: &TempDir, contents: &str) -> String {
        let path = temp.path().join("project.toml");
        fs::write(&path, contents).unwrap();
        path.to_string_lossy().to_string()
    }

    #[test]
    fn loads_project_metadata_dependencies_natures_and_configurations() {
        let temp = TempDir::new("project-load");
        let project_file = write_project(
            &temp,
            r#"
            [project]
            name = "Demo"
            version = "1.2.3"
            description = "Demo project"
            authors = [ "Alice", "Bob" ]
            license = "MIT"
            homepage = "https://example.com"
            sourcepage = "https://github.com/Example/Demo"
            natures = [ "eclipse", "maven" ]

            # This legacy dependency shape should still load through migration.
            [dependencies]
            github = { type = "fetchFromGithub", repository = "Example/Library" }
            local = { type = "loadArchive", path = "lib/local.jar" }

            [configuration.main]
            sources = [ "src/" ]
            dependencies = [ "github", "local" ]
            targets = [ "target/demo.jar" ]
            "#,
        );

        let project = Project::from(Some(project_file)).unwrap();

        assert_eq!(project.info().name(), "Demo");
        assert_eq!(project.info().description(), "Demo project");
        assert_eq!(project.info().authors(), &["Alice", "Bob"]);
        assert_eq!(project.info().license(), &["MIT"]);
        assert_eq!(
            project.info().homepage().map(String::as_str),
            Some("https://example.com")
        );
        assert_eq!(
            project.info().sourcepage().map(String::as_str),
            Some("https://github.com/Example/Demo")
        );
        assert_eq!(project.info().natures().len(), 2);
        assert!(matches!(project.info().natures()[0], Nature::Eclipse));
        assert!(matches!(project.info().natures()[1], Nature::Maven));

        match project.dependencies().get("github").unwrap() {
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

        let configuration = project.info().configurations().get("main").unwrap();
        assert!(configuration.tasks().contains_key("build"));
    }

    #[test]
    fn loads_grouped_dependency_tables() {
        let temp = TempDir::new("project-load-grouped-dependencies");
        let project_file = write_project(
            &temp,
            r#"
            [project]
            name = "Demo"
            version = "1.2.3"
            description = "Demo project"

            [dependencies.github]
            github = { repository = "Example/Library" }

            [dependencies.maven]
            maven = { group_id = "com.example", artifact_id = "library" }

            [configuration.main]
            sources = [ "src/" ]
            dependencies = [ "github", "maven" ]
            targets = [ "target/demo.jar" ]
            "#,
        );

        let project = Project::from(Some(project_file)).unwrap();

        assert!(matches!(
            project.dependencies().get("github").unwrap(),
            Dependency::FetchFromGithub { .. }
        ));
        assert!(matches!(
            project.dependencies().get("maven").unwrap(),
            Dependency::FetchFromMaven { .. }
        ));
    }

    #[test]
    fn applies_single_level_configuration_inheritance() {
        let temp = TempDir::new("project-inherit");
        let project_file = write_project(
            &temp,
            r#"
            [project]
            name = "Demo"
            version = "1.0.0"
            description = "Demo project"

            [configuration.base]
            sources = [ "src/main/" ]
            dependencies = [ "base-dep" ]
            targets = [ "target/base.jar" ]

            [configuration.child]
            inherit = "base"
            sources = [ "src/child/" ]
            dependencies = [ "child-dep" ]
            "#,
        );

        let project = Project::from(Some(project_file)).unwrap();
        let child = project.info().configurations().get("child").unwrap();

        assert_eq!(
            child.sources().unwrap(),
            &vec![String::from("src/child/"), String::from("src/main/")]
        );
        assert_eq!(
            child.dependencies().unwrap(),
            &vec![String::from("child-dep"), String::from("base-dep")]
        );
        assert_eq!(
            child.targets().unwrap(),
            &vec![String::from("target/base.jar")]
        );
        assert!(child.tasks().contains_key("build"));
    }

    #[test]
    fn rejects_self_inheriting_configuration() {
        let temp = TempDir::new("project-self-inherit");
        let project_file = write_project(
            &temp,
            r#"
            [project]
            name = "Demo"
            version = "1.0.0"
            description = "Demo project"

            [configuration.main]
            inherit = "main"
            "#,
        );

        let error = match Project::from(Some(project_file)) {
            Ok(_) => panic!("expected self-inheritance to fail"),
            Err(error) => error,
        };

        assert!(error.0.contains("cannot inherit from itself"));
        assert_eq!(error.1, 40);
    }

    #[test]
    fn rejects_missing_inherited_configuration() {
        let temp = TempDir::new("project-missing-inherit");
        let project_file = write_project(
            &temp,
            r#"
            [project]
            name = "Demo"
            version = "1.0.0"
            description = "Demo project"

            [configuration.main]
            inherit = "missing"
            "#,
        );

        let error = match Project::from(Some(project_file)) {
            Ok(_) => panic!("expected missing inherited configuration to fail"),
            Err(error) => error,
        };

        assert!(error.0.contains("No such configuration"));
        assert_eq!(error.1, 41);
    }

    #[test]
    fn rejects_project_file_without_project_table() {
        let temp = TempDir::new("project-missing-project-table");
        let project_file = write_project(
            &temp,
            r#"
            [configuration.main]
            sources = [ "src/" ]
            "#,
        );

        let error = match Project::from(Some(project_file)) {
            Ok(_) => panic!("expected missing [project] table to fail"),
            Err(error) => error,
        };

        assert!(error.0.contains("Missing [project] section"));
        assert!(error.0.contains("name`, `version`, and `description"));
        assert_eq!(error.1, 10);
    }

    #[test]
    fn rejects_malformed_optional_project_metadata() {
        let temp = TempDir::new("project-bad-authors");
        let project_file = write_project(
            &temp,
            r#"
            [project]
            name = "Demo"
            version = "1.0.0"
            description = "Demo project"
            authors = 42
            "#,
        );

        let error = match Project::from(Some(project_file)) {
            Ok(_) => panic!("expected malformed authors to fail"),
            Err(error) => error,
        };

        assert!(error.0.contains("Invalid [project].authors"));
        assert!(error.0.contains("authors = [ \"Your Name\" ]"));
        assert_eq!(error.1, 13);
    }

    #[test]
    fn rejects_unknown_project_nature() {
        let temp = TempDir::new("project-unknown-nature");
        let project_file = write_project(
            &temp,
            r#"
            [project]
            name = "Demo"
            version = "1.0.0"
            description = "Demo project"
            natures = [ "eclipse", "unknown" ]
            "#,
        );

        let error = match Project::from(Some(project_file)) {
            Ok(_) => panic!("expected unknown nature to fail"),
            Err(error) => error,
        };

        assert!(error.0.contains("Invalid [project].natures entry"));
        assert!(error.0.contains("supported natures"));
        assert_eq!(error.1, 31);
    }
}
