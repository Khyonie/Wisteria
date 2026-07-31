use std::{
    collections::HashMap,
    fs::{create_dir_all, remove_dir_all, remove_file, write},
    io::ErrorKind,
};

use regex::Regex;

use crate::{
    eclipse::eq_sep_config,
    generators::{eclipse, maven},
    model::{Configuration, Project},
    util::consts,
};

fn ignore_not_found(result: std::io::Result<()>) -> Result<(), String> {
    match result {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("{e}")),
    }
}

#[derive(Clone)]
pub enum Nature {
    Eclipse,
    Maven,
}

impl Nature {
    pub fn setup_nature(
        &self,
        project: &Project,
        configuration: &Configuration,
        regexes: &HashMap<&str, Regex>,
    ) -> Result<(), String> {
        match self {
            Nature::Eclipse => {
                create_dir_all(consts::ECLIPSE_SETTINGS_DIR).map_err(|e| format!("{e}"))?;
                write(
                    consts::ECLIPSE_JDT_PREFS_FILE,
                    eq_sep_config::generate_config(eclipse::generate_eclipse_config(configuration)),
                )
                .map_err(|e| format!("{e}"))?;
                write(
                    consts::ECLIPSE_M2E_PREFS_FILE,
                    eq_sep_config::generate_config(eclipse::generate_maven_config()),
                )
                .map_err(|e| format!("{e}"))?;

                write(
                    consts::ECLIPSE_PROJECT_FILE,
                    eclipse::generate_project(project)?,
                )
                .map_err(|e| format!("{e}"))?;

                write(
                    consts::ECLIPSE_CLASSPATH_FILE,
                    eclipse::generate_classpath(project, configuration, regexes)?,
                )
                .map_err(|e| format!("{e}"))?;

                Ok(())
            }
            Nature::Maven => {
                create_dir_all(consts::ECLIPSE_SETTINGS_DIR).map_err(|e| format!("{e}"))?;
                write(
                    consts::ECLIPSE_M2E_PREFS_FILE,
                    eq_sep_config::generate_config(eclipse::generate_maven_config()),
                )
                .map_err(|e| format!("{e}"))?;
                write(
                    consts::MAVEN_POM_FILE,
                    maven::generate_pom(project, configuration)?,
                )
                .map_err(|e| format!("{e}"))?;

                Ok(())
            }
        }
    }

    pub fn remove_nature(&self) -> Result<(), String> {
        match self {
            Self::Eclipse => {
                ignore_not_found(remove_dir_all(consts::ECLIPSE_SETTINGS_DIR))?;
                ignore_not_found(remove_file(consts::ECLIPSE_CLASSPATH_FILE))?;
                ignore_not_found(remove_file(consts::ECLIPSE_PROJECT_FILE))?;
            }
            Self::Maven => {
                ignore_not_found(remove_file(consts::MAVEN_POM_FILE))?;
                ignore_not_found(remove_file(consts::ECLIPSE_M2E_PREFS_FILE))?;
            }
        }

        Ok(())
    }

    pub fn type_str(&self) -> &str {
        match self {
            Nature::Eclipse => "Eclipse",
            Nature::Maven => "Maven",
        }
    }

    pub fn values() -> Vec<Nature> {
        vec![Nature::Eclipse, Nature::Maven]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TempDir, with_current_dir};
    use std::fs;

    fn regexes() -> HashMap<&'static str, Regex> {
        let mut regexes = HashMap::new();
        regexes.insert("envvars", Regex::new(r#"\{(.+?)}"#).unwrap());
        regexes
    }

    fn project_from_toml(temp: &TempDir, contents: &str) -> Project {
        let project_file = temp.path().join("project.toml");
        fs::write(&project_file, contents).unwrap();

        Project::from(Some(project_file.to_string_lossy().to_string())).unwrap()
    }

    #[test]
    fn remove_nature_ignores_missing_generated_files() {
        let temp = TempDir::new("nature-remove-missing");

        with_current_dir(temp.path(), || {
            Nature::Eclipse.remove_nature().unwrap();
            Nature::Maven.remove_nature().unwrap();
        });
    }

    #[test]
    fn remove_maven_nature_removes_pom_and_maven_settings_file() {
        let temp = TempDir::new("nature-remove-maven");
        fs::create_dir_all(temp.path().join(".settings")).unwrap();
        fs::write(temp.path().join("pom.xml"), "").unwrap();
        fs::write(temp.path().join(".settings/org.eclipse.m2e.core.prefs"), "").unwrap();

        with_current_dir(temp.path(), || {
            Nature::Maven.remove_nature().unwrap();
        });

        assert!(!temp.path().join("pom.xml").exists());
        assert!(
            !temp
                .path()
                .join(".settings/org.eclipse.m2e.core.prefs")
                .exists()
        );
    }

    #[test]
    fn setup_eclipse_nature_writes_expected_workspace_files() {
        let temp = TempDir::new("nature-setup-eclipse");
        let project = project_from_toml(
            &temp,
            r#"
            [project]
            name = "Demo"
            version = "1.0.0"
            description = "Demo"
            natures = [ "eclipse" ]

            [configuration.main]
            sources = [ "src/" ]
            dependencies = [ ]
            targets = [ "target/demo.jar" ]
            "#,
        );
        let configuration = project.info().configurations().get("main").unwrap();

        with_current_dir(temp.path(), || {
            Nature::Eclipse
                .setup_nature(&project, configuration, &regexes())
                .unwrap();
        });

        assert!(temp.path().join(".project").exists());
        assert!(temp.path().join(".classpath").exists());
        assert!(
            temp.path()
                .join(".settings/org.eclipse.jdt.core.prefs")
                .exists()
        );
        assert!(
            temp.path()
                .join(".settings/org.eclipse.m2e.core.prefs")
                .exists()
        );
    }

    #[test]
    fn setup_maven_nature_writes_pom_and_settings() {
        let temp = TempDir::new("nature-setup-maven");
        let project = project_from_toml(
            &temp,
            r#"
            [project]
            name = "Demo"
            version = "1.0.0"
            description = "Demo"
            natures = [ "maven" ]

            [configuration.main]
            dependencies = [ ]
            "#,
        );
        let configuration = project.info().configurations().get("main").unwrap();

        with_current_dir(temp.path(), || {
            Nature::Maven
                .setup_nature(&project, configuration, &regexes())
                .unwrap();
        });

        assert!(temp.path().join("pom.xml").exists());
        assert!(
            temp.path()
                .join(".settings/org.eclipse.m2e.core.prefs")
                .exists()
        );
    }
}
