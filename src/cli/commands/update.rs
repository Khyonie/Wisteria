use std::process::exit;

use crate::cli::commands::{envvar_regexes, update_dependencies_with_context};
use crate::dependency::UpdateContext;
use crate::model::{Metadata, Project};

pub fn trigger_update(project: Result<Project, (String, u8)>, args: &[String]) {
    let project: Project = match project {
        Ok(p) => p,
        Err(e) => {
            println!(
                "Could not read a Wisteria project.toml file in this directory. ({})",
                e.0
            );
            exit(e.1.into())
        }
    };

    let metadata = match Metadata::load() {
        Ok(m) => m,
        Err((e, code)) => {
            println!("{e}");
            exit(code as i32)
        }
    };

    let configuration = project
        .info()
        .configurations()
        .get(&metadata.configuration)
        .unwrap();
    let regexes = envvar_regexes();

    if args[2] == "all" {
        let keys: Vec<String> = project.dependencies().keys().cloned().collect();

        let failed = update_dependencies_with_context(
            &keys,
            project.dependencies(),
            configuration.environment(),
            &regexes,
            UpdateContext::Update,
        );

        if !failed.is_empty() {
            println!("Failed to resolve one or more dependencies:");
            for (name, reason) in &failed {
                println!("\t{name}: {reason}");
            }

            exit(1)
        }

        println!("Operation complete!");

        exit(0)
    }

    let mut target_dependencies: Vec<String> = Vec::new();
    for a in args[2..].iter() {
        if !project.dependencies().contains_key(a) {
            println!("No such dependency \"{}\" has been defined.", a);
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

    if !failed.is_empty() {
        println!("Failed to resolve one or more dependencies:");
        for (name, reason) in &failed {
            println!("\t{name}: {reason}");
        }

        exit(1)
    }

    println!("Operation complete!");
}
