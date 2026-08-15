use std::process::exit;

use crate::cli::args::StartupFlags;
use crate::cli::commands::{configuration_or_exit, envvar_regexes, project_or_exit};
use crate::model::{Metadata, Project};
use crate::output;
use crate::workspace::refresh::refresh;

/// `wisteria refresh`
pub fn trigger_refresh(project: Result<Project, String>, flags: &StartupFlags) {
    let project: Project = project_or_exit(project);
    let mut output = output::renderer(flags.output_mode);

    let metadata = match Metadata::load() {
        Ok(m) => m,
        Err(e) => {
            output.log(&e);
            exit(1)
        }
    };

    output.log(&format!(
        "Refreshing project \"{}\" with configuration \"{}\".",
        project.info().name(),
        &metadata.configuration
    ));

    let configuration = configuration_or_exit(&project, &metadata.configuration);
    let regexes = envvar_regexes();

    if let Err((nature, error)) = refresh(&project, configuration, &regexes, output.as_mut()) {
        output.log(&format!(
            "Failed to refresh nature {}: {error}",
            nature.type_str()
        ));
        output.log("Project might be in a degraded state.");
        exit(1)
    }
}
