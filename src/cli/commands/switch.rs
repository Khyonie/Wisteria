use std::{fs::write, process::exit};

use crate::cli::args::StartupFlags;
use crate::cli::commands::{
    CommandOutput, configuration_or_exit, envvar_regexes, project_or_exit,
    update_dependencies_with_context,
};
use crate::dependency::UpdateContext;
use crate::generators::generate_metadata;
use crate::model::lockfile::try_read_lockfile;
use crate::model::{Configuration, Metadata, Project};
use crate::output;
use crate::util::consts;
use crate::workspace::refresh::refresh;

pub fn trigger_switch(project: Result<Project, String>, args: &[String], flags: &StartupFlags) {
    let project: Project = project_or_exit(project);
    let mut output = output::renderer(flags.output_mode);

    let mut metadata = match Metadata::load() {
        Ok(m) => m,
        Err(e) => {
            output.log(&e);
            exit(1)
        }
    };

    if metadata.configuration == args[2] {
        output.log(
            "Project is already set to use that configuration. To reload the project configuration, use \"wisteria refresh\" instead."
        );
        exit(1)
    }

    let configuration: &Configuration = configuration_or_exit(&project, &args[2]);

    let regexes = envvar_regexes();
    if !flags.no_refresh
        && let Err((nature, error)) = refresh(&project, configuration, &regexes, output.as_mut())
    {
        output.log(&format!(
            "Failed to refresh nature {}: {error}",
            nature.type_str()
        ));
        output.log("Project might be in a degraded state.");
        exit(1)
    }

    let mut failed_downloads: Vec<(String, String)> = Vec::new();
    if let Some(dependencies) = configuration.dependencies() {
        let lockfile = match try_read_lockfile() {
            Ok(lockfile) => lockfile,
            Err(error) => {
                output.log(&error);
                exit(1)
            }
        };
        let dependency_names: Vec<String> = dependencies
            .iter()
            .map(|reference| reference.name().to_string())
            .collect();
        let result = update_dependencies_with_context(
            CommandOutput::new(output.as_mut(), "switch"),
            &dependency_names,
            project.dependencies(),
            configuration.environment(),
            &regexes,
            UpdateContext::SwitchConfiguration,
            lockfile.as_ref(),
        );
        failed_downloads = result.failed;
    }

    output.operation_started("switch", 1);
    output.step_started("switch", "Writing", "metadata", 1, 1);
    metadata.configuration = args[2].clone();
    if let Err(error) = write(consts::METADATA_FILE, generate_metadata(&metadata)) {
        let message = format!("Could not write {}: {error}", consts::METADATA_FILE);
        output.step_failed("switch", "Writing", "metadata", 1, 1, &message);
        output.operation_completed("switch", "Switch finished with errors.");
        exit(1);
    }
    output.step_completed("switch", "Writing", "metadata", 1, 1, "Done");

    if failed_downloads.is_empty() {
        output.operation_completed(
            "switch",
            &format!("Switched to configuration \"{}\"", args[2]),
        );
        exit(0)
    }

    output.operation_completed(
        "switch",
        &format!(
            "Switched to configuration \"{}\" with dependency resolution errors",
            args[2]
        ),
    );
    output.log("Failed to resolve the following dependencies:");
    for (name, error) in failed_downloads {
        output.log(&format!("- {name}: {error}"))
    }
    exit(1)
}
