use std::{collections::HashMap, fs::{create_dir, remove_dir_all, remove_file, write}};

use regex::Regex;

use crate::{configuration::Configuration, eclipse::eq_sep_config, project::Project, template};

#[derive(Clone)]
pub enum Nature
{
    Eclipse,
    Maven
}

impl Nature 
{
    pub fn setup_nature(&self, project: &Project, configuration: &Configuration, regexes: &HashMap<&str, Regex>)
    {
        match self 
        {
            Nature::Eclipse => {
                let _ = create_dir(".settings/");
                let _ = write(".settings/org.eclipse.jdt.core.prefs", eq_sep_config::generate_config(template::generate_eclipse_config(configuration)));
                let _ = write(".settings/org.eclipse.m2e.core.prefs", eq_sep_config::generate_config(template::generate_maven_config()));
                let _ = write(".project", template::generate_project(project).unwrap());

                let _ = write(".classpath", template::generate_classpath(project, configuration, regexes).unwrap());
                let _ = write(".project", template::generate_project(project).unwrap());
            },
            Nature::Maven => {
                let _ = write(".settings/org.eclipse.m2e.core.prefs", eq_sep_config::generate_config(template::generate_eclipse_config(configuration)));
                let _ = write("pom.xml", template::generate_pom(project, configuration).unwrap());
            },
        }
    }

    pub fn remove_nature(&self) -> Result<(), String>
    {
        match self 
        {
            Self::Eclipse => {
                remove_dir_all(".settings").map_err(| e | format!("{e}"))?;
                remove_file(".classpath").map_err(| e | format!("{e}"))?;
                remove_file(".project").map_err(| e | format!("{e}"))?;
            }
            Self::Maven => remove_file("pom.xml").map_err(| e | format!("{e}"))?
        }

        Ok(())
    }

    pub fn type_str(&self) -> &str 
    {
        match self
        {
            Nature::Eclipse => "Eclipse",
            Nature::Maven => "Maven",
        }
    }

    pub fn values() -> Vec<Nature>
    {
        vec![Nature::Eclipse, Nature::Maven]
    }
}

