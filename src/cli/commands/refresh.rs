use std::process::exit;

use crate::cli::commands::{envvar_regexes, print_header};
use crate::model::{Metadata, Project};
use crate::workspace::refresh::refresh;

/// `wisteria refresh`
pub fn trigger_refresh(project: Result<Project, (String, u8)>) {
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

    print_header();
    println!(
        "Refreshing project \"{}\" with configuration \"{}\"...",
        project.info().name(),
        &metadata.configuration
    );

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

    if let Err((nature, error)) = refresh(&project, configuration, &regexes)
    {
        println!("Failed to refresh nature {}: {error}", nature.type_str());
        println!("Project might be in a degraded state.");
        exit(1)
    }

    println!("Operation complete!");
}
