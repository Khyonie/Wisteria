use std::{fs::write, process::exit};

use crate::cli::args::StartupFlags;
use crate::cli::commands::{envvar_regexes, print_header, update_dependencies_with_context};
use crate::dependency::UpdateContext;
use crate::generators::generate_metadata;
use crate::model::{Configuration, Metadata, Project};
use crate::util::consts;
use crate::workspace::nature::Nature;

pub fn trigger_switch(
    project: Result<Project, (String, u8)>,
    args: &[String],
    flags: &StartupFlags,
) {
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

    let mut metadata = match Metadata::load() {
        Ok(m) => m,
        Err((e, code)) => {
            println!("{e}");
            exit(code as i32)
        }
    };

    print_header();

    if metadata.configuration == args[2] {
        println!("Project is already set to use that configuration. To reload the project configuration, use \"wisteria refresh\" instead.");
        exit(1)
    }

    let configuration: &Configuration = match project.info().configurations().get(&args[2]) {
        Some(c) => c,
        None => {
            println!("No such configuration \"{}\".", args[2]);
            exit(1)
        }
    };

    if !flags.no_refresh {
        // TODO Move refresh logic into here
    }
    let regexes = envvar_regexes();

    print!("Removing natures... ");
    for nature in Nature::values() {
        let _ = nature.remove_nature();
    }
    println!("Done!");

    for nature in project.info().natures() {
        print!("Applying project nature \"{}\"... ", nature.type_str());
        nature.setup_nature(&project, configuration, &regexes);
        println!("Done!");
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

    println!("Operation complete with dependency resolution errors. Your project is now set up to use the configuration \"{}\".", args[2]);
    println!("Failed to resolve the following dependencies:");
    for (name, error) in failed_downloads {
        println!("- {name}: {error}")
    }
    exit(1)
}
