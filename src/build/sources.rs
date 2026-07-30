use std::{
    fs::{self, File},
    path::PathBuf,
};

use crate::model::Configuration;
use crate::util::consts;
use crate::workspace::files;

pub fn collect_sources(configuration: &Configuration) -> Result<Vec<String>, (String, u8)> {
    match configuration.sources() {
        Some(sources) => {
            if sources.is_empty() {
                return Err((
                    String::from("No source folders given, nothing to compile"),
                    1,
                ));
            }

            let mut copied_files: Vec<String> = Vec::new();

            let _ = fs::remove_dir_all(consts::SOURCE_OUT_PATH);

            for source in sources {
                let files = files::collect_files_with_extension(&PathBuf::from(source), "java");
                if files.is_empty() {
                    continue;
                }

                for f in &files {
                    let relative_path = f.to_string_lossy().replacen(source, "", 1);
                    let relative_path = relative_path.trim_start_matches(['/', '\\']);
                    let copy_path = format!("{}/{}", consts::SOURCE_OUT_PATH, relative_path);
                    let mut path = PathBuf::from(&copy_path);
                    path.pop();
                    fs::create_dir_all(path)
                        .map_err(|e| (format!("Failed to create directory: {e}"), 1))?;
                    File::create(&copy_path)
                        .map_err(|e| (format!("Failed to create relative path: {e}"), 1))?;

                    match fs::copy(f, &copy_path) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("{e}");
                            continue;
                        }
                    }

                    copied_files.push(copy_path);
                }
            }

            Ok(copied_files)
        }
        None => Err((
            String::from("No source folders given, nothing to compile"),
            1,
        )),
    }
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

        assert_eq!(error.0, "No source folders given, nothing to compile");
        assert_eq!(error.1, 1);
    }
}
