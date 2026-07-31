use std::{fs::write, process::exit};

use crate::cli::args::StartupFlags;
use crate::cli::commands::{
    configuration_or_exit, envvar_regexes, print_header, project_or_exit,
    update_dependencies_with_context,
};
use crate::dependency::UpdateContext;
use crate::generators::generate_metadata;
use crate::model::{Configuration, Metadata, Project};
use crate::util::consts;
use crate::workspace::refresh::refresh;

pub fn trigger_switch(project: Result<Project, String>, args: &[String], flags: &StartupFlags) {
    let project: Project = project_or_exit(project);

    let mut metadata = match Metadata::load() {
        Ok(m) => m,
        Err(e) => {
            println!("{e}");
            exit(1)
        }
    };

    print_header();

    if metadata.configuration == args[2] {
        println!(
            "Project is already set to use that configuration. To reload the project configuration, use \"wisteria refresh\" instead."
        );
        exit(1)
    }

    let configuration: &Configuration = configuration_or_exit(&project, &args[2]);

    let regexes = envvar_regexes();
    if !flags.no_refresh
        && let Err((nature, error)) = refresh(&project, configuration, &regexes)
    {
        println!("Failed to refresh nature {}: {error}", nature.type_str());
        println!("Project might be in a degraded state.");
        exit(1)
    }

    let mut failed_downloads: Vec<(String, String)> = Vec::new();
    if let Some(dependencies) = configuration.dependencies() {
        failed_downloads = update_dependencies_with_context(
            dependencies,
            project.dependencies(),
            configuration.environment(),
            &regexes,
            UpdateContext::SwitchConfiguration,
        );
    }

    print!("Finishing up... ");
    metadata.configuration = args[2].clone();
    let _ = write(consts::METADATA_FILE, generate_metadata(&metadata));
    println!("Done!");

    if failed_downloads.is_empty() {
        println!(
            "Operation complete! Your project is now set up to use the configuration \"{}\".",
            args[2]
        );
        exit(0)
    }

    println!(
        "Operation complete with dependency resolution errors. Your project is now set up to use the configuration \"{}\".",
        args[2]
    );
    println!("Failed to resolve the following dependencies:");
    for (name, error) in failed_downloads {
        println!("- {name}: {error}")
    }
    exit(1)
}
