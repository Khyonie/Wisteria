use std::{
    fs::{create_dir, remove_dir_all, write},
    path::{Path, PathBuf},
    process::exit,
};

use crate::cli::args::StartupFlags;
use crate::cli::commands::print_header;
use crate::generators::{WISTERIA_METADATA_TEMPLATE, generate_wisteria_project};
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

    print_header();

    if let Err(error) = create_project(&path, &args[2], flags.minimal) {
        println!("{error}");
        exit(1)
    }

    println!(
        "Operation complete! You should now open {}/{} in your favorite text editor to tweak the project to suit your needs.",
        args[2],
        consts::PROJECT_FILE
    );
    exit(0)
}

fn create_project(path: &Path, project_name: &str, minimal: bool) -> Result<(), String> {
    create_dir(path).map_err(|e| {
        format!(
            "Could not create a new project \"{project_name}\" in the current directory: {e}.\nFix: ensure that you have the correct permissions and try again."
        )
    })?;

    if let Err(error) = write_project_files(path, project_name, minimal) {
        return Err(cleanup_partial_project(path, error));
    }

    Ok(())
}

fn write_project_files(path: &Path, project_name: &str, minimal: bool) -> Result<(), String> {
    create_dir(path.join(consts::WISTERIA_DIR)).map_err(|e| {
        format!(
            "Could not create Wisteria metadata directory \"{}\": {e}",
            path.join(consts::WISTERIA_DIR).display()
        )
    })?;
    write(path.join(consts::METADATA_FILE), WISTERIA_METADATA_TEMPLATE).map_err(|e| {
        format!(
            "Could not write Wisteria metadata file \"{}\": {e}",
            path.join(consts::METADATA_FILE).display()
        )
    })?;
    write(
        path.join(consts::PROJECT_FILE),
        generate_wisteria_project(project_name, minimal),
    )
    .map_err(|e| {
        format!(
            "Could not write project configuration \"{}\": {e}",
            path.join(consts::PROJECT_FILE).display()
        )
    })?;
    create_dir(path.join(consts::PROJECT_SOURCE_DIR)).map_err(|e| {
        format!(
            "Could not create source directory \"{}\": {e}",
            path.join(consts::PROJECT_SOURCE_DIR).display()
        )
    })?;

    Ok(())
}

fn cleanup_partial_project(path: &Path, creation_error: String) -> String {
    match remove_dir_all(path) {
        Ok(()) => format!(
            "{creation_error}\nCleaned up partial project directory \"{}\".",
            path.display()
        ),
        Err(cleanup_error) => format!(
            "{creation_error}\nCould not clean up partial project directory \"{}\": {cleanup_error}.\nFix: remove that directory manually before trying again.",
            path.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;
    use std::fs;

    #[test]
    fn create_project_writes_expected_project_layout() {
        let temp = TempDir::new("create-project");
        let project_path = temp.path().join("Demo");

        create_project(&project_path, "Demo", false).unwrap();

        assert!(project_path.join(consts::WISTERIA_DIR).is_dir());
        assert!(project_path.join(consts::METADATA_FILE).is_file());
        assert!(project_path.join(consts::PROJECT_FILE).is_file());
        assert!(project_path.join(consts::PROJECT_SOURCE_DIR).is_dir());
        assert!(
            !project_path
                .join(consts::LEGACY_PROJECT_LIBRARY_DIR)
                .exists()
        );
    }

    #[test]
    fn cleanup_partial_project_removes_created_directory() {
        let temp = TempDir::new("create-cleanup");
        let project_path = temp.path().join("Demo");
        fs::create_dir_all(project_path.join("nested")).unwrap();
        fs::write(project_path.join("nested/file.txt"), "").unwrap();

        let error = cleanup_partial_project(&project_path, String::from("write failed"));

        assert!(error.contains("write failed"));
        assert!(error.contains("Cleaned up partial project directory"));
        assert!(!project_path.exists());
    }
}
