use std::process::exit;

use crate::cli::commands::{configuration_or_exit, project_or_exit};
use crate::model::{Configuration, Metadata, Project};

pub fn trigger_task(project: Result<Project, (String, u8)>, args: &[String]) {
    let project: Project = project_or_exit(project);

    let metadata = match Metadata::load() {
        Ok(m) => m,
        Err((e, code)) => {
            println!("{e}");
            exit(code as i32)
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

    if let Err((message, code)) = task.invoke(project.info(), &project, configuration) {
        println!("Failed to execute task (TODO Chain over to fail if defined): {message}");
        exit(code as i32)
    }

    println!("Operation complete!");
}
