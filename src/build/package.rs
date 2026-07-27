use std::{collections::HashMap, fs, path::PathBuf, process::Command};

use regex::Regex;
use sha256::digest;

use crate::java::manifest::{Manifest, ManifestEntry};
use crate::model::Configuration;
use crate::util::consts;
use crate::workspace::paths::resolve_filepath;

pub fn package_jar(
    configuration: &Configuration,
    dep_paths: &[PathBuf],
    shaded_jars: &[PathBuf],
    targets: Option<&Vec<String>>,
    regexes: &HashMap<&str, Regex>,
) -> Result<Vec<String>, (String, u8)> {
    let mut manifest: Manifest = Manifest::new();
    manifest.add_entry(ManifestEntry::CreatedBy {
        signature: String::from("Wisteria 3"),
    });

    if let Some(entry) = configuration.entry() {
        manifest.add_entry(ManifestEntry::MainClass {
            class: entry.clone(),
        })
    }

    if !dep_paths.is_empty() {
        let dep_strings: Vec<String> = dep_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        manifest.add_entry(ManifestEntry::ClassPath { path: dep_strings })
    }

    let manifest_path = PathBuf::from(consts::MANIFEST_DIR);
    if manifest_path.exists() {
        fs::remove_dir_all(consts::MANIFEST_DIR).unwrap();
    }
    fs::create_dir_all(manifest_path).unwrap();
    fs::write(consts::MANIFEST_FILE, manifest.to_file()).unwrap();

    let mut jar_command = Command::new("jar");
    jar_command.args(["-cMf", consts::TARGET_JAR_PATH]);
    if let Some(includes) = configuration.includes() {
        for i in includes {
            jar_command.arg(i);
        }
    }

    jar_command.args(["-C", consts::BINARY_OUT_PATH, "."]);

    match jar_command.output() {
        Ok(output) => {
            let stdout = String::from_utf8(output.stdout).unwrap();
            let stderr = String::from_utf8(output.stderr).unwrap();
            if !stdout.is_empty() {
                println!("{stdout}")
            }
            if !stderr.is_empty() {
                println!("{stderr}")
            }

            if !output.status.success() {
                let code = output.status.code().unwrap_or(1);
                return Err((
                    format!("jar package failed with status {}", output.status),
                    u8::try_from(code).unwrap_or(1),
                ));
            }
        }
        Err(e) => return Err((format!("Failed to package: {e}"), 1)),
    }

    if !shaded_jars.is_empty() {
        let mut jar_update_command: Command = Command::new("jar");
        jar_update_command.args([
            "-uf",
            consts::TARGET_JAR_PATH,
            "-C",
            consts::SHADED_OUT_PATH,
            ".",
        ]);

        match jar_update_command.output() {
            Ok(output) => {
                let stdout = String::from_utf8(output.stdout).unwrap();
                let stderr = String::from_utf8(output.stderr).unwrap();
                if !stdout.is_empty() {
                    println!("{stdout}")
                }
                if !stderr.is_empty() {
                    println!("{stderr}")
                }

                if !output.status.success() {
                    let code = output.status.code().unwrap_or(1);
                    return Err((
                        format!("jar shade update failed with status {}", output.status),
                        u8::try_from(code).unwrap_or(1),
                    ));
                }
            }
            Err(e) => return Err((format!("Failed to update package with shaded jars: {e}"), 1)),
        }
    }

    let mut outputs = Vec::new();
    let bytes: Vec<u8> = fs::read(consts::TARGET_JAR_PATH).unwrap();
    let hash = digest(bytes);
    println!("Packaged, hash: #{hash}");

    if let Some(targets) = targets {
        for target in targets {
            let target = resolve_filepath(target, configuration.environment(), regexes)?;
            let target_path: PathBuf = PathBuf::from(&target);

            if !target_path.exists() {
                let parent = target_path.parent().unwrap();

                fs::create_dir_all(parent).map_err(|e| {
                    (
                        format!(
                            "Could not create parent folder {}: {e}",
                            parent.to_string_lossy()
                        ),
                        1,
                    )
                })?;
            }

            fs::write(&target, fs::read(consts::TARGET_JAR_PATH).unwrap())
                .map_err(|e| (format!("Failed to write to target {target}: {e}"), 1))?;
            println!("Successfully written target {target}");
            outputs.push(target_path.to_string_lossy().to_string())
        }
    }

    Ok(outputs)
}
