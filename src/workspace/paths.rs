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
) -> Result<String, String> {
    let mut interim_path: String = path.to_string();
    let envvar_regex = regexes.get("envvars").ok_or_else(|| {
        format!(
            "Could not resolve path \"{path}\": internal envvar matcher is missing.\nFix: report this as a Wisteria bug; path expansion requires the `envvars` regex to be registered."
        )
    })?;

    while let Some(capture) = envvar_regex.captures(&interim_path) {
        let (full, [key]) = capture.extract();
        {
            interim_path = match environment.get(key) {
                Some(value) => interim_path.replace(full, value),
                None => {
                    return Err(format!(
                        "Use of undefined environmental variable \"{key}\" in path \"{path}\""
                    ));
                }
            };
        }
    }

    if interim_path.starts_with('~') {
        let home = resolve_os_var("HOME", "HOMEPATH").ok_or_else(|| {
            format!(
                "Could not expand home directory in path \"{path}\" because no home directory environment variable is available.\nFix: set HOME on macOS/Linux or HOMEPATH on Windows, or use an absolute path instead of `~`."
            )
        })?;
        interim_path = interim_path.replacen('~', &home, 1);
    }

    if interim_path.starts_with("./") {
        let current_dir = env::current_dir().map_err(|e| {
            format!(
                "Could not resolve relative path \"{path}\" because the current directory could not be read: {e}.\nFix: run Wisteria from a valid project directory or use an absolute path."
            )
        })?;
        let current_dir = current_dir.to_str().ok_or_else(|| {
            format!(
                "Could not resolve relative path \"{path}\" because the current directory \"{}\" is not valid UTF-8.\nFix: move the project to a path with UTF-8-compatible characters or use an absolute path.",
                current_dir.to_string_lossy()
            )
        })?;
        interim_path.replace_range(..1, current_dir);
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

        assert!(error.contains("undefined environmental variable"));
    }

    #[test]
    fn fails_when_envvar_regex_is_missing() {
        let error = resolve_filepath(
            "target/{configuration}.jar",
            &environment(),
            &HashMap::new(),
        )
        .unwrap_err();

        assert!(error.contains("internal envvar matcher is missing"));
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

    #[cfg(unix)]
    #[test]
    fn fails_when_current_directory_is_not_valid_utf8() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let temp = TempDir::new("paths-invalid-utf8");
        let invalid_dir = temp
            .path()
            .join(OsString::from_vec(vec![b'd', b'i', b'r', 0x80]));
        fs::create_dir_all(&invalid_dir).unwrap();

        with_current_dir(&invalid_dir, || {
            let error =
                resolve_filepath("./target/app.jar", &environment(), &regexes()).unwrap_err();

            assert!(error.contains("not valid UTF-8"));
        });
    }
}
