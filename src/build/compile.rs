use std::process::Command;

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
    let mut javac_command: Command = Command::new("javac");
    javac_command.args(["-d", consts::BINARY_OUT_PATH]);
    javac_command.args(["--source-path", consts::SOURCE_OUT_PATH]);

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
