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
                    ))
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
