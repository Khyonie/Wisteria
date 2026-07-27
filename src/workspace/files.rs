use std::path::{Path, PathBuf};

pub fn collect_files_with_extension(path: &Path, extension: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();

    collect_files_recursive(path, extension, &mut files);

    files
}

fn collect_files_recursive(path: &Path, extension: &str, files: &mut Vec<PathBuf>) {
    if !path.exists() {
        return;
    }

    let read = match path.read_dir() {
        Ok(r) => r,
        Err(e) => {
            println!("Could not read source \"{}\": {e}", path.to_string_lossy());
            return;
        }
    };

    for dir in read {
        let entry = match dir {
            Ok(e) => e,
            Err(e) => {
                println!("{e}");
                continue;
            }
        };
        let new_path = entry.path();

        if new_path.is_dir() {
            collect_files_recursive(&new_path, extension, files);
            continue;
        }

        if let Some(ext) = new_path.extension() {
            if ext == extension {
                files.push(new_path)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;
    use std::fs;

    #[test]
    fn collect_files_with_extension_recurses_and_filters_by_extension() {
        let temp = TempDir::new("collect-files");
        fs::create_dir_all(temp.path().join("src/nested")).unwrap();
        fs::write(temp.path().join("src/Main.java"), "").unwrap();
        fs::write(temp.path().join("src/nested/Other.java"), "").unwrap();
        fs::write(temp.path().join("src/nested/notes.txt"), "").unwrap();

        let mut files = collect_files_with_extension(&temp.path().join("src"), "java");
        files.sort();

        assert_eq!(
            files,
            vec![
                temp.path().join("src/Main.java"),
                temp.path().join("src/nested/Other.java"),
            ]
        );
    }

    #[test]
    fn collect_files_with_extension_returns_empty_for_missing_path() {
        let temp = TempDir::new("collect-files-missing");

        assert!(collect_files_with_extension(&temp.path().join("missing"), "java").is_empty());
    }
}
