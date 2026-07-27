use std::{
    fs::{create_dir, write},
    path::PathBuf,
    process::exit,
};

use crate::cli::args::StartupFlags;
use crate::cli::commands::print_header;
use crate::generators::{generate_wisteria_project, WISTERIA_METADATA_TEMPLATE};
use crate::util::consts;

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

    create_dir(path.join(consts::WISTERIA_DIR)).unwrap();
    write(
        path.join(consts::METADATA_FILE),
        WISTERIA_METADATA_TEMPLATE,
    )
    .unwrap();
    write(
        path.join(consts::PROJECT_FILE),
        generate_wisteria_project(&args[2], flags.minimal),
    )
    .unwrap();
    create_dir(path.join(consts::PROJECT_SOURCE_DIR)).unwrap();
    create_dir(path.join(consts::PROJECT_LIBRARY_DIR)).unwrap();

    println!(
        "Operation complete! You should now open {}/{} in your favorite text editor to tweak the project to suit your needs.",
        args[2],
        consts::PROJECT_FILE
    );
    exit(0)
}
