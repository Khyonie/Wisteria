use std::collections::HashMap;

use regex::Regex;

use crate::build::resolve::resolve_dependencies;
use crate::build::task::{TaskOutput, TaskRunner};
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
                String::from("resolve"),
                String::from("collect"),
                String::from("compile"),
                String::from("shade"),
                String::from("package"),
            ],
        }
    }
}

impl Default for ImplicitBuildTask {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskRunner for ImplicitBuildTask {
    fn invoke(
        &self,
        _info: &ProjectInfo,
        project: &Project,
        configuration: &Configuration,
        output: &mut TaskOutput<'_>,
    ) -> Result<(), String> {
        let mut regexes: HashMap<&str, Regex> = HashMap::new();
        regexes.insert("envvars", Regex::new(r#"\{(.+?)}"#).unwrap());

        output.step_started("Resolving", "dependencies", 1);
        let dependencies = match resolve_dependencies(project, configuration, &regexes) {
            Ok(dependencies) => {
                output.step_completed(
                    "Resolving",
                    "dependencies",
                    1,
                    resolve_message(configuration),
                );
                dependencies
            }
            Err(error) => {
                output.step_failed("Resolving", "dependencies", 1, &error);
                return Err(error);
            }
        };

        output.step_started("Collecting", "sources", 2);
        let copied_files = match sources::collect_sources(configuration) {
            Ok(copied_files) => {
                output.step_completed(
                    "Collecting",
                    "sources",
                    2,
                    &format!(
                        "{} source {}",
                        copied_files.len(),
                        plural(copied_files.len())
                    ),
                );
                copied_files
            }
            Err(error) => {
                output.step_failed("Collecting", "sources", 2, &error);
                return Err(error);
            }
        };

        output.step_started("Compiling", "classes", 3);
        if let Err(error) = compile::compile_sources(
            configuration,
            copied_files.clone(),
            dependencies.classpath().as_deref(),
            output.renderer(),
        ) {
            output.step_failed("Compiling", "classes", 3, &error);
            return Err(error);
        }
        output.step_completed(
            "Compiling",
            "classes",
            3,
            &format!(
                "{} source {}",
                copied_files.len(),
                plural(copied_files.len())
            ),
        );

        output.step_started("Shading", "dependencies", 4);
        if let Err(error) = shade::shade_jars(dependencies.shaded_jars()) {
            output.step_failed("Shading", "dependencies", 4, &error);
            return Err(error);
        }
        output.step_completed(
            "Shading",
            "dependencies",
            4,
            shade_message(dependencies.shaded_jars().len()),
        );

        output.step_started("Packaging", "jar", 5);
        let package_hash = match package::package_jar(
            configuration,
            dependencies.paths(),
            dependencies.shaded_jars(),
            configuration.targets(),
            &regexes,
            output.renderer(),
        ) {
            Ok(package_hash) => package_hash,
            Err(error) => {
                output.step_failed("Packaging", "jar", 5, &error);
                return Err(error);
            }
        };
        output.step_completed("Packaging", "jar", 5, &format!("Hash #{package_hash}"));

        Ok(())
    }

    fn phase_order(&self) -> &[String] {
        self.order.as_ref()
    }
}

fn resolve_message(configuration: &Configuration) -> &'static str {
    match configuration
        .dependencies()
        .map(|dependencies| {
            dependencies
                .iter()
                .filter(|reference| !reference.scope().is_test_only())
                .count()
        })
        .unwrap_or(0)
    {
        0 => "No dependencies",
        _ => "Done",
    }
}

fn shade_message(shaded_jars: usize) -> &'static str {
    match shaded_jars {
        0 => "No shaded jars",
        _ => "Done",
    }
}

fn plural(count: usize) -> &'static str {
    match count {
        1 => "file",
        _ => "files",
    }
}
