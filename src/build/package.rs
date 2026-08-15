use std::{collections::HashMap, fs, path::PathBuf, process::Command};

use regex::Regex;
use sha256::digest;

use crate::java::manifest::{Manifest, ManifestEntry};
use crate::model::Configuration;
use crate::output::{self, OutputRenderer};
use crate::util::{consts, exit_code};
use crate::workspace::paths::resolve_filepath;

pub fn package_jar(
    configuration: &Configuration,
    dep_paths: &[PathBuf],
    shaded_jars: &[PathBuf],
    targets: Option<&Vec<String>>,
    regexes: &HashMap<&str, Regex>,
    renderer: &mut dyn OutputRenderer,
) -> Result<String, String> {
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
        fs::remove_dir_all(consts::MANIFEST_DIR)
            .map_err(|e| format!("Failed to remove manifest path: {e}"))?;
    }
    fs::create_dir_all(manifest_path)
        .map_err(|e| format!("Failed to create manifest path: {e}"))?;
    fs::write(consts::MANIFEST_FILE, manifest.to_file())
        .map_err(|e| format!("Failed to write manifest file: {e}"))?;

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
            output::log_process_output(renderer, &output.stdout, &output.stderr);

            if !output.status.success() {
                exit_code::record_external_process_exit_code(output.status);
                return Err(format!("jar package failed with status {}", output.status));
            }
        }
        Err(e) => return Err(format!("Failed to package: {e}")),
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
                output::log_process_output(renderer, &output.stdout, &output.stderr);

                if !output.status.success() {
                    exit_code::record_external_process_exit_code(output.status);
                    return Err(format!(
                        "jar shade update failed with status {}",
                        output.status
                    ));
                }
            }
            Err(e) => return Err(format!("Failed to update package with shaded jars: {e}")),
        }
    }

    let bytes: Vec<u8> = fs::read(consts::TARGET_JAR_PATH)
        .map_err(|e| format!("Failed to read packaged jar for hashing: {e}"))?;
    let hash = digest(&bytes);

    if let Some(targets) = targets {
        for target in targets {
            let target = resolve_filepath(target, configuration.environment(), regexes)?;
            let target_path: PathBuf = PathBuf::from(&target);

            if !target_path.exists()
                && let Some(parent) = target_path.parent()
                && !parent.as_os_str().is_empty()
            {
                fs::create_dir_all(parent).map_err(|e| {
                    format!(
                        "Could not create parent folder {}: {e}",
                        parent.to_string_lossy()
                    )
                })?;
            }

            fs::write(&target, &bytes)
                .map_err(|e| format!("Failed to write to target {target}: {e}"))?;
            renderer.log(&format!("Successfully written target {target}"));
        }
    }

    Ok(hash)
}
