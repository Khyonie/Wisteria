use std::{collections::HashMap, fs, path::PathBuf, process::Command};

use regex::Regex;
use sha256::digest;

use crate::java::manifest::{Manifest, ManifestEntry};
use crate::model::Configuration;
use crate::workspace::paths::resolve_filepath;

pub fn package_jar(
    configuration: &Configuration,
    dep_paths: &[PathBuf],
    shaded_jars: &[PathBuf],
    regexes: &HashMap<&str, Regex>,
) -> Result<(), (String, u8)> {
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

    let manifest_path = PathBuf::from(".wisteria/work/bin/META-INF/");
    if manifest_path.exists() {
        fs::remove_dir_all(".wisteria/work/bin/META-INF/").unwrap();
    }
    fs::create_dir_all(manifest_path).unwrap();
    fs::write(
        ".wisteria/work/bin/META-INF/MANIFEST.MF",
        manifest.to_file(),
    )
    .unwrap();

    let mut jar_command = Command::new("jar");
    jar_command.args(["-cMf", ".wisteria/work/target.jar"]);
    if let Some(includes) = configuration.includes() {
        for i in includes {
            jar_command.arg(i);
        }
    }

    jar_command.args(["-C", ".wisteria/work/bin/", "."]);

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
        }
        Err(e) => return Err((format!("Failed to package: {e}"), 1)),
    }

    if !shaded_jars.is_empty() {
        let mut jar_update_command: Command = Command::new("jar");
        jar_update_command.args([
            "-uf",
            ".wisteria/work/target.jar",
            "-C",
            ".wisteria/work/shaded/",
            ".",
        ]);

        let _ = jar_update_command.output();
    }

    let bytes: Vec<u8> = fs::read(".wisteria/work/target.jar").unwrap();
    let hash = digest(bytes);
    println!("Packaged, hash: #{hash}");

    if let Some(targets) = configuration.targets() {
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

            fs::write(&target, fs::read(".wisteria/work/target.jar").unwrap())
                .map_err(|e| (format!("Failed to write to target {target}: {e}"), 1))?;
            println!("Successfully written target {target}");
        }
    }

    Ok(())
}
