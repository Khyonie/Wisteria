use std::{collections::HashMap, path::PathBuf};

use regex::Regex;

use crate::build::task::TaskRunner;
use crate::build::{compile, package, shade, sources};
use crate::dependency::UpdateContext;
use crate::model::{Configuration, Project, ProjectInfo};
use crate::util::consts;

#[derive(Clone)]
pub struct ImplicitBuildTask {
    order: Vec<String>,
}

struct ResolvedDependencies {
    paths: Vec<PathBuf>,
    shaded_jars: Vec<PathBuf>,
    classpath: Option<String>,
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
            dependencies.classpath.as_deref(),
        )?;
        shade::shade_jars(&dependencies.shaded_jars)?;
        package::package_jar(
            configuration,
            &dependencies.paths,
            &dependencies.shaded_jars,
            &regexes,
        )?;

        Ok(())
    }

    fn phase_order(&self) -> &[String] {
        self.order.as_ref()
    }
}

fn resolve_dependencies(
    project: &Project,
    configuration: &Configuration,
    regexes: &HashMap<&str, Regex>,
) -> Result<ResolvedDependencies, (String, u8)> {
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut shaded_jars: Vec<PathBuf> = Vec::new();
    let mut classpath: Option<String> = None;

    let mut failed_downloads: Vec<(String, String)> = Vec::new();
    if let Some(dependencies) = configuration.dependencies() {
        let mut width: usize = usize::MIN;
        for name in dependencies.iter() {
            width = usize::max(name.len(), width);
        }

        width += 5;
        let size = dependencies.len();

        for (index, d) in dependencies.iter().enumerate() {
            if let Some((name, dep)) = project.dependencies().get_key_value(d) {
                print!(
                    "({}/{size}) Updating {:width$}",
                    index + 1,
                    format!("{name} ... ")
                );
                let mut updated = match dep.resolve(
                    name,
                    configuration.environment(),
                    regexes,
                    UpdateContext::TaskInvoked,
                ) {
                    Ok(p) => p,
                    Err(e) => {
                        println!("Could not download {name}: {}", e.0);
                        failed_downloads.push((name.clone(), e.0));
                        continue;
                    }
                };

                if dep.is_shaded(name, configuration).is_some_and(|s| s) {
                    shaded_jars.append(&mut updated.clone());
                }

                paths.append(&mut updated);
            }
        }

        if !failed_downloads.is_empty() {
            println!("Failed to resolve {} {}:", failed_downloads.len(), {
                if failed_downloads.len() == 1 {
                    "dependency"
                } else {
                    "dependencies"
                }
            });
            for (name, error) in failed_downloads {
                println!("- {name}: {error}");
            }

            return Err((String::from("Could not resolve all dependencies"), 1));
        }

        println!("Successfully resolved all dependencies!");
        let mut buffer: String = String::new();
        for dep in &paths {
            buffer.push_str(&dep.to_string_lossy());
            buffer.push(consts::java_seperator());
        }

        buffer.pop();
        classpath = Some(buffer);
    }

    Ok(ResolvedDependencies {
        paths,
        shaded_jars,
        classpath,
    })
}
