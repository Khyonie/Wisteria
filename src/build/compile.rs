use std::process::Command;

use crate::model::Configuration;

pub fn compile_sources(
    configuration: &Configuration,
    copied_files: Vec<String>,
    classpath: Option<&str>,
) -> Result<(), (String, u8)> {
    let mut javac_command: Command = Command::new("javac");
    javac_command.args(["-d", "./.wisteria/work/bin/"]);
    javac_command.args(["--source-path", ".wisteria/work/src/"]);

    if let Some(deps) = classpath {
        javac_command.args(["--class-path", deps]);
    }

    if let Some(flags) = configuration.compiler_flags() {
        for flag in flags {
            javac_command.args(flag.get_canon_flag());
        }
    }

    for file in copied_files {
        javac_command.arg(file);
    }

    println!("Compiling sources");
    match javac_command.output() {
        Ok(out) => {
            if !out.stdout.is_empty() {
                println!("{}", String::from_utf8(out.stdout).unwrap());
            }

            if !out.stderr.is_empty() {
                let stderr = String::from_utf8(out.stderr).unwrap();
                println!("{stderr}");

                if !stderr.starts_with("Note: ") {
                    return Err((String::from("Could not compile project"), 1));
                }
            }
        }
        Err(e) => return Err((format!("{e}"), 1)),
    }

    Ok(())
}
