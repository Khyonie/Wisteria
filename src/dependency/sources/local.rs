use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use regex::Regex;

use crate::workspace::paths::resolve_filepath;

pub fn resolve_file(
    path: &str,
    environment: &HashMap<String, String>,
    regexes: &HashMap<&str, Regex>,
) -> Result<Vec<PathBuf>, (String, u8)> {
    let path = resolve_filepath(path, environment, regexes)?;
    let pathbuf = PathBuf::from(&path);

    if !pathbuf.exists() {
        return Err((format!("Dependency \"{path}\" does not exist"), 63));
    }

    if pathbuf.is_dir() {
        return Err((format!("Dependency \"{path}\" is a file, not a library. To load a folder, use a \"loadFolder\" dependency type"), 63));
    }

    let canon_path = match pathbuf.canonicalize() {
        Ok(p) => p,
        Err(e) => return Err((format!("Could not canonicalize path \"{path}\": {e}"), 62)),
    };

    println!("File found: {}", &canon_path.to_string_lossy());
    Ok(vec![canon_path])
}

pub fn resolve_folder(
    path: &str,
    recursive: bool,
    environment: &HashMap<String, String>,
    regexes: &HashMap<&str, Regex>,
) -> Result<Vec<PathBuf>, (String, u8)> {
    let path = resolve_filepath(path, environment, regexes)?;
    let pathbuf = PathBuf::from(&path);

    if !pathbuf.exists() {
        return Err((format!("Dependency folder \"{path}\" does not exist"), 63));
    }

    if pathbuf.is_file() {
        return Err((
            format!("Dependency folder \"{path}\" is a regular file, not a folder"),
            1,
        ));
    }

    let mut files: Vec<PathBuf> = Vec::new();

    if let Ok(dir) = pathbuf.read_dir() {
        for file in dir.flatten() {
            if file.path().is_dir() {
                if recursive {
                    collect_recursive(&file.path(), &mut files)
                }
                continue;
            }

            if file.file_name().to_string_lossy().ends_with(".jar") {
                files.push(file.path());
            }
        }
    }

    let text_plural = if files.len() == 1 {
        "library"
    } else {
        "libraries"
    };

    println!("Found {} {text_plural}", &files.len());
    Ok(files)
}

fn collect_recursive(path: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(dir) = path.read_dir() {
        for f in dir.flatten() {
            if f.path().is_dir() {
                collect_recursive(&f.path(), files);
                continue;
            }

            if f.file_name().to_string_lossy().ends_with(".jar") {
                files.push(f.path());
            }
        }
    }
}
