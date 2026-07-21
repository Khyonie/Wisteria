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

    let configuration = project
        .info()
        .configurations()
        .get(&metadata.configuration)
        .unwrap();
    let regexes = envvar_regexes();

    refresh(&project, &configuration, &regexes);

    println!("Operation complete!");
}
