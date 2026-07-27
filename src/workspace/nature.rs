use std::{
    collections::HashMap,
    fs::{create_dir, remove_dir_all, remove_file, write},
};

use regex::Regex;

use crate::{
    eclipse::eq_sep_config,
    generators::{eclipse, maven},
    model::{Configuration, Project},
    util::consts,
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
    ) {
        match self {
            Nature::Eclipse => {
                let _ = create_dir(consts::ECLIPSE_SETTINGS_DIR);
                let _ = write(
                    consts::ECLIPSE_JDT_PREFS_FILE,
                    eq_sep_config::generate_config(eclipse::generate_eclipse_config(configuration)),
                );
                let _ = write(
                    consts::ECLIPSE_M2E_PREFS_FILE,
                    eq_sep_config::generate_config(eclipse::generate_maven_config()),
                );
                let _ = write(
                    consts::ECLIPSE_PROJECT_FILE,
                    eclipse::generate_project(project).unwrap(),
                );

                let _ = write(
                    consts::ECLIPSE_CLASSPATH_FILE,
                    eclipse::generate_classpath(project, configuration, regexes).unwrap(),
                );
            }
            Nature::Maven => {
                let _ = create_dir(consts::ECLIPSE_SETTINGS_DIR);
                let _ = write(
                    consts::ECLIPSE_M2E_PREFS_FILE,
                    eq_sep_config::generate_config(eclipse::generate_maven_config()),
                );
                let _ = write(
                    consts::MAVEN_POM_FILE,
                    maven::generate_pom(project, configuration).unwrap(),
                );
            }
        }
    }

    pub fn remove_nature(&self) -> Result<(), String> {
        match self {
            Self::Eclipse => {
                remove_dir_all(consts::ECLIPSE_SETTINGS_DIR).map_err(|e| format!("{e}"))?;
                remove_file(consts::ECLIPSE_CLASSPATH_FILE).map_err(|e| format!("{e}"))?;
                remove_file(consts::ECLIPSE_PROJECT_FILE).map_err(|e| format!("{e}"))?;
            }
            Self::Maven => remove_file(consts::MAVEN_POM_FILE).map_err(|e| format!("{e}"))?,
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
