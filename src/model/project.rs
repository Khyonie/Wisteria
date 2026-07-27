use std::{collections::HashMap, fs::read_to_string};

use toml::Table;

use crate::{
    cli::args::StartupFlags, config::toml_utils, dependency::Dependency, model::Configuration,
    util::consts, workspace::nature::Nature
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
    pub fn from(project_file: Option<String>, flags: StartupFlags) -> Result<Self, (String, u8)> {
        let project_toml_string =
            read_to_string(project_file.unwrap_or(String::from(consts::PROJECT_FILE)))
                .map_err(|e| (format!("{e}"), 1))?;

        let project_toml: Table = project_toml_string
            .parse::<Table>()
            .map_err(|e| (format!("Could not read {}: {e}", consts::PROJECT_FILE), 1))?;

        let toml = project_toml.get("project").unwrap().as_table().unwrap();
        let configuration_map = project_toml.get("configuration");
        let dependencies_map = project_toml.get("dependencies");

        let name: String = toml_utils::read_string("name", toml)?;
        let version: String = toml_utils::read_string("version", toml)?;
        let info = ProjectInfo {
            name: name.clone(),
            description: toml_utils::read_string("description", toml)?,
            authors: toml_utils::read_string_array("authors", toml).unwrap_or_default(),
            version: version.clone(),
            license: toml_utils::read_string_array("license", toml).unwrap_or_default(),
            homepage: toml_utils::read_string("homepage", toml).ok(),
            sourcepage: toml_utils::read_string("sourcepage", toml).ok(),
            natures: {
                let natures = toml_utils::read_string_array("natures", toml)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|v| match v.as_str() {
                        "eclipse" => Some(Nature::Eclipse),
                        "maven" => Some(Nature::Maven),
                        _ => None,
                    })
                    .collect();

                natures
            },
            configurations: match configuration_map {
                Some(v) if v.is_table() => {
                    let v = v.as_table().unwrap();
                    let mut configurations: HashMap<String, Configuration> = HashMap::new();

                    for key in v.keys() {
                        match v.get(key)
                        {
                            Some(config) if config.is_table() => {
                                let mut configuration = Configuration::from(key.clone(), config.as_table().unwrap(), name.clone(), version.clone())?;
                                configuration.apply_implicit(flags.clone());
                                configurations.insert(key.clone(), configuration)
                            }
                            Some(v) => return Err((format!("Mismatched type for task \"{key}\", expected a table, found {}", v.type_str()), 16)),
                            None => None
                        };
                    }

                    let mut updated_configurations: HashMap<String, Configuration> = HashMap::new();
                    for (config_name, configuration) in configurations.iter() {
                        if let Some(target) = configuration.inherits() {
                            if config_name.eq(target) {
                                return Err((format!("Configuration \"{config_name}\" cannot inherit from itself"), 40));
                            }

                            let target = match configurations.get(target)
                            {
                                Some(c) => c,
                                None => return Err((format!("No such configuration \"{target}\" to be inherited by \"{config_name}\""), 41))
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
                            "Mismatched type for \"configuration\", expected a table, found {}",
                            v.type_str()
                        ),
                        16,
                    ))
                }
                None => HashMap::new(),
            },
        };

        let dependencies: HashMap<String, Dependency> = match dependencies_map {
            Some(v) if v.is_table() => {
                let v = v.as_table().unwrap();
                let mut dependencies: HashMap<String, Dependency> = HashMap::new();

                for (name, t) in v {
                    match t.as_table()
                    {
                        Some(t) => dependencies.insert(name.clone(), Dependency::load(t)?),
                        None => return Err((format!("Mismatched type for dependency \"{name}\", expected a table, found {}", t.type_str()), 16))
                    };
                }

                dependencies
            }
            Some(v) => {
                return Err((
                    format!(
                        "Mismatched type for \"dependencies\", expected a table, found {}",
                        v.type_str()
                    ),
                    16,
                ))
            }
            None => HashMap::new(),
        };

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
        assert_eq!(project.info().homepage().map(String::as_str), Some("https://example.com"));
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
        assert_eq!(child.targets().unwrap(), &vec![String::from("target/base.jar")]);
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
}
