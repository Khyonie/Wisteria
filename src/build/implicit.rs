use std::collections::HashMap;

use regex::Regex;

use crate::build::resolve::resolve_dependencies;
use crate::build::task::TaskRunner;
use crate::build::{compile, package, shade, sources};
use crate::model::{Configuration, Project, ProjectInfo};

#[derive(Clone)]
pub struct ImplicitBuildTask {
    order: Vec<String>,
}

impl ImplicitBuildTask {
    pub fn new() -> Self {
        ImplicitBuildTask {
            order: vec![
                String::from("collect"),
                String::from("compile"),
                String::from("shade"),
                String::from("package"),
            ],
        }
    }
}

impl ImplicitBuildTask {
    pub fn new() -> Self {
        Self::default()
    }
}

impl TaskRunner for ImplicitBuildTask {
    fn invoke(
        &self,
        _info: &ProjectInfo,
        project: &Project,
        configuration: &Configuration,
    ) -> Result<(), (String, u8)> {
        let mut regexes: HashMap<&str, Regex> = HashMap::new();
        regexes.insert("envvars", Regex::new(r#"\{(.+?)}"#).unwrap());

        let dependencies = resolve_dependencies(project, configuration, &regexes)?;
        let copied_files = sources::collect_sources(configuration)?;

        compile::compile_sources(
            configuration,
            copied_files,
            dependencies.classpath().as_deref(),
        )?;
        shade::shade_jars(&dependencies.shaded_jars())?;
        package::package_jar(
            configuration,
            &dependencies.paths(),
            &dependencies.shaded_jars(),
            configuration.targets(),
            &regexes,
        )?;

        Ok(())
    }

    fn phase_order(&self) -> &[String] {
        self.order.as_ref()
    }
}
