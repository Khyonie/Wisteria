use std::{
    collections::HashMap,
    env,
    fs::{self, create_dir, read_to_string, write},
    path::PathBuf,
    process::exit,
};

use configuration::Configuration;
use dependency::{Dependency, UpdateContext};
use nature::Nature;
use project::Project;
use regex::Regex;
use template::generate_metadata;
use toml::Table;
use util::{
    consts::{self, print_action_header},
    toml_utils::{read_boolean, read_string},
};

mod compiler;
mod configuration;
mod dependency;
mod eclipse;
mod java;
mod maven;
mod nature;
mod project;
mod task;
mod template;
mod util;

pub const VERSION: &str = "3.3.0";

fn main() {
    // Load project
    let mut args: Vec<String> = Vec::new();
    let flags: StartupFlags = load_arguments(&mut args);
    let project: Result<Project, (String, u8)> = Project::from(flags.use_project.clone());

    match args[1].to_lowercase().as_str() {
        // wisteria3 refresh
        "refresh" => trigger_refresh(project),

        // wisteria3 update
        "update" if args.len() == 2 => {
            println!(
                "Not enough arguments. Expected at least one argument, but none were supplied."
            );
            exit(1)
        }
        "update" => trigger_update(project, &args),

        // wisteria3 clean
        "clean" if args.len() == 2 => {
            println!("Not enough arguments. Expected one of [ classes, dependencies, all ], but nothing was supplied.");
            exit(1)
        }
        "clean" => trigger_clean(&args),

        // wisteria3 new/create
        "new" | "create" if args.len() == 2 => exit(1),
        "new" | "create" => trigger_create(&args, &flags),
        "info" => trigger_info(project),
        "switch" => {
            let project: Project = match project {
                Ok(p) => p,
                Err(e) => {
                    println!(
                        "Could not read a Wisteria project.toml file in this directory. ({})",
                        e.0
                    );
                    exit(e.1.into())
                }
            };

            let mut metadata = match load_metadata() {
                Ok(m) => m,
                Err((e, code)) => {
                    println!("{e}");
                    exit(code as i32)
                }
            };

            print_header();

            if metadata.configuration == args[2] {
                println!("Project is already set to use that configuration. To reload the project configuration, use \"wisteria refresh\" instead.");
                exit(1)
            }

            let configuration: &Configuration = match project.info().configurations().get(&args[2])
            {
                Some(c) => c,
                None => {
                    println!("No such configuration \"{}\".", args[2]);
                    exit(1)
                }
            };

            if !flags.no_refresh {
                // TODO Move refresh logic into here
            }
            let mut regexes: HashMap<&str, Regex> = HashMap::new();
            regexes.insert("envvars", Regex::new(r#"\{(.+)}"#).unwrap());

            print!("Removing natures... ");
            for nature in Nature::values() {
                let _ = nature.remove_nature();
            }
            println!("Done!");

            for nature in project.info().natures() {
                print!("Applying project nature \"{}\"... ", nature.type_str());
                nature.setup_nature(&project, configuration, &regexes);
                println!("Done!");
            }

            let mut failed_downloads: Vec<(String, String)> = Vec::new();
            if let Some(dependencies) = configuration.dependencies() {
                let mut width: usize = usize::MIN;
                for name in dependencies.iter() {
                    width = usize::max(name.len(), width);
                }

                width += 5;
                let size = dependencies.len();

                for (index, d) in dependencies.iter().enumerate() {
                    if let Some((name, dep)) = project.dependencies().get_key_value(d) {
                        print!(
                            "({}/{size}) Updating {:width$}",
                            index + 1,
                            format!("{name} ... ")
                        );
                        let _ = match dep.resolve(
                            name,
                            configuration.environment(),
                            &regexes,
                            UpdateContext::SwitchConfiguration,
                        ) {
                            Ok(p) => p,
                            Err(e) => {
                                println!("Could not download {name}: {}", e.0);
                                failed_downloads.push((name.clone(), e.0));
                                continue;
                            }
                        };
                    }
                }
            }

            print!("Finishing up... ");
            metadata.configuration = args[2].clone();
            let _ = write(".wisteria/metadata.toml", generate_metadata(&metadata));
            println!("Done!");

            if failed_downloads.is_empty() {
                println!("Operation complete! Your project is now set up to use the configuration \"{}\".", args[2]);
                exit(0)
            }

            println!("Operation complete with dependency resolution errors. Your project is now set up to use the configuration \"{}\".", args[2]);
            println!("Failed to resolve the following dependencies:");
            for (name, error) in failed_downloads {
                println!("- {name}: {error}")
            }
            exit(1)
        }
        _ => {
            let project: Project = match project {
                Ok(p) => p,
                Err(e) => {
                    println!(
                        "Could not read a Wisteria project.toml file in this directory. ({})",
                        e.0
                    );
                    exit(e.1.into())
                }
            };

            // TODO Load actual configuration from meta
            let metadata = match load_metadata() {
                Ok(m) => m,
                Err((e, code)) => {
                    println!("{e}");
                    exit(code as i32)
                }
            };
            let configuration: &Configuration = project
                .info()
                .configurations()
                .get(&metadata.configuration)
                .unwrap();

            let task = match configuration.tasks().get(&args[1]) {
                Some(t) => t,
                None => {
                    println!("No such task \"{}\" for configuration.", args[1]);
                    exit(1)
                }
            };

            if let Err((message, code)) = task.invoke(project.info(), &project, configuration) {
                println!("Failed to execute task (TODO Chain over to fail if defined): {message}");
                exit(code as i32)
            }

            println!("Operation complete!");
        }
    }
}

//--------------------------------------------------------------------------------

/// Flags added with the -- prefix
struct StartupFlags {
    minimal: bool,
    use_project: Option<String>,
    no_refresh: bool,
}

/// Project metadata
pub struct Metadata {
    dirty: bool,
    configuration: String,
}

fn load_metadata() -> Result<Metadata, (String, u8)> {
    let toml_string = read_to_string(".wisteria/metadata.toml").map_err(|e| (format!("{e}"), 1))?;

    let toml: Table = toml_string.parse::<Table>()
        .map_err(| e | (format!("Invalid or corrupt Wisteria metadata file. Fix \".wisteria/metadata.toml\" in your favorite text editor, or run \"wisteria clean metadata\": {e}"), 1))?;

    let dirty = read_boolean("dirty", &toml)?;
    let configuration = read_string("current_configuration", &toml).unwrap_or(String::from("main"));

    Ok(Metadata {
        dirty,
        configuration,
    })
}

/// "wisteria3 refresh"
fn trigger_refresh(project: Result<Project, (String, u8)>) {
    let project: Project = match project {
        Ok(p) => p,
        Err(e) => {
            println!(
                "Could not read a Wisteria project.toml file in this directory. ({})",
                e.0
            );
            exit(e.1.into())
        }
    };

    let metadata = match load_metadata() {
        Ok(m) => m,
        Err((e, code)) => {
            println!("{e}");
            exit(code as i32)
        }
    };

    print_header();
    println!(
        "Refreshing project \"{}\" with configuration \"{}\"...",
        project.info().name(),
        &metadata.configuration
    );

    let configuration: &Configuration = project
        .info()
        .configurations()
        .get(&metadata.configuration)
        .unwrap();
    let mut regexes: HashMap<&str, Regex> = HashMap::new();
    regexes.insert("envvars", Regex::new(r#"\{(.+)}"#).unwrap());

    print_action_header("Removing natures", 1, 2);
    for nature in Nature::values() {
        print!("> Removing project nature \"{}\" ... ", nature.type_str());
        let _ = nature.remove_nature();
        println!("Done!");
    }
    println!("Natures removed!");

    print_action_header("Applying natures", 2, 2);
    for nature in project.info().natures() {
        println!("> Applying project nature \"{}\"... ", nature.type_str());
        nature.setup_nature(&project, configuration, &regexes);
        println!("Done!");
    }

    println!("Operation complete!");
}

fn trigger_update(project: Result<Project, (String, u8)>, args: &[String]) {
    let project: Project = match project {
        Ok(p) => p,
        Err(e) => {
            println!(
                "Could not read a Wisteria project.toml file in this directory. ({})",
                e.0
            );
            exit(e.1.into())
        }
    };

    let metadata = match load_metadata() {
        Ok(m) => m,
        Err((e, code)) => {
            println!("{e}");
            exit(code as i32)
        }
    };

    let configuration: &Configuration = project
        .info()
        .configurations()
        .get(&metadata.configuration)
        .unwrap();
    let mut regexes: HashMap<&str, Regex> = HashMap::new();
    regexes.insert("envvars", Regex::new(r#"\{(.+)}"#).unwrap());

    if args[2] == "all" {
        let keys: Vec<String> = project.dependencies().keys().cloned().collect();

        let failed: Vec<(String, String)> = update_dependencies(
            &keys,
            project.dependencies(),
            configuration.environment(),
            &regexes,
        );

        if !failed.is_empty() {
            println!("Failed to resolve one or more dependencies:");
            for (name, reason) in &failed {
                println!("\t{name}: {reason}");
            }

            exit(1)
        }

        println!("Operation complete!");

        exit(0)
    }

    let mut target_dependencies: Vec<String> = Vec::new();
    for a in args[2..].iter() {
        if !project.dependencies().contains_key(a) {
            println!("No such dependency \"{}\" has been defined.", a);
            exit(1)
        }

        target_dependencies.push(a.clone());
    }

    let failed: Vec<(String, String)> = update_dependencies(
        &target_dependencies,
        project.dependencies(),
        configuration.environment(),
        &regexes,
    );

    if !failed.is_empty() {
        println!("Failed to resolve one or more dependencies:");
        for (name, reason) in &failed {
            println!("\t{name}: {reason}");
        }

        exit(1)
    }

    println!("Operation complete!");
}

fn trigger_info(project: Result<Project, (String, u8)>) {
    let project: Project = match project {
        Ok(p) => p,
        Err(e) => {
            println!(
                "Could not read a Wisteria project.toml file in this directory. ({})",
                e.0
            );
            exit(e.1.into())
        }
    };
    project.print_info();
    exit(0);
}

fn trigger_clean(args: &[String]) {
    match args[2].to_lowercase().as_str() {
        "classes" => {
            if !PathBuf::from(".wisteria/work/bin/").exists() {
                println!("Binary folder does not exist, nothing to do.");
                exit(0)
            }
            match fs::remove_dir_all(".wisteria/work/bin/") {
                Ok(_) => println!("Operation complete."),
                Err(e) => {
                    println!("Could not remove classes folder: {e}");
                    exit(1)
                }
            }
        }
        "dependencies" => {
            if !PathBuf::from(".wisteria/cache/").exists() {
                println!("Dependency cache folder does not exist, nothing to do.");
                exit(0)
            }
            match fs::remove_dir_all(".wisteria/cache/") {
                Ok(_) => println!("Operation complete."),
                Err(e) => {
                    println!("Could not remove dependency folder: {e}");
                    exit(1)
                }
            }
        }
        _ => println!("Unknown clean target {}", args[2]),
    }
}

fn trigger_create(args: &[String], flags: &StartupFlags) {
    if args[2].contains('/') || args[2].contains('\\') {
        println!("Invalid project name. A project name must not contain any slashes.");
        exit(1)
    }

    let path = PathBuf::from(&args[2]);
    if path.exists() {
        println!("A project by that name already exists in this directory.");
        exit(1)
    }

    if create_dir(&path).is_err() {
        println!("Could not create a new project \"{}\" in the current directory. Ensure that you have the correct permissions and try again.", args[2]);
        exit(1)
    }

    print_header();

    // Setup project
    create_dir(path.join(".wisteria/")).unwrap();
    write(
        format!("{}/.wisteria/metadata.toml", args[2]),
        template::WISTERIA_METADATA_TEMPLATE,
    )
    .unwrap();
    write(
        format!("{}/project.toml", args[2]),
        template::generate_wisteria_project(&args[2], flags.minimal),
    )
    .unwrap();
    create_dir(path.join("src/")).unwrap();
    create_dir(path.join("lib/")).unwrap();

    println!("Operation complete! You should now open {}/project.toml in your favorite text editor to tweak the project to suit your needs.", args[2]);
    exit(0)
}

/// Takes the arguments passed into the program and turn them into arguments and flags
fn load_arguments(args: &mut Vec<String>) -> StartupFlags {
    // Read args
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

/// Given a set of dependencies, attempt to update and resolve their filepaths.
fn update_dependencies(
    targets: &[String],
    dependencies: &HashMap<String, Dependency>,
    environment: &HashMap<String, String>,
    regexes: &HashMap<&str, Regex>,
) -> Vec<(String, String)> {
    let mut width: usize = usize::MIN;
    for name in targets.iter() {
        width = usize::max(name.len(), width);
    }

    width += 5;

    let mut failed_downloads: Vec<(String, String)> = Vec::new();
    let size = targets.len();
    for (index, target) in targets.iter().enumerate() {
        if let Some((name, dep)) = dependencies.get_key_value(target) {
            print!(
                "({}/{size}) Updating {:width$}",
                index + 1,
                format!("{name} ... ")
            );
            let _ = match dep.resolve(name, environment, regexes, UpdateContext::Update) {
                Ok(p) => p,
                Err(e) => {
                    println!("Could not download {name}: {}", e.0);
                    failed_downloads.push((name.clone(), e.0));
                    continue;
                }
            };
        }
    }

    failed_downloads
}

/// Does what it says on the tin. Prints the copyright header.
fn print_header() {
    println!("Wisteria v{VERSION}");
    println!("Copyright © 2025 Hailey-Jane \"Khyonie\" Veverka <http://www.khyonieheart.coffee/>");
}
