use std::process::exit;

use crate::cli::args::StartupFlags;
use crate::cli::commands::{envvar_regexes, update_dependencies_with_context};
use crate::dependency::UpdateContext;
use crate::model::{Metadata, Project};
use crate::workspace::refresh::refresh;

pub fn trigger_update(project: Result<Project, (String, u8)>, args: &[String], flags: &StartupFlags) {
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

    let configuration = match project
        .info()
        .configurations()
        .get(&metadata.configuration) 
    {
        Some(c) => c,
        None => {
            println!("No configuration named \"{}\" has been defined in project.toml, consider using 'wisteria switch <configuration>' to  move to a valid configuration", &metadata.configuration);
            println!("Valid configurations:");
            for s in project.info().configurations().keys()
            {
                println!("- {s}")
            }
            exit(1)
        },
    };
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
            if let Err((nature, error)) = refresh(&project, configuration, &regexes)
            {
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

        if refresh_failed
        {
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
        if let Err((nature, error)) = refresh(&project, configuration, &regexes)
        {
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
    if refresh_failed
    {
        println!("Dependency updated, however project might be in a degraded state.");
        exit(1)
    }

    println!("Operation complete!");
}
