use std::{collections::HashMap, path::PathBuf};

use regex::Regex;

use crate::{
    dependency::reference::{DependencyReference, DependencyScope},
    dependency::resolver::ResolveContext,
    model::lockfile::try_read_lockfile,
    model::{Configuration, Project},
    project::UpdateContext,
    util::consts,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClasspathView {
    MainCompile,
    MainRuntime,
    PackageRuntime,
    TestCompile,
    TestRuntime,
}

#[derive(Debug)]
pub struct ResolvedDependencies {
    main_compile_paths: Vec<PathBuf>,
    main_runtime_paths: Vec<PathBuf>,
    package_runtime_paths: Vec<PathBuf>,
    test_compile_paths: Vec<PathBuf>,
    test_runtime_paths: Vec<PathBuf>,
    shaded_jars: Vec<PathBuf>,
}

impl ResolvedDependencies {
    pub fn paths_for(&self, view: ClasspathView) -> &[PathBuf] {
        match view {
            ClasspathView::MainCompile => &self.main_compile_paths,
            ClasspathView::MainRuntime => &self.main_runtime_paths,
            ClasspathView::PackageRuntime => &self.package_runtime_paths,
            ClasspathView::TestCompile => &self.test_compile_paths,
            ClasspathView::TestRuntime => &self.test_runtime_paths,
        }
    }

    pub fn classpath_for(&self, view: ClasspathView) -> Option<String> {
        join_classpath(self.paths_for(view))
    }

    pub fn paths(&self) -> &[PathBuf] {
        self.paths_for(ClasspathView::PackageRuntime)
    }

    pub fn shaded_jars(&self) -> &[PathBuf] {
        &self.shaded_jars
    }

    pub fn classpath(&self) -> Option<String> {
        self.classpath_for(ClasspathView::MainCompile)
    }
}

pub(crate) fn resolve_dependencies(
    project: &Project,
    configuration: &Configuration,
    regexes: &HashMap<&str, Regex>,
) -> Result<ResolvedDependencies, String> {
    resolve_dependencies_for_views(
        project,
        configuration,
        regexes,
        &[ClasspathView::MainCompile, ClasspathView::PackageRuntime],
    )
}

pub(crate) fn resolve_dependencies_for_views(
    project: &Project,
    configuration: &Configuration,
    regexes: &HashMap<&str, Regex>,
    views: &[ClasspathView],
) -> Result<ResolvedDependencies, String> {
    let mut main_compile_paths: Vec<PathBuf> = Vec::new();
    let mut main_runtime_paths: Vec<PathBuf> = Vec::new();
    let mut package_runtime_paths: Vec<PathBuf> = Vec::new();
    let mut test_compile_paths: Vec<PathBuf> = Vec::new();
    let mut test_runtime_paths: Vec<PathBuf> = Vec::new();
    let mut shaded_jars: Vec<PathBuf> = Vec::new();
    let lockfile = try_read_lockfile()?;

    let mut failed_downloads: Vec<(String, String)> = Vec::new();
    if let Some(dependencies) = configuration.dependencies() {
        for reference in dependencies.iter() {
            let Some((name, dep)) = project.dependencies().get_key_value(reference.name()) else {
                failed_downloads.push((
                    reference.name().to_string(),
                    String::from("dependency is not declared in [dependencies]"),
                ));
                continue;
            };

            if !is_needed_for_views(reference, views) {
                continue;
            }

            {
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
                        failed_downloads.push((name.clone(), e));
                        continue;
                    }
                };

                if should_shade(reference, views) {
                    shaded_jars.extend(updated.paths().cloned());
                }

                for view in views {
                    if view.includes(reference) {
                        paths_for_view_mut(
                            *view,
                            &mut main_compile_paths,
                            &mut main_runtime_paths,
                            &mut package_runtime_paths,
                            &mut test_compile_paths,
                            &mut test_runtime_paths,
                        )
                        .extend(updated.paths().cloned());
                    }
                }
            }
        }

        if !failed_downloads.is_empty() {
            return Err(format_failed_dependencies(failed_downloads));
        }
    }

    Ok(ResolvedDependencies {
        main_compile_paths,
        main_runtime_paths,
        package_runtime_paths,
        test_compile_paths,
        test_runtime_paths,
        shaded_jars,
    })
}

impl ClasspathView {
    fn includes(self, reference: &DependencyReference) -> bool {
        match self {
            Self::MainCompile => matches!(
                reference.scope(),
                DependencyScope::Compile | DependencyScope::Provided
            ),
            Self::MainRuntime => matches!(
                reference.scope(),
                DependencyScope::Compile | DependencyScope::Runtime
            ),
            Self::PackageRuntime => {
                matches!(
                    reference.scope(),
                    DependencyScope::Compile | DependencyScope::Runtime
                ) && !reference.is_shaded()
            }
            Self::TestCompile => matches!(
                reference.scope(),
                DependencyScope::Compile | DependencyScope::Provided | DependencyScope::Test
            ),
            Self::TestRuntime => matches!(
                reference.scope(),
                DependencyScope::Compile
                    | DependencyScope::Provided
                    | DependencyScope::Runtime
                    | DependencyScope::Test
            ),
        }
    }
}

fn is_needed_for_views(reference: &DependencyReference, views: &[ClasspathView]) -> bool {
    views.iter().any(|view| view.includes(reference)) || should_shade(reference, views)
}

fn should_shade(reference: &DependencyReference, views: &[ClasspathView]) -> bool {
    views.contains(&ClasspathView::PackageRuntime)
        && reference.is_shaded()
        && !reference.scope().is_test_only()
}

fn paths_for_view_mut<'a>(
    view: ClasspathView,
    main_compile_paths: &'a mut Vec<PathBuf>,
    main_runtime_paths: &'a mut Vec<PathBuf>,
    package_runtime_paths: &'a mut Vec<PathBuf>,
    test_compile_paths: &'a mut Vec<PathBuf>,
    test_runtime_paths: &'a mut Vec<PathBuf>,
) -> &'a mut Vec<PathBuf> {
    match view {
        ClasspathView::MainCompile => main_compile_paths,
        ClasspathView::MainRuntime => main_runtime_paths,
        ClasspathView::PackageRuntime => package_runtime_paths,
        ClasspathView::TestCompile => test_compile_paths,
        ClasspathView::TestRuntime => test_runtime_paths,
    }
}

fn join_classpath(paths: &[PathBuf]) -> Option<String> {
    let mut buffer = String::new();
    for path in paths {
        buffer.push_str(&path.to_string_lossy());
        buffer.push(consts::java_seperator());
    }

    if buffer.is_empty() {
        return None;
    }

    buffer.pop();
    Some(buffer)
}

fn format_failed_dependencies(failed_downloads: Vec<(String, String)>) -> String {
    let mut message = format!(
        "Failed to resolve {} {}:",
        failed_downloads.len(),
        if failed_downloads.len() == 1 {
            "dependency"
        } else {
            "dependencies"
        }
    );

    for (name, error) in failed_downloads {
        message.push_str(&format!("\n- {name}: {error}"));
    }

    message
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

    fn assert_contains(paths: &[PathBuf], path: &PathBuf) {
        assert!(
            contains_path(paths, path),
            "expected {} in {paths:?}",
            path.display()
        );
    }

    fn assert_excludes(paths: &[PathBuf], path: &PathBuf) {
        assert!(
            !contains_path(paths, path),
            "did not expect {} in {paths:?}",
            path.display()
        );
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
        let resolved = resolve_dependencies_for_views(
            &project,
            configuration,
            &regexes(),
            &[
                ClasspathView::MainCompile,
                ClasspathView::MainRuntime,
                ClasspathView::PackageRuntime,
                ClasspathView::TestCompile,
                ClasspathView::TestRuntime,
            ],
        )
        .unwrap();
        let compile = compile.canonicalize().unwrap();
        let provided = provided.canonicalize().unwrap();
        let runtime = runtime.canonicalize().unwrap();
        let shaded = shaded.canonicalize().unwrap();
        let test = test.canonicalize().unwrap();
        let main_compile_classpath = resolved.classpath_for(ClasspathView::MainCompile).unwrap();

        assert!(main_compile_classpath.contains(&compile.to_string_lossy().to_string()));
        assert!(main_compile_classpath.contains(&provided.to_string_lossy().to_string()));
        assert!(main_compile_classpath.contains(&shaded.to_string_lossy().to_string()));
        assert!(!main_compile_classpath.contains(&runtime.to_string_lossy().to_string()));
        assert!(!main_compile_classpath.contains(&test.to_string_lossy().to_string()));

        let main_compile = resolved.paths_for(ClasspathView::MainCompile);
        assert_contains(main_compile, &compile);
        assert_contains(main_compile, &provided);
        assert_contains(main_compile, &shaded);
        assert_excludes(main_compile, &runtime);
        assert_excludes(main_compile, &test);

        let main_runtime = resolved.paths_for(ClasspathView::MainRuntime);
        assert_contains(main_runtime, &compile);
        assert_contains(main_runtime, &runtime);
        assert_contains(main_runtime, &shaded);
        assert_excludes(main_runtime, &provided);
        assert_excludes(main_runtime, &test);

        let package_runtime = resolved.paths_for(ClasspathView::PackageRuntime);
        assert_contains(package_runtime, &compile);
        assert_contains(package_runtime, &runtime);
        assert_excludes(package_runtime, &provided);
        assert_excludes(package_runtime, &shaded);
        assert_excludes(package_runtime, &test);

        let test_compile = resolved.paths_for(ClasspathView::TestCompile);
        assert_contains(test_compile, &compile);
        assert_contains(test_compile, &provided);
        assert_contains(test_compile, &shaded);
        assert_contains(test_compile, &test);
        assert_excludes(test_compile, &runtime);

        let test_runtime = resolved.paths_for(ClasspathView::TestRuntime);
        assert_contains(test_runtime, &compile);
        assert_contains(test_runtime, &provided);
        assert_contains(test_runtime, &runtime);
        assert_contains(test_runtime, &shaded);
        assert_contains(test_runtime, &test);

        assert_eq!(resolved.paths(), package_runtime);
        assert_eq!(
            resolved.classpath(),
            resolved.classpath_for(ClasspathView::MainCompile)
        );
        assert_eq!(resolved.shaded_jars(), &[shaded]);
    }

    #[test]
    fn default_dependency_resolution_skips_test_only_dependencies() {
        let temp = TempDir::new("resolve-skip-test-dependencies");
        let compile = temp.path().join("compile.jar");
        let missing_test = temp.path().join("missing-test.jar");
        fs::write(&compile, "").unwrap();

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
                test_dep = {{ path = "{}" }}

                [configuration.main]
                dependencies = [
                    {{ name = "compile_dep", scope = "compile" }},
                    {{ name = "test_dep", scope = "test" }},
                ]
                "#,
                compile.display(),
                missing_test.display(),
            ),
        )
        .unwrap();

        let project = Project::from(Some(project_file.to_string_lossy().to_string())).unwrap();
        let configuration = project.info().configurations().get("main").unwrap();

        resolve_dependencies(&project, configuration, &regexes()).unwrap();
        let error = resolve_dependencies_for_views(
            &project,
            configuration,
            &regexes(),
            &[ClasspathView::TestCompile],
        )
        .unwrap_err();

        assert!(error.contains("test_dep"));
        assert!(error.contains("does not exist"));
    }
}
