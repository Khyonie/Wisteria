use std::process::exit;

use crate::model::Project;

pub mod args;
pub mod commands;

pub fn run() {
    let mut args: Vec<String> = Vec::new();
    let flags = args::load_arguments(&mut args);
    let command = args[1].to_lowercase();

    match command.as_str() {
        "migrate" => commands::migrate::trigger_migrate(&args, &flags),
        "migrate2" => commands::migrate::trigger_migrate2(&flags),
        _ => {}
    }

    let project: Result<Project, String> =
        Project::from_with_flags(flags.use_project.clone(), flags.clone());

    match command.as_str() {
        "refresh" => commands::refresh::trigger_refresh(project),
        "sync" => commands::sync::trigger_sync(project, &args),
        "fetch" => commands::fetch::trigger_fetch(project, &args),
        "verify" => commands::verify::trigger_verify(project, &args),
        "update" if args.len() == 2 => {
            println!(
                "Not enough arguments. Expected at least one argument, but none were supplied."
            );
            exit(1)
        }
        "update" => commands::update::trigger_update(project, &args, &flags),
        "clean" if args.len() == 2 => {
            println!(
                "Not enough arguments. Expected one of [ classes, dependencies, targets, javadocs, metadata, natures, all ], but nothing was supplied."
            );
            exit(1)
        }
        "clean" => commands::clean::trigger_clean(project, &args),
        "new" | "create" if args.len() == 2 => exit(1),
        "new" | "create" => commands::create::trigger_create(&args, &flags),
        "info" => commands::info::trigger_info(project),
        "switch" if args.len() == 2 => {
            println!("Not enough arguments. Expected a configuration name.");
            exit(1)
        }
        "switch" => commands::switch::trigger_switch(project, &args, &flags),
        _ => commands::task::trigger_task(project, &args),
    }
}
