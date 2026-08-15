use std::{env, process::exit};

use crate::output::OutputMode;
use crate::util::consts;

/// Flags added with the -- prefix.
#[derive(Clone, Default)]
pub struct StartupFlags {
    pub minimal: bool,
    pub use_project: Option<String>,
    pub no_refresh: bool,
    pub no_git: bool,
    pub output_mode: OutputMode,
    pub passed_args: Vec<String>,
}

/// Takes the arguments passed into the program and turns them into arguments and flags.
pub fn load_arguments(args: &mut Vec<String>) -> StartupFlags {
    let raw_args: Vec<String> = env::args().collect();

    let mut flags: StartupFlags = StartupFlags::default();
    let mut args_iter = raw_args.iter();

    let mut passed = false;
    while let Some(arg) = args_iter.next() {
        // Args after a "--" are passed into Java as-is, when running jars.
        if passed {
            flags.passed_args.push(arg.clone());
            continue;
        }

        if arg.starts_with("--") {
            // Passed args
            if arg.len() == 2 {
                passed = true;
                continue;
            }

            let raw_flag = arg.strip_prefix("--").unwrap();
            let (flag, inline_value) = match raw_flag.split_once('=') {
                Some((flag, value)) => (flag, Some(value.to_string())),
                None => (raw_flag, None),
            };

            // Wisteria args
            match flag {
                "minimal" => flags.minimal = true,
                "norefresh" => flags.no_refresh = true,
                "nogit" => flags.no_git = true,
                "output" | "format" => match flag_value(
                    flag,
                    inline_value,
                    &mut args_iter,
                    "Expected one of [auto, plain, terminal, json].",
                ) {
                    Some(value) => match OutputMode::load(&value) {
                        Ok(mode) => flags.output_mode = mode,
                        Err(error) => {
                            println!("{error}");
                            exit(1)
                        }
                    },
                    None => exit(1),
                },
                "project" => match flag_value(
                    flag,
                    inline_value,
                    &mut args_iter,
                    &format!(
                        "Must specify the file which contains the project configuration, usually \"{}\".",
                        consts::PROJECT_FILE
                    ),
                ) {
                    Some(value) => flags.use_project = Some(value),
                    None => exit(1),
                },
                _ => {
                    println!("Unknown flag \"{arg}\"");
                    exit(1)
                }
            }

            continue;
        }
        args.push(arg.clone())
    }

    if args.len() == 1 {
        println!("Not enough arguments.\n{}", consts::USAGE_TEXT);
        exit(1);
    }

    flags
}

fn flag_value(
    flag: &str,
    inline_value: Option<String>,
    args_iter: &mut std::slice::Iter<'_, String>,
    expectation: &str,
) -> Option<String> {
    match inline_value {
        Some(value) => Some(value),
        None => match args_iter.next() {
            Some(value) => Some(value.clone()),
            None => {
                println!("Missing argument for --{flag} flag. {expectation}");
                None
            }
        },
    }
}
