use std::{fs, process::Command};

use crate::{
    model::Configuration,
    output::{self, OutputRenderer},
    util::{consts, exit_code},
};

pub fn compile_sources(
    configuration: &Configuration,
    copied_files: Vec<String>,
    classpath: Option<&str>,
    renderer: &mut dyn OutputRenderer,
) -> Result<(), String> {
    compile_sources_to(
        configuration,
        copied_files,
        classpath,
        consts::SOURCE_OUT_PATH,
        consts::BINARY_OUT_PATH,
        renderer,
    )
}

pub fn compile_sources_to(
    configuration: &Configuration,
    copied_files: Vec<String>,
    classpath: Option<&str>,
    source_path: &str,
    binary_path: &str,
    renderer: &mut dyn OutputRenderer,
) -> Result<(), String> {
    fs::create_dir_all(binary_path).map_err(|e| {
        format!("Failed to create Java class output directory \"{binary_path}\": {e}")
    })?;

    let mut javac_command: Command = Command::new("javac");
    javac_command.args(["-d", binary_path]);
    javac_command.args(["--source-path", source_path]);

    if let Some(deps) = classpath {
        javac_command.args(["--class-path", deps]);
    }

    if let Some(flags) = configuration.compiler_flags() {
        for flag in flags {
            javac_command.args(flag.get_canon_flag());
        }
    }

    for file in &copied_files {
        javac_command.arg(file);
    }

    match javac_command.output() {
        Ok(out) => {
            output::log_process_output(renderer, &out.stdout, &out.stderr);

            if !out.status.success() {
                exit_code::record_external_process_exit_code(out.status);
                return Err(format!("javac failed with status {}", out.status));
            }
        }
        Err(e) => return Err(format!("Failed to run javac command: {e}")),
    }

    Ok(())
}
