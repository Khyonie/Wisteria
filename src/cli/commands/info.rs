use std::process::exit;

use crate::model::Project;

pub fn trigger_info(project: Result<Project, (String, u8)>) {
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
    project.print_info();
    exit(0);
}
