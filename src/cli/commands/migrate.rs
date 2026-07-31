use std::{path::PathBuf, process::exit};

use crate::{
    cli::{args::StartupFlags, commands::print_header},
    model::migration::migrate_wisteria2_project_file,
    util::consts,
};

pub fn trigger_migrate(args: &[String], flags: &StartupFlags) {
    let Some(target) = args.get(2) else {
        println!("Not enough arguments. Expected a migration target, such as \"wisteria2\".");
        exit(1)
    };

    match target.to_lowercase().as_str() {
        "wisteria2" | "wisteria-2" | "2" | "v2" => trigger_wisteria2_migration(flags),
        _ => {
            println!("Unknown migration target \"{target}\". Expected \"wisteria2\".");
            exit(1)
        }
    }
}

pub fn trigger_migrate2(flags: &StartupFlags) {
    trigger_wisteria2_migration(flags)
}

fn trigger_wisteria2_migration(flags: &StartupFlags) {
    print_header();

    let project_file = PathBuf::from(
        flags
            .use_project
            .clone()
            .unwrap_or_else(|| String::from(consts::PROJECT_FILE)),
    );

    match migrate_wisteria2_project_file(&project_file) {
        Ok(migration) => {
            println!(
                "Migrated {} from Wisteria 2 format.",
                project_file.to_string_lossy()
            );
            println!(
                "Backup written to {}",
                migration.backup_path.to_string_lossy()
            );
            println!(
                "Generated {} dependencies and {} configurations.",
                migration.dependency_count, migration.configuration_count
            );

            if !migration.warnings.is_empty() {
                println!("Warnings:");
                for warning in migration.warnings {
                    println!("- {warning}");
                }
            }

            exit(0)
        }
        Err(error) => {
            println!("Could not migrate Wisteria 2 project.toml: {error}");
            exit(1)
        }
    }
}
