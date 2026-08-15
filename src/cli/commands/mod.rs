use std::{collections::HashMap, process::exit};

use regex::Regex;

use crate::dependency::resolver::{ResolveContext, ResolvedDependency};
use crate::dependency::{Dependency, UpdateContext};
use crate::model::{Configuration, Lockfile, Project};
use crate::output::OutputRenderer;

pub mod clean;
pub mod create;
pub mod dependencies;
pub mod fetch;
pub mod info;
pub mod migrate;
pub mod refresh;
pub mod switch;
pub mod sync;
pub mod task;
pub mod update;
pub mod verify;

pub(crate) fn print_header() {
    println!("Wisteria v{}", env!("CARGO_PKG_VERSION"));
    println!("Copyright © 2026 Hailey-Jane \"Khyonie\" Garrett <http://www.khyonieheart.coffee/>");
}

pub(crate) fn envvar_regexes() -> HashMap<&'static str, Regex> {
    let mut regexes: HashMap<&str, Regex> = HashMap::new();
    regexes.insert("envvars", Regex::new(r#"\{(.+?)}"#).unwrap());
    regexes
}

pub(crate) fn project_or_exit(project: Result<Project, String>) -> Project {
    match project {
        Ok(project) => project,
        Err(message) => {
            println!("Could not load Wisteria project configuration.\n\n{message}");
            exit(1)
        }
    }
}

pub(crate) fn configuration_or_exit<'a>(
    project: &'a Project,
    configuration_name: &str,
) -> &'a Configuration {
    match project.info().configurations().get(configuration_name) {
        Some(configuration) => configuration,
        None => {
            println!(
                "No configuration named \"{configuration_name}\" has been defined in project.toml."
            );

            if project.info().configurations().is_empty() {
                println!(
                    "Fix: add a configuration such as `[configuration.main]` with `sources` and, for builds, `targets`."
                );
            } else {
                println!("Valid configurations:");
                let mut configurations: Vec<&str> = project
                    .info()
                    .configurations()
                    .keys()
                    .map(String::as_str)
                    .collect();
                configurations.sort_unstable();
                for configuration in configurations {
                    println!("- {configuration}");
                }
                println!(
                    "Fix: run `wisteria switch <configuration>` to select an existing configuration, or add `[configuration.{configuration_name}]` to project.toml."
                );
            }

            exit(1)
        }
    }
}

pub(crate) struct DependencyResolutionResult {
    pub(crate) resolved: Vec<ResolvedDependency>,
    pub(crate) failed: Vec<(String, String)>,
}

pub(crate) struct CommandOutput<'a> {
    renderer: &'a mut dyn OutputRenderer,
    operation: &'a str,
}

impl<'a> CommandOutput<'a> {
    pub(crate) fn new(renderer: &'a mut dyn OutputRenderer, operation: &'a str) -> Self {
        Self {
            renderer,
            operation,
        }
    }
}

pub(crate) fn update_dependencies_with_context(
    output: CommandOutput<'_>,
    targets: &[String],
    dependencies: &HashMap<String, Dependency>,
    environment: &HashMap<String, String>,
    regexes: &HashMap<&str, Regex>,
    context: UpdateContext,
    lockfile: Option<&Lockfile>,
) -> DependencyResolutionResult {
    let mut resolved_dependencies: Vec<ResolvedDependency> = Vec::new();
    let mut failed_downloads: Vec<(String, String)> = Vec::new();
    let (action, failure_action) = match context {
        UpdateContext::ResolveOnly => ("Resolving", "resolve"),
        _ => ("Updating", "download"),
    };
    let size = targets.len();
    output.renderer.operation_started(output.operation, size);

    for (index, target) in targets.iter().enumerate() {
        let step = index + 1;
        match dependencies.get_key_value(target) {
            Some((name, dep)) => {
                output
                    .renderer
                    .step_started(output.operation, action, name, step, size);
                match dep.resolve(
                    name,
                    environment,
                    regexes,
                    ResolveContext::for_dependency(context, lockfile, name),
                ) {
                    Ok(resolved) => {
                        output.renderer.step_completed(
                            output.operation,
                            action,
                            name,
                            step,
                            size,
                            "Done",
                        );
                        resolved_dependencies.push(resolved)
                    }
                    Err(e) => {
                        output.renderer.step_failed(
                            output.operation,
                            action,
                            name,
                            step,
                            size,
                            &format!("Could not {failure_action} {name}: {e}"),
                        );
                        failed_downloads.push((name.clone(), e));
                        continue;
                    }
                };
            }
            None => {
                output
                    .renderer
                    .log(&format!("Usage of undeclared dependency \"{target}\""));
            }
        }
    }

    if failed_downloads.is_empty() {
        output.renderer.operation_completed(
            output.operation,
            &dependency_operation_summary(action, resolved_dependencies.len()),
        );
    } else {
        output.renderer.operation_completed(
            output.operation,
            "Dependency resolution finished with errors.",
        );
    }

    DependencyResolutionResult {
        resolved: resolved_dependencies,
        failed: failed_downloads,
    }
}

fn dependency_operation_summary(action: &str, count: usize) -> String {
    format!(
        "{} {count} {}",
        completed_action(action),
        dependency_label(count)
    )
}

fn completed_action(action: &str) -> &'static str {
    match action {
        "Resolving" => "Resolved",
        "Updating" => "Updated",
        _ => "Completed",
    }
}

fn dependency_label(count: usize) -> &'static str {
    match count {
        1 => "dependency",
        _ => "dependencies",
    }
}
