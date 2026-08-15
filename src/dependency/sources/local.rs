use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use regex::Regex;

use crate::dependency::resolver::ResolvedDependency;
use crate::workspace::paths::resolve_filepath;

pub fn resolve_file(
    name: &str,
    path: &str,
    environment: &HashMap<String, String>,
    regexes: &HashMap<&str, Regex>,
) -> Result<ResolvedDependency, String> {
    let path = resolve_filepath(path, environment, regexes)?;
    let pathbuf = PathBuf::from(&path);

    if !pathbuf.exists() {
        return Err(format!("Dependency \"{path}\" does not exist"));
    }

    if pathbuf.is_dir() {
        return Err(format!(
            "Dependency \"{path}\" is a file, not a library. To load a folder, use a \"loadFolder\" dependency type"
        ));
    }

    let canon_path = match pathbuf.canonicalize() {
        Ok(p) => p,
        Err(e) => return Err(format!("Could not canonicalize path \"{path}\": {e}")),
    };

    Ok(ResolvedDependency::from_paths(
        String::from(name),
        vec![canon_path],
    ))
}

pub fn resolve_folder(
    name: &str,
    path: &str,
    recursive: bool,
    environment: &HashMap<String, String>,
    regexes: &HashMap<&str, Regex>,
) -> Result<ResolvedDependency, String> {
    let path = resolve_filepath(path, environment, regexes)?;
    let pathbuf = PathBuf::from(&path);

    if !pathbuf.exists() {
        return Err(format!("Dependency folder \"{path}\" does not exist"));
    }

    if pathbuf.is_file() {
        return Err(format!(
            "Dependency folder \"{path}\" is a regular file, not a folder"
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

    Ok(ResolvedDependency::from_paths(String::from(name), files))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;
    use std::{collections::HashMap, fs};

    fn regexes() -> HashMap<&'static str, Regex> {
        let mut regexes = HashMap::new();
        regexes.insert("envvars", Regex::new(r#"\{(.+?)}"#).unwrap());
        regexes
    }

    fn environment() -> HashMap<String, String> {
        HashMap::from([(String::from("lib"), String::from("library.jar"))])
    }

    #[test]
    fn resolve_file_returns_canonical_file_path() {
        let temp = TempDir::new("local-resolve-file");
        let library = temp.path().join("library.jar");
        fs::write(&library, "").unwrap();

        let resolved = resolve_file(
            "library",
            &library.to_string_lossy(),
            &environment(),
            &regexes(),
        )
        .unwrap();

        assert_eq!(
            resolved.paths().cloned().collect::<Vec<_>>(),
            vec![library.canonicalize().unwrap()]
        );
    }

    #[test]
    fn resolve_file_substitutes_environment_variables() {
        let temp = TempDir::new("local-resolve-env-file");
        let library = temp.path().join("library.jar");
        fs::write(&library, "").unwrap();
        let path = temp.path().join("{lib}");

        let resolved = resolve_file(
            "library",
            &path.to_string_lossy(),
            &environment(),
            &regexes(),
        )
        .unwrap();

        assert_eq!(
            resolved.paths().cloned().collect::<Vec<_>>(),
            vec![library.canonicalize().unwrap()]
        );
    }

    #[test]
    fn resolve_file_rejects_missing_file() {
        let temp = TempDir::new("local-missing-file");
        let error = resolve_file(
            "library",
            &temp.path().join("missing.jar").to_string_lossy(),
            &environment(),
            &regexes(),
        )
        .unwrap_err();

        assert!(error.contains("does not exist"));
    }

    #[test]
    fn resolve_folder_collects_jar_files_recursively_when_requested() {
        let temp = TempDir::new("local-resolve-folder");
        fs::create_dir_all(temp.path().join("lib/nested")).unwrap();
        fs::write(temp.path().join("lib/root.jar"), "").unwrap();
        fs::write(temp.path().join("lib/nested/nested.jar"), "").unwrap();
        fs::write(temp.path().join("lib/nested/readme.txt"), "").unwrap();

        let resolved = resolve_folder(
            "library",
            &temp.path().join("lib").to_string_lossy(),
            true,
            &environment(),
            &regexes(),
        )
        .unwrap();
        let mut paths = resolved.paths().cloned().collect::<Vec<_>>();
        paths.sort();

        assert_eq!(
            paths,
            vec![
                temp.path().join("lib/nested/nested.jar"),
                temp.path().join("lib/root.jar"),
            ]
        );
    }

    #[test]
    fn resolve_folder_skips_nested_jar_files_when_not_recursive() {
        let temp = TempDir::new("local-resolve-folder-nonrecursive");
        fs::create_dir_all(temp.path().join("lib/nested")).unwrap();
        fs::write(temp.path().join("lib/root.jar"), "").unwrap();
        fs::write(temp.path().join("lib/nested/nested.jar"), "").unwrap();

        let resolved = resolve_folder(
            "library",
            &temp.path().join("lib").to_string_lossy(),
            false,
            &environment(),
            &regexes(),
        )
        .unwrap();

        assert_eq!(
            resolved.paths().cloned().collect::<Vec<_>>(),
            vec![temp.path().join("lib/root.jar")]
        );
    }
}
