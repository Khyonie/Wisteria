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
