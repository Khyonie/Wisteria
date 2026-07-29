use std::process::exit;

use crate::cli::commands::{configuration_or_exit, envvar_regexes, print_header, project_or_exit};
use crate::model::{Metadata, Project};
use crate::workspace::refresh::refresh;

/// `wisteria refresh`
pub fn trigger_refresh(project: Result<Project, (String, u8)>) {
    let project: Project = project_or_exit(project);

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

    let configuration = configuration_or_exit(&project, &metadata.configuration);
    let regexes = envvar_regexes();

    if let Err((nature, error)) = refresh(&project, configuration, &regexes) {
        println!("Failed to refresh nature {}: {error}", nature.type_str());
        println!("Project might be in a degraded state.");
        exit(1)
    }

    println!("Operation complete!");
}
