use std::process::exit;

use crate::cli::commands::{configuration_or_exit, project_or_exit};
use crate::model::{Configuration, Metadata, Project};
use crate::util::exit_code;

pub fn trigger_task(project: Result<Project, String>, args: &[String]) {
    let project: Project = project_or_exit(project);

    let metadata = match Metadata::load() {
        Ok(m) => m,
        Err(e) => {
            println!("{e}");
            exit(1)
        }
    };
    let configuration: &Configuration = configuration_or_exit(&project, &metadata.configuration);

    let task = match configuration.tasks().get(&args[1]) {
        Some(t) => t,
        None => {
            println!(
                "No task named \"{}\" exists for configuration \"{}\".",
                args[1], metadata.configuration
            );
            println!(
                "Fix: run `wisteria info` to see available tasks, or define `[configuration.{}.task.{}]` in project.toml.",
                metadata.configuration, args[1]
            );
            exit(1)
        }
    };

    exit_code::clear_external_process_exit_code();
    if let Err(message) = task.invoke(project.info(), &project, configuration) {
        println!("Failed to execute task (TODO Chain over to fail if defined): {message}");
        exit(exit_code::take_external_process_exit_code().unwrap_or(1))
    }

    println!("Operation complete!");
}
