use std::{collections::HashMap, env::{self, var_os}, fs::{self, File}, io::copy, path::{Path, PathBuf}};

use regex::Regex;
use reqwest::{blocking::Client, StatusCode};
use toml::Table;

pub const UNIX_CONFIG_PATH: &str = "$HOME/.config/wisteria/";
pub const WINDOWS_CONFIG_PATH: &str = "%LOCALAPPDATA%/Wisteria/";

pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36";

pub fn resolve_filepath(path: &String, environment: &HashMap<String, String>, regexes: &HashMap<&str, Regex>) -> Result<String, (String, u8)>
{
    let mut interim_path: String = path.clone();

    // Resolve environmental variables
    while let Some(capture) = regexes.get("envvars").unwrap().captures(&interim_path)
    {
        let (full, [key]) = capture.extract();
        {
            interim_path = match environment.get(key)
            {
                Some(value) => interim_path.replace(full, value),
                None => return Err((format!("Use of undefined environmental variable \"{key}\" in path \"{path}\""), 61))
            };
        }
    }

    // Resolve home path
    if interim_path.starts_with('~')
    {
        interim_path = interim_path.replacen('~', &resolve_os_var("HOME", "HOMEPATH").unwrap(), 1);
    }

    // Resolve working directory
    if interim_path.starts_with("./")
    {
        interim_path.replace_range(..1, env::current_dir().unwrap().to_str().unwrap());
    }

    // TODO Replace other user vars

    Ok(interim_path)
}

pub fn download(name: String, url: String, filepath: String) -> Result<(), (String, u8)>
{
    let client: Client = Client::new();
    let mut response = match client.get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
    {
        Ok(r) => r,
        Err(e) => return Err((format!("{url}, status: {}", e.status().unwrap_or(StatusCode::SERVICE_UNAVAILABLE).as_str()), 1))
    };

    if !response.status().is_success()
    {
        return Err((format!("{url}, status: {}", response.status().as_str()), 1))
    }
    // TODO Give codes a meaningful value

    let mut file = File::create(&filepath)
        .map_err(| e | (format!("Could not create file {filepath} for dependency {name}: {e}"), 1))?;

    let size: f32 = match copy(&mut response, &mut file) {
        Ok(v) => v as f32 / 1000000.0,
        Err(e) => return Err((format!("Could not copy from URL {url} into file {filepath}: {e}"), 1))
    };

    println!("Copied {:.3} MB into {filepath}", size);
    
    Ok(())
}

fn resolve_os_var(unix: &str, windows: &str) -> Option<String>
{
    match env::consts::OS
    {
        "macos" | "linux" => var_os(unix).map(| s | s.to_string_lossy().to_string() ),
        "windows" => var_os(windows).map(| s | s.to_string_lossy().to_string() ),
        _ => {
            println!("You're using an unknown operating system. Cannot resolve environmental variables.");
            None
        }
    }
}

pub fn ensure_parents(filepath: &str) -> Result<PathBuf, String>
{
    let mut path: PathBuf = PathBuf::from(filepath);
    path.pop();

    if path.exists()
    {
        return Ok(path);
    }

    fs::create_dir_all(&path)
        .map(| _ | path)
        .map_err(| e | format!("Could not create parent directories for file {filepath}: {e}"))
}

pub fn ensure_toml(filepath: &str, toml: Table) -> Result<(), String>
{
    let path: PathBuf = PathBuf::from(filepath);

    if path.exists()
    {
        return Ok(())
    }

    let parents = path.parent().unwrap();
    if !parents.exists()
    {
        fs::create_dir_all(parents).map_err(| e | format!("Could not create parent directory for {filepath}: {e}"))?;
    }

    fs::write(filepath, toml::to_string_pretty(&toml).unwrap()).map_err(| e | format!("Could not write TOML file {filepath}: {e}"))?;

    Ok(())
}

pub fn collect_files_with_extension(path: &Path, extension: &str) -> Vec<PathBuf>
{
    let mut files: Vec<PathBuf> = Vec::new();

    collect_files_recursive(path, extension, &mut files);

    files
}

fn collect_files_recursive(path: &Path, extension: &str, files: &mut Vec<PathBuf>)
{
    if !path.exists()
    {
        return
    }

    let read = match path.read_dir() {
        Ok(r) => r,
        Err(e) => {
            println!("Could not read source \"{}\": {e}", path.to_string_lossy());
            return
        }
    };

    for dir in read
    {
        let entry = match dir {
            Ok(e) => e,
            Err(e) => {
                println!("{e}");
                continue
            }
        };
        let new_path = entry.path();

        if new_path.is_dir()
        {
            collect_files_recursive(&new_path, extension, files);
            continue
        }

        if let Some(ext) = new_path.extension()
        {
            if ext == extension
            {
                files.push(new_path)
            }
        }
    }
}
