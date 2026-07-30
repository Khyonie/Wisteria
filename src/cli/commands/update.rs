use std::process::exit;

use crate::cli::args::StartupFlags;
use crate::cli::commands::{
    configuration_or_exit, envvar_regexes, project_or_exit, update_dependencies_with_context,
};
use crate::dependency::UpdateContext;
use crate::model::{Metadata, Project};
use crate::workspace::refresh::refresh;

pub fn trigger_update(
    project: Result<Project, (String, u8)>,
    args: &[String],
    flags: &StartupFlags,
) {
    let project: Project = project_or_exit(project);

    let metadata = match Metadata::load() {
        Ok(m) => m,
        Err((e, code)) => {
            println!("{e}");
            exit(code as i32)
        }
    };

    let configuration = configuration_or_exit(&project, &metadata.configuration);
    let regexes = envvar_regexes();
    let mut refresh_failed = false;

    if args[2] == "all" {
        let keys: Vec<String> = project.dependencies().keys().cloned().collect();

        let failed = update_dependencies_with_context(
            &keys,
            project.dependencies(),
            configuration.environment(),
            &regexes,
            UpdateContext::Update,
        );

        if !flags.no_refresh {
            if let Err((nature, error)) = refresh(&project, configuration, &regexes) {
                println!("Failed to refresh nature {}: {error}", nature.type_str());
                refresh_failed = true
            }
        }

        if !failed.is_empty() {
            println!("Failed to resolve one or more dependencies:");
            for (name, reason) in &failed {
                println!("\t{name}: {reason}");
            }

            exit(1)
        }

        if refresh_failed {
            println!("Dependencies updated, however project might be in a degraded state.");
            exit(1)
        }

        println!("Operation complete!");

        exit(0)
    }

    let mut target_dependencies: Vec<String> = Vec::new();
    for a in args[2..].iter() {
        if !project.dependencies().contains_key(a) {
            println!("No such dependency \"{}\" has been defined.", a);
            if project.dependencies().is_empty() {
                println!(
                    "Fix: add dependencies under a table such as `[dependencies.maven]`, or run `wisteria update all` only after dependencies are configured."
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
                    "Fix: use one of the dependency names above, or add `{a}` to project.toml."
                );
            }
            exit(1)
        }

        target_dependencies.push(a.clone());
    }

    let failed = update_dependencies_with_context(
        &target_dependencies,
        project.dependencies(),
        configuration.environment(),
        &regexes,
        UpdateContext::Update,
    );

    if !flags.no_refresh {
        if let Err((nature, error)) = refresh(&project, configuration, &regexes) {
            println!("Failed to refresh nature {}: {error}", nature.type_str());
            refresh_failed = true
        }
    }

    if !failed.is_empty() {
        println!("Failed to resolve one or more dependencies:");
        for (name, reason) in &failed {
            println!("\t{name}: {reason}");
        }

        exit(1)
    }
    if refresh_failed {
        println!("Dependency updated, however project might be in a degraded state.");
        exit(1)
    }

    println!("Operation complete!");
}
