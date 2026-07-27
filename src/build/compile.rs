use std::process::Command;

use crate::{model::Configuration, util::consts};

pub fn compile_sources(
    configuration: &Configuration,
    copied_files: Vec<String>,
    classpath: Option<&str>,
) -> Result<(), (String, u8)> {
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

    println!("Compiling {} source files", copied_files.len());
    match javac_command.output() {
        Ok(out) => {
            if !out.stdout.is_empty() {
                println!("{}", String::from_utf8(out.stdout).unwrap());
            }

            if !out.stderr.is_empty() {
                println!("{}", String::from_utf8(out.stderr).unwrap());
            }

            if !out.status.success() {
                let code = out.status.code().unwrap_or(1);
                return Err((
                    format!("javac failed with status {}", out.status),
                    u8::try_from(code).unwrap_or(1),
                ));
            }
        }
        Err(e) => return Err((format!("{e}"), 1)),
    }

    Ok(())
}
