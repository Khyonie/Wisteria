use std::{
    collections::HashMap,
    env::{self, var_os},
    fs,
    path::PathBuf,
};

use regex::Regex;

pub fn resolve_filepath(
    path: &str,
    environment: &HashMap<String, String>,
    regexes: &HashMap<&str, Regex>,
) -> Result<String, (String, u8)> {
    let mut interim_path: String = path.to_string();

    while let Some(capture) = regexes.get("envvars").unwrap().captures(&interim_path) {
        let (full, [key]) = capture.extract();
        {
            interim_path = match environment.get(key) {
                Some(value) => interim_path.replace(full, value),
                None => {
                    return Err((
                        format!(
                            "Use of undefined environmental variable \"{key}\" in path \"{path}\""
                        ),
                        61,
                    ));
                }
            };
        }
    }

    if interim_path.starts_with('~') {
        interim_path = interim_path.replacen('~', &resolve_os_var("HOME", "HOMEPATH").unwrap(), 1);
    }

    if interim_path.starts_with("./") {
        interim_path.replace_range(..1, env::current_dir().unwrap().to_str().unwrap());
    }

    Ok(interim_path)
}

pub fn ensure_parents(filepath: &str) -> Result<PathBuf, String> {
    let mut path: PathBuf = PathBuf::from(filepath);
    path.pop();

    if path.exists() {
        return Ok(path);
    }

    fs::create_dir_all(&path)
        .map(|_| path)
        .map_err(|e| format!("Could not create parent directories for file {filepath}: {e}"))
}

fn resolve_os_var(unix: &str, windows: &str) -> Option<String> {
    match env::consts::OS {
        "macos" | "linux" => var_os(unix).map(|s| s.to_string_lossy().to_string()),
        "windows" => var_os(windows).map(|s| s.to_string_lossy().to_string()),
        _ => {
            println!(
                "You're using an unknown operating system. Cannot resolve environmental variables."
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TempDir, with_current_dir};

    fn regexes() -> HashMap<&'static str, Regex> {
        let mut regexes = HashMap::new();
        regexes.insert("envvars", Regex::new(r#"\{(.+?)}"#).unwrap());
        regexes
    }

    fn environment() -> HashMap<String, String> {
        HashMap::from([
            (String::from("project_name"), String::from("Demo")),
            (String::from("configuration"), String::from("main")),
            (String::from("version"), String::from("1.2.3")),
        ])
    }

    #[test]
    fn resolves_project_environment_placeholders() {
        assert_eq!(
            resolve_filepath(
                "target/{configuration}/{project_name}-{version}.jar",
                &environment(),
                &regexes(),
            )
            .unwrap(),
            "target/main/Demo-1.2.3.jar"
        );
    }

    #[test]
    fn fails_on_unknown_environment_placeholder() {
        let error =
            resolve_filepath("target/{missing}.jar", &environment(), &regexes()).unwrap_err();

        assert!(error.0.contains("undefined environmental variable"));
        assert_eq!(error.1, 61);
    }

    #[test]
    fn expands_dot_relative_paths_from_current_directory() {
        let temp = TempDir::new("paths-relative");

        with_current_dir(temp.path(), || {
            let resolved =
                resolve_filepath("./target/app.jar", &environment(), &regexes()).unwrap();

            assert_eq!(
                resolved,
                temp.path()
                    .join("target/app.jar")
                    .to_string_lossy()
                    .to_string()
            );
        });
    }

    #[test]
    fn ensure_parents_creates_missing_parent_directories() {
        let temp = TempDir::new("paths-parents");
        let target = temp.path().join("a/b/c.jar");

        let parent = ensure_parents(&target.to_string_lossy()).unwrap();

        assert_eq!(parent, temp.path().join("a/b"));
        assert!(parent.exists());
    }
}
