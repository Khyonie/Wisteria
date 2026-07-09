use std::{
    fs::{create_dir, write},
    path::PathBuf,
    process::exit,
};

use crate::cli::args::StartupFlags;
use crate::cli::commands::print_header;
use crate::generators::{generate_wisteria_project, WISTERIA_METADATA_TEMPLATE};

pub fn trigger_create(args: &[String], flags: &StartupFlags) {
    if args[2].contains('/') || args[2].contains('\\') {
        println!("Invalid project name. A project name must not contain any slashes.");
        exit(1)
    }

    let path = PathBuf::from(&args[2]);
    if path.exists() {
        println!("A project by that name already exists in this directory.");
        exit(1)
    }

    if create_dir(&path).is_err() {
        println!("Could not create a new project \"{}\" in the current directory. Ensure that you have the correct permissions and try again.", args[2]);
        exit(1)
    }

    print_header();

    create_dir(path.join(".wisteria/")).unwrap();
    write(
        format!("{}/.wisteria/metadata.toml", args[2]),
        WISTERIA_METADATA_TEMPLATE,
    )
    .unwrap();
    write(
        format!("{}/project.toml", args[2]),
        generate_wisteria_project(&args[2], flags.minimal),
    )
    .unwrap();
    create_dir(path.join("src/")).unwrap();
    create_dir(path.join("lib/")).unwrap();

    println!("Operation complete! You should now open {}/project.toml in your favorite text editor to tweak the project to suit your needs.", args[2]);
    exit(0)
}
