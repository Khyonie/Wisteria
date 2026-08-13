use std::{collections::HashSet, process::exit};

use crate::dependency::resolver::ResolvedDependency;
use crate::model::lockfile::{
    lockable_artifacts, lockable_artifacts_to_toml, lockfile_artifacts_to_toml, try_read_lockfile,
    write_lockfile,
};
use crate::model::{Lockfile, LockfileArtifact, Project};

pub(crate) struct DependencySelection {
    names: Vec<String>,
    all_dependencies: bool,
}

impl DependencySelection {
    pub(crate) fn names(&self) -> &[String] {
        &self.names
    }

    pub(crate) fn all_dependencies(&self) -> bool {
        self.all_dependencies
    }
}

pub(crate) fn dependency_selection_or_exit(
    project: &Project,
    args: &[String],
    command: &str,
    default_to_all: bool,
) -> DependencySelection {
    if args.len() == 2 {
        if default_to_all {
            return DependencySelection {
                names: all_dependency_names(project),
                all_dependencies: true,
            };
        }

        println!(
            "Not enough arguments. Expected one or more dependency names, or `all`.\nFix: run `wisteria {command} all`, or name dependencies explicitly."
        );
        exit(1)
    }

    if args[2] == "all" {
        if args.len() > 3 {
            println!(
                "Invalid arguments: `all` cannot be combined with explicit dependency names.\nFix: run either `wisteria {command} all` or `wisteria {command} {}`.",
                args[3..].join(" ")
            );
            exit(1)
        }

        return DependencySelection {
            names: all_dependency_names(project),
            all_dependencies: true,
        };
    }

    DependencySelection {
        names: dependency_names_or_exit(project, &args[2..], command),
        all_dependencies: false,
    }
}

pub(crate) fn read_lockfile_or_exit() -> Option<Lockfile> {
    match try_read_lockfile() {
        Ok(lockfile) => lockfile,
        Err(error) => {
            println!("{error}");
            exit(1)
        }
    }
}

pub(crate) fn require_lockfile_or_exit() -> Lockfile {
    match read_lockfile_or_exit() {
        Some(lockfile) => lockfile,
        None => {
            println!(
                "No wisteria.lock file exists.\nFix: run `wisteria update all` to resolve dependencies and create one, or restore it from source control."
            );
            exit(1)
        }
    }
}

pub(crate) fn write_full_lockfile_or_exit(resolved_dependencies: &[ResolvedDependency]) {
    let toml = match lockable_artifacts_to_toml(resolved_dependencies) {
        Ok(toml) => toml,
        Err(error) => {
            println!("{error}");
            exit(1)
        }
    };

    write_lockfile_or_exit(&toml);
}

pub(crate) fn write_full_lockfile_artifacts_or_exit(artifacts: Vec<LockfileArtifact>) {
    let toml = match lockfile_artifacts_to_toml(artifacts) {
        Ok(toml) => toml,
        Err(error) => {
            println!("{error}");
            exit(1)
        }
    };

    write_lockfile_or_exit(&toml);
}

pub(crate) fn write_partial_lockfile_or_exit(
    existing_lockfile: Option<&Lockfile>,
    resolved_dependencies: &[ResolvedDependency],
    target_dependencies: &[String],
) {
    let target_names: HashSet<&str> = target_dependencies.iter().map(String::as_str).collect();
    let mut artifacts: Vec<LockfileArtifact> = existing_lockfile
        .map(|lockfile| {
            lockfile
                .artifacts()
                .iter()
                .filter(|artifact| !target_names.contains(artifact.name()))
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    artifacts.extend(lockable_artifacts(resolved_dependencies));

    let toml = match lockfile_artifacts_to_toml(artifacts) {
        Ok(toml) => toml,
        Err(error) => {
            println!("{error}");
            exit(1)
        }
    };

    write_lockfile_or_exit(&toml);
}

pub(crate) fn write_partial_lockfile_artifacts_or_exit(
    existing_lockfile: Option<&Lockfile>,
    mut resolved_artifacts: Vec<LockfileArtifact>,
    target_dependencies: &[String],
) {
    let target_names: HashSet<&str> = target_dependencies.iter().map(String::as_str).collect();
    let mut artifacts: Vec<LockfileArtifact> = existing_lockfile
        .map(|lockfile| {
            lockfile
                .artifacts()
                .iter()
                .filter(|artifact| !target_names.contains(artifact.name()))
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    artifacts.append(&mut resolved_artifacts);

    let toml = match lockfile_artifacts_to_toml(artifacts) {
        Ok(toml) => toml,
        Err(error) => {
            println!("{error}");
            exit(1)
        }
    };

    write_lockfile_or_exit(&toml);
}

fn all_dependency_names(project: &Project) -> Vec<String> {
    let mut names: Vec<String> = project.dependencies().keys().cloned().collect();
    names.sort_unstable();
    names
}

fn dependency_names_or_exit(project: &Project, names: &[String], command: &str) -> Vec<String> {
    let mut selected = Vec::new();

    for name in names {
        if !project.dependencies().contains_key(name) {
            println!("No such dependency \"{name}\" has been defined.");
            if project.dependencies().is_empty() {
                println!(
                    "Fix: add dependencies under a table such as `[dependencies.maven]`, or run `wisteria {command} all` only after dependencies are configured."
                );
            } else {
                println!("Valid dependencies:");
                let mut dependencies: Vec<&str> =
                    project.dependencies().keys().map(String::as_str).collect();
                dependencies.sort_unstable();
                for dependency in dependencies {
                    println!("- {dependency}");
                }
                println!(
                    "Fix: use one of the dependency names above, or add `{name}` to project.toml."
                );
            }
            exit(1)
        }

        selected.push(name.clone());
    }

    selected
}

fn write_lockfile_or_exit(toml: &str) {
    if let Err(error) = write_lockfile(toml) {
        println!("{error}");
        exit(1)
    }
}
