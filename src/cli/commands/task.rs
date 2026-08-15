use std::process::exit;

use crate::build::task::TaskOutput;
use crate::cli::args::StartupFlags;
use crate::cli::commands::{configuration_or_exit, project_or_exit};
use crate::model::{Configuration, Metadata, Project};
use crate::output;
use crate::util::exit_code;

pub fn trigger_task(project: Result<Project, String>, args: &[String], flags: &StartupFlags) {
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
    let mut renderer = output::renderer(flags.output_mode);
    let mut task_output = TaskOutput::new(renderer.as_mut(), &args[1], task.phase_order().len());

    task_output.operation_started();
    if let Err(message) = task.invoke(project.info(), &project, configuration, &mut task_output) {
        task_output.log(&format!(
            "Failed to execute task \"{}\": {message}",
            args[1]
        ));
        task_output.operation_completed("Task finished with errors.");
        exit(exit_code::take_external_process_exit_code().unwrap_or(1))
    }

    task_output.operation_completed(&task_summary(&args[1]));
}

fn task_summary(task: &str) -> String {
    match task {
        "build" => String::from("Built project"),
        "javadocs" | "javadoc" => String::from("Generated javadocs"),
        "run" => String::from("Finished run task"),
        task => format!("Completed task \"{task}\""),
    }
}
