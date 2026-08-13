use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

pub fn collect_files_with_extension(path: &Path, extension: &str) -> Result<Vec<PathBuf>, String> {
    let mut files: Vec<PathBuf> = Vec::new();

    collect_files_recursive(path, extension, &mut files)?;

    Ok(files)
}

fn collect_files_recursive(
    path: &Path,
    extension: &str,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    let read = match path.read_dir() {
        Ok(r) => r,
        Err(e) => {
            return Err(format!(
                "Could not read directory \"{}\": {e}",
                path.display()
            ));
        }
    };

    for dir in read {
        let entry = match dir {
            Ok(e) => e,
            Err(e) => {
                return Err(format!(
                    "Could not read an entry in directory \"{}\": {e}",
                    path.display()
                ));
            }
        };
        let new_path = entry.path();

        if new_path.is_dir() {
            collect_files_recursive(&new_path, extension, files)?;
            continue;
        }

        if let Some(ext) = new_path.extension()
            && ext == extension
        {
            files.push(new_path)
        }
    }

    Ok(())
}

const BUFFER_SIZE: usize = 0x2000; // 8kb file buffer
pub fn generate_sha2_for_file(path: &Path) -> Result<String, String> {
    let file = File::open(path).map_err(|e| {
        format!(
            "Failed to generate hash for file {}: {e}",
            path.to_string_lossy()
        )
    })?;

    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0; BUFFER_SIZE]; // 8kb buffer

    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read file while hashing it: {e}"))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    // 4. Finalize the hash and convert it to a hex string
    let result = hasher.finalize();
    Ok(hex::encode(result))
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

        let mut files = collect_files_with_extension(&temp.path().join("src"), "java").unwrap();
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

        assert!(
            collect_files_with_extension(&temp.path().join("missing"), "java")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn collect_files_with_extension_errors_when_path_cannot_be_read() {
        let temp = TempDir::new("collect-files-unreadable");
        let source_file = temp.path().join("src");
        fs::write(&source_file, "").unwrap();

        let error = collect_files_with_extension(&source_file, "java").unwrap_err();

        assert!(error.contains("Could not read directory"));
        assert!(error.contains(&source_file.display().to_string()));
    }
}
