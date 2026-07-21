use std::{
    collections::HashMap,
    fs::{create_dir_all, remove_dir_all, remove_file, write}, io::ErrorKind,
};

use regex::Regex;

use crate::{
    eclipse::eq_sep_config,
    generators::{eclipse, maven},
    model::{Configuration, Project},
};

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
                create_dir_all(".settings/").map_err(| e | format!("{e}"))?;
                write(
                    ".settings/org.eclipse.jdt.core.prefs",
                    eq_sep_config::generate_config(eclipse::generate_eclipse_config(configuration)),
                ).map_err(| e | format!("{e}"))?;
                write(
                    ".settings/org.eclipse.m2e.core.prefs",
                    eq_sep_config::generate_config(eclipse::generate_maven_config()),
                ).map_err(| e | format!("{e}"))?;

                write(".project", eclipse::generate_project(project)?).map_err(| e | format!("{e}"))?;

                write(
                    ".classpath",
                    eclipse::generate_classpath(project, configuration, regexes)?,
                ).map_err(| e | format!("{e}"))?;

                Ok(())
            }
            Nature::Maven => {
                create_dir_all(".settings/").map_err(| e | format!("{e}"))?;
                write(
                    ".settings/org.eclipse.m2e.core.prefs",
                    eq_sep_config::generate_config(eclipse::generate_maven_config()),
                ).map_err(| e | format!("{e}"))?;
                write(
                    "pom.xml",
                    maven::generate_pom(project, configuration)?,
                ).map_err(| e | format!("{e}"))?;

                Ok(())
            }
        }
    }

    pub fn remove_nature(&self) -> Result<(), String> {
        match self {
            Self::Eclipse => {
                 if let Err(e) = remove_dir_all(".settings") {
                    if e.kind() != ErrorKind::NotFound {
                        return Err(format!("{e}"))
                    }
                }
                if let Err(e) = remove_file(".classpath") {
                    if e.kind() != ErrorKind::NotFound {
                        return Err(format!("{e}"))
                    }
                }

                if let Err(e) = remove_file(".project") {
                    if e.kind() != ErrorKind::NotFound {
                        return Err(format!("{e}"))
                    }
                }
            }
            Self::Maven => {
                if let Err(e) = remove_file("pom.xml") {
                    if e.kind() != ErrorKind::NotFound {
                        return Err(format!("{e}"))
                    }
                }
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
