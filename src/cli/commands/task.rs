use std::process::exit;

use crate::model::{Configuration, Metadata, Project};

pub fn trigger_task(project: Result<Project, (String, u8)>, args: &[String]) {
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
    let configuration: &Configuration = project
        .info()
        .configurations()
        .get(&metadata.configuration)
        .unwrap();

    let task = match configuration.tasks().get(&args[1]) {
        Some(t) => t,
        None => {
            println!("No such task \"{}\" for configuration.", args[1]);
            exit(1)
        }
    };

    if let Err((message, code)) = task.invoke(project.info(), &project, configuration) {
        println!("Failed to execute task (TODO Chain over to fail if defined): {message}");
        exit(code as i32)
    }

    println!("Operation complete!");
}
