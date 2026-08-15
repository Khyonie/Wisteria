use std::process::exit;

use crate::cli::args::StartupFlags;
use crate::cli::commands::dependencies::{
    dependency_selection_or_exit, read_lockfile_or_exit, write_full_lockfile_or_exit,
    write_partial_lockfile_or_exit,
};
use crate::cli::commands::{
    CommandOutput, configuration_or_exit, envvar_regexes, project_or_exit,
    update_dependencies_with_context,
};
use crate::dependency::UpdateContext;
use crate::model::{Metadata, Project};
use crate::output;
use crate::workspace::refresh::refresh;

pub fn trigger_update(project: Result<Project, String>, args: &[String], flags: &StartupFlags) {
    let project: Project = project_or_exit(project);

    let metadata = match Metadata::load() {
        Ok(m) => m,
        Err(e) => {
            println!("{e}");
            exit(1)
        }
    };

    let configuration = configuration_or_exit(&project, &metadata.configuration);
    let regexes = envvar_regexes();
    let lockfile = read_lockfile_or_exit();
    let mut refresh_failed = false;
    let selection = dependency_selection_or_exit(&project, args, "update", false);
    let mut output = output::renderer(flags.output_mode);

    let result = update_dependencies_with_context(
        CommandOutput::new(output.as_mut(), "update"),
        selection.names(),
        project.dependencies(),
        configuration.environment(),
        &regexes,
        UpdateContext::Update,
        lockfile.as_ref(),
    );

    if !result.failed.is_empty() {
        output.log("Failed to resolve one or more dependencies:");
        for (name, reason) in &result.failed {
            output.log(&format!("\t{name}: {reason}"));
        }

        exit(1)
    }

    if selection.all_dependencies() {
        write_full_lockfile_or_exit(&result.resolved);
    } else {
        write_partial_lockfile_or_exit(lockfile.as_ref(), &result.resolved, selection.names());
    }

    if !flags.no_refresh
        && let Err((nature, error)) = refresh(&project, configuration, &regexes, output.as_mut())
    {
        output.log(&format!(
            "Failed to refresh nature {}: {error}",
            nature.type_str()
        ));
        refresh_failed = true
    }

    if refresh_failed {
        if selection.all_dependencies() {
            output.log("Dependencies updated, however project might be in a degraded state.");
        } else {
            output.log("Dependency updated, however project might be in a degraded state.");
        }
        exit(1)
    }
}
