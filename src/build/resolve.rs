use std::{collections::HashMap, path::PathBuf};

use regex::Regex;

use crate::{
    dependency::resolver::ResolveContext,
    model::lockfile::try_read_lockfile,
    model::{Configuration, Project},
    project::UpdateContext,
    util::consts,
};

pub struct ResolvedDependencies {
    paths: Vec<PathBuf>,
    shaded_jars: Vec<PathBuf>,
    classpath: Option<String>,
}

impl ResolvedDependencies {
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    pub fn shaded_jars(&self) -> &[PathBuf] {
        &self.shaded_jars
    }

    pub fn classpath(&self) -> Option<String> {
        self.classpath.clone()
    }
}

pub(crate) fn resolve_dependencies(
    project: &Project,
    configuration: &Configuration,
    regexes: &HashMap<&str, Regex>,
) -> Result<ResolvedDependencies, String> {
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut compile_paths: Vec<PathBuf> = Vec::new();
    let mut shaded_jars: Vec<PathBuf> = Vec::new();
    let mut classpath: Option<String> = None;
    let lockfile = try_read_lockfile()?;

    let mut failed_downloads: Vec<(String, String)> = Vec::new();
    if let Some(dependencies) = configuration.dependencies() {
        let mut width: usize = usize::MIN;
        for reference in dependencies.iter() {
            width = usize::max(reference.name().len(), width);
        }

        width += 5;
        let size = dependencies.len();

        for (index, reference) in dependencies.iter().enumerate() {
            let Some((name, dep)) = project.dependencies().get_key_value(reference.name()) else {
                println!("Usage of undeclared dependency \"{}\"", reference.name());
                failed_downloads.push((
                    reference.name().to_string(),
                    String::from("dependency is not declared in [dependencies]"),
                ));
                continue;
            };

            if reference.scope().is_test_only() {
                continue;
            }

            {
                print!(
                    "({}/{size}) Updating {:width$}",
                    index + 1,
                    format!("{name} ... ")
                );
                let updated = match dep.resolve(
                    name,
                    configuration.environment(),
                    regexes,
                    ResolveContext::for_dependency(
                        UpdateContext::TaskInvoked,
                        lockfile.as_ref(),
                        name,
                    ),
                ) {
                    Ok(p) => p,
                    Err(e) => {
                        println!("Could not download {name}: {e}");
                        failed_downloads.push((name.clone(), e));
                        continue;
                    }
                };

                if reference.is_shaded() {
                    shaded_jars.extend(updated.paths().cloned());
                }

                if reference.scope().is_on_compile_classpath() {
                    compile_paths.extend(updated.paths().cloned());
                }

                if reference.scope().is_on_runtime_classpath() && !reference.is_shaded() {
                    paths.extend(updated.paths().cloned());
                }
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

            return Err(String::from("Could not resolve all dependencies"));
        }

        println!("Successfully resolved all dependencies!");
        let mut buffer: String = String::new();
        for dep in &compile_paths {
            buffer.push_str(&dep.to_string_lossy());
            buffer.push(consts::java_seperator());
        }

        if !buffer.is_empty() {
            buffer.pop();
            classpath = Some(buffer);
        }
    }

    Ok(ResolvedDependencies {
        paths,
        shaded_jars,
        classpath,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{model::Project, test_support::TempDir};
    use std::fs;

    fn regexes() -> HashMap<&'static str, Regex> {
        let mut regexes = HashMap::new();
        regexes.insert("envvars", Regex::new(r#"\{(.+?)}"#).unwrap());
        regexes
    }

    fn contains_path(paths: &[PathBuf], path: &PathBuf) -> bool {
        paths.iter().any(|candidate| candidate == path)
    }

    #[test]
    fn dependency_references_drive_classpath_and_shading_views() {
        let temp = TempDir::new("resolve-dependency-references");
        let compile = temp.path().join("compile.jar");
        let provided = temp.path().join("provided.jar");
        let runtime = temp.path().join("runtime.jar");
        let shaded = temp.path().join("shaded.jar");
        let test = temp.path().join("test.jar");
        for jar in [&compile, &provided, &runtime, &shaded, &test] {
            fs::write(jar, "").unwrap();
        }

        let project_file = temp.path().join("project.toml");
        fs::write(
            &project_file,
            format!(
                r#"
                [project]
                name = "Demo"
                version = "1.0.0"
                description = "Demo"

                [dependencies.archive]
                compile_dep = {{ path = "{}" }}
                provided_dep = {{ path = "{}" }}
                runtime_dep = {{ path = "{}" }}
                shaded_dep = {{ path = "{}" }}
                test_dep = {{ path = "{}" }}

                [configuration.main]
                dependencies = [
                    {{ name = "compile_dep", scope = "compile" }},
                    {{ name = "provided_dep", scope = "provided" }},
                    {{ name = "runtime_dep", scope = "runtime" }},
                    {{ name = "shaded_dep", scope = "compile", package = "shade" }},
                    {{ name = "test_dep", scope = "test" }},
                ]
                "#,
                compile.display(),
                provided.display(),
                runtime.display(),
                shaded.display(),
                test.display(),
            ),
        )
        .unwrap();

        let project = Project::from(Some(project_file.to_string_lossy().to_string())).unwrap();
        let configuration = project.info().configurations().get("main").unwrap();
        let resolved = resolve_dependencies(&project, configuration, &regexes()).unwrap();
        let compile = compile.canonicalize().unwrap();
        let provided = provided.canonicalize().unwrap();
        let runtime = runtime.canonicalize().unwrap();
        let shaded = shaded.canonicalize().unwrap();
        let test = test.canonicalize().unwrap();
        let classpath = resolved.classpath().unwrap();

        assert!(classpath.contains(&compile.to_string_lossy().to_string()));
        assert!(classpath.contains(&provided.to_string_lossy().to_string()));
        assert!(classpath.contains(&shaded.to_string_lossy().to_string()));
        assert!(!classpath.contains(&runtime.to_string_lossy().to_string()));
        assert!(!classpath.contains(&test.to_string_lossy().to_string()));

        assert!(contains_path(resolved.paths(), &compile));
        assert!(contains_path(resolved.paths(), &runtime));
        assert!(!contains_path(resolved.paths(), &provided));
        assert!(!contains_path(resolved.paths(), &shaded));
        assert!(!contains_path(resolved.paths(), &test));
        assert_eq!(resolved.shaded_jars(), &[shaded]);
    }
}
