use std::{env, path::PathBuf, process::{exit, Command, Output}};

use regex::Regex;

use crate::{silentln, Flags};

pub struct MajorVersion 
{
    pub version: u8
}

pub fn get_java_version(flags: &Flags, version: &mut MajorVersion) -> String
{
    let path_seperator = match env::consts::OS
    {
        "windows" => ';',
        _ => ':'
    };

    match flags.use_java_executable.as_ref()
    {
        Some(home) => {
            let mut found_javac = false;
            let mut found_java = false;

            if PathBuf::from(format!("{}/bin/javac", home).replace("//", "/")).exists()
            {
                found_javac = true;
            }

            if PathBuf::from(format!("{}/bin/java", home).replace("//", "/")).exists()
            {
                found_java = true;
            }

            if !found_javac && !found_java
            {
                silentln!(flags, "No java installation found at the specified path. Install a Java development kit from Oracle's website (on windows) or your favorite package manager.");
                exit(1);
            }

            if found_java && !found_javac
            {
                silentln!(flags, "Specified path points to a valid Java runtime but not a valid JDK.");
                exit(1);
            }

            // Query java version
            let output = Command::new(format!("{}/bin/java", home).replace("//", "/"))
                .arg("-version")
                .output()
                .unwrap();

            return extract_java_version(output, version);
        }, 
        None => {}
    }

    // Otherwise search through path
    if let Ok(path_env) = env::var("PATH")
    {
        let mut found_javac = false;
        let mut found_java = false;
        for path_entry in path_env.split(path_seperator)
        {
            let path: PathBuf = PathBuf::from(path_entry);
            if let Ok(entries) = path.read_dir()
            {
                for entry in entries
                {
                    if let Ok(e) = entry
                    {
                        match e.path().file_stem().unwrap().to_string_lossy().trim_end_matches(".exe")
                        {
                            "javac" => found_javac = true,
                            "java" => found_java = true,
                            _ => continue
                        }
                    }
                }
            }
        }

        if found_java && found_javac
        {
            // Query java version
            let output = Command::new("java")
                .arg("-version")
                .output()
                .unwrap();

            return extract_java_version(output, version);
        }

        if found_java && !found_javac
        {
            silentln!(flags, "Found a Java runtime but no JDK. Install a Java development kit from Oracle's website (on windows) or your favorite package manager.");
            exit(1);
        }

        silentln!(flags, "Java is not installed. Install a Java development kit from Oracle's website (on windows) or your favorite package manager.");
        exit(1);
    }

    "(Not found)".to_string()
}

fn extract_java_version(output: Output, version: &mut MajorVersion) -> String
{
    let output_string = String::from_utf8(output.stderr).unwrap_or("(Unknown)".to_string());
    
    for capture in Regex::new("(.*) version \"(.*)\"").unwrap().captures_iter(&output_string)
    {
        match capture.extract()
        {
            (_, [distributor, ver]) => {
                if ver.starts_with("1.") // Older Java versions
                {
                    version.version = ver.split(".").collect::<Vec<&str>>()[1].parse().unwrap();
                } else { // Newer java versions
                    version.version = match ver.split_once(".").unwrap()
                    {
                        (major, _) => major.parse().unwrap()
                    };
                }

                return format!("{} {}", distributor, ver).to_string();
            }
        }
    }

    "(Not found)".to_string()
}

pub fn get_seperator() -> char
{
    match env::consts::OS
    {
        "windows" => ';',
        _ => ':'
    }
}
