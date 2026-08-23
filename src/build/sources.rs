use std::{
    fs::{self, File},
    path::PathBuf,
};

use crate::model::Configuration;
use crate::util::consts;
use crate::workspace::files;

pub fn collect_sources(configuration: &Configuration) -> Result<Vec<String>, String> {
    match configuration.sources() {
        Some(sources) => collect_source_folders(sources, consts::SOURCE_OUT_PATH),
        None => Err(String::from("No source folders given, nothing to compile")),
    }
}

pub fn collect_source_folders(
    sources: &[String],
    output_path: &str,
) -> Result<Vec<String>, String> {
    if sources.is_empty() {
        return Err(String::from("No source folders given, nothing to compile"));
    }

    let mut copied_files: Vec<String> = Vec::new();

    let _ = fs::remove_dir_all(output_path);

    for source in sources {
        let files = files::collect_files_with_extension(&PathBuf::from(source), "java")
            .map_err(|e| format!("Could not collect source files from \"{source}\": {e}"))?;
        if files.is_empty() {
            continue;
        }

        for f in &files {
            let relative_path = f.to_string_lossy().replacen(source, "", 1);
            let relative_path = relative_path.trim_start_matches(['/', '\\']);
            let copy_path = format!("{output_path}/{relative_path}");
            let mut path = PathBuf::from(&copy_path);
            path.pop();
            fs::create_dir_all(path).map_err(|e| format!("Failed to create directory: {e}"))?;
            File::create(&copy_path).map_err(|e| format!("Failed to create relative path: {e}"))?;

            fs::copy(f, &copy_path).map_err(|e| {
                format!(
                    "Failed to copy source file \"{}\" to \"{copy_path}\": {e}",
                    f.display()
                )
            })?;

            copied_files.push(copy_path);
        }
    }

    Ok(copied_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TempDir, with_current_dir};
    use std::fs;
    use toml::Table;

    fn configuration(toml: &str) -> Configuration {
        Configuration::from(
            String::from("main"),
            &toml.parse::<Table>().unwrap(),
            String::from("Demo"),
            String::from("1.0.0"),
        )
        .unwrap()
    }

    #[test]
    fn collect_sources_copies_java_files_to_work_directory() {
        let temp = TempDir::new("collect-sources");
        let source = temp.path().join("src");
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::write(source.join("Main.java"), "class Main {}").unwrap();
        fs::write(source.join("nested/Other.java"), "class Other {}").unwrap();
        fs::write(source.join("notes.txt"), "ignore me").unwrap();

        with_current_dir(temp.path(), || {
            let configuration = configuration(&format!(
                r#"
                sources = [ "{}" ]
                "#,
                source.to_string_lossy()
            ));

            let mut copied = collect_sources(&configuration).unwrap();
            copied.sort();

            assert_eq!(copied.len(), 2);
            assert!(temp.path().join(".wisteria/work/src/Main.java").exists());
            assert!(
                temp.path()
                    .join(".wisteria/work/src/nested/Other.java")
                    .exists()
            );
            assert!(!temp.path().join(".wisteria/work/src/notes.txt").exists());
        });
    }

    #[test]
    fn collect_sources_rejects_empty_source_list() {
        let configuration = configuration("sources = [ ]");

        let error = collect_sources(&configuration).unwrap_err();

        assert_eq!(error, "No source folders given, nothing to compile");
    }

    #[test]
    fn collect_source_folders_can_use_custom_output_directory() {
        let temp = TempDir::new("collect-custom-sources");
        let source = temp.path().join("tests");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("ExampleTest.java"), "class ExampleTest {}").unwrap();

        with_current_dir(temp.path(), || {
            let copied = collect_source_folders(
                &[source.to_string_lossy().to_string()],
                consts::TEST_SOURCE_OUT_PATH,
            )
            .unwrap();

            assert_eq!(copied.len(), 1);
            assert!(
                temp.path()
                    .join(".wisteria/work/test-src/ExampleTest.java")
                    .exists()
            );
        });
    }

    #[test]
    fn collect_sources_errors_when_source_path_cannot_be_traversed() {
        let temp = TempDir::new("collect-sources-unreadable");
        let source = temp.path().join("src");
        fs::write(&source, "").unwrap();

        with_current_dir(temp.path(), || {
            let configuration = configuration(&format!(
                r#"
                sources = [ "{}" ]
                "#,
                source.to_string_lossy()
            ));

            let error = collect_sources(&configuration).unwrap_err();

            assert!(error.contains("Could not collect source files"));
            assert!(error.contains(&source.display().to_string()));
        });
    }
}
