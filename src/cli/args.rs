use std::{env, process::exit};

use crate::util::consts;

/// Flags added with the -- prefix.
pub struct StartupFlags {
    pub minimal: bool,
    pub use_project: Option<String>,
    pub no_refresh: bool,
}

/// Takes the arguments passed into the program and turns them into arguments and flags.
pub fn load_arguments(args: &mut Vec<String>) -> StartupFlags {
    let raw_args: Vec<String> = env::args().collect();

    let mut flags: StartupFlags = StartupFlags {
        minimal: false,
        use_project: None,
        no_refresh: false,
    };
    let mut args_iter = raw_args.iter();

    while let Some(arg) = args_iter.next() {
        if arg.starts_with("--") {
            match arg.split_once("--").unwrap().1 {
                "minimal" => flags.minimal = true,
                "norefresh" => flags.no_refresh = true,
                "project" => match args_iter.next() {
                    Some(a) => flags.use_project = Some(a.clone()),
                    None => {
                        println!("Missing argument for --project flag. Must specify the file which contains the project configuration, usually \"project.toml\".");
                        exit(1)
                    }
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
