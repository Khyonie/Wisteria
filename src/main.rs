/*
 * Wisteria ~ Java Project Manager
 * Copyright (C) 2024  Hailey-Jane "Khyonie" Garrett <http://www.khyonieheart.coffee>
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::{env, fs, path::PathBuf, process::exit, time::{SystemTime, UNIX_EPOCH}};

use builder::BuildInformation;
use java::MajorVersion;
use project::Project;
use rand::Rng;
use task::Task;

use crate::metadata::Metadata;

mod utilities;
mod task;
mod project;
mod source;
mod java;
mod builder;
mod template;
mod compatibility;
mod metadata;
mod gpl;

const NO_OPERATION_ERROR: &str = 
r#"Usage: wisteria [< build | run | update | tasks | new | init | convert | help >]
    build <task>
        Builds the project with the specified task 
    run <task> 
        Builds the project with the specified task and executes the resulting jar
    update 
        Updates dependencies
    tasks 
        Lists all defined tasks
    new <name> <package> 
        Creates a new Wisteria-compatible Java project with the given name
    init 
        Creates a Wisteria project configuration file for an existing project
    convert
        Converts a Wisteria 1.x.x project configuration into a Wisteria 2.x.x project configuration
    help 
        Displays this help dialogue
Options: 
    --silent, -s
        Suppresses all output, including the Wisteria header (--noheader)
    --debug, -d
        Outputs debugging information
    --noheader, -h
        Skips printing the Wisteria header
    --bland, -b 
        Suppresses printing the fun flavor text when a project has finished building
    --dry, -dr  (Only for "init" and "convert")
        Skips writing an updated project.toml to disk
    --recompile-all, -r (Only for "build" and "run")
        Compile all source files, not just source files that changed since last compilation
    --jdk <path to Java home>, -j <path to Java home> 
        Uses a specific JDK instead of a default JDK found through the user environment
    --use-manifest, -m
        Uses a META-INF/MANIFEST.MF file in this project instead of creating one
"#;

const HEADER: &str =
r#"Wisteria v2.1.5 ~ Java Project Manager
Copyright (C) 2024  Hailey-Jane "Khyonie" Garrett <http://www.khyonieheart.coffee>

This program comes with ABSOLUTELY NO WARRANTY; for details type `wisteria warranty'.
This is free software, and you are welcome to redistribute it
under certain conditions; type `wisteria copying' for details.
-"#;

enum Operation
{
    BuildProject{ task: String },
    RunProject{ task: String },
    UpdateDependencies{ task: String },
    ListTasks,
    CreateNewProject{ name: String, package: String },
    InitializeExistingProject,
    ConvertExistingProject
}

pub struct Flags
{
    debug: bool,
    silent: bool,
    no_header: bool,
    bland: bool,
    dry: bool,
    recompile_all: bool,
    use_existing_manifest: bool,
    use_java_executable: Option<String>
}

fn main() 
{
    let mut arguments: Vec<String> = env::args()
        .collect();

    let mut flags: Flags = Flags { debug: false, silent: false, no_header: false, bland: false, dry: false, recompile_all: false, use_existing_manifest: false, use_java_executable: None };

    let mut arg_iter = arguments.iter();
    let mut wisteria_args: Vec<String> = Vec::new();
    let mut runtime_args: Vec<String> = Vec::new();
    wisteria_args.push(arg_iter.next().unwrap().to_string());
    while let Some(a) = arg_iter.next()
    {
        if a == "--"
        {
            while let Some(arg) = arg_iter.next()
            {
                runtime_args.push(arg.to_string());
            }
            break;
        }
        match a.to_lowercase().as_str()
        {
            "--silent" | "-s" => flags.silent = true,
            "--debug" | "-d" => flags.debug = true,
            "--noheader" | "-h" => flags.no_header = true,
            "--jdk" | "-j" => {
                if let Some(version) = arg_iter.next()
                {
                    if !PathBuf::from(version).exists()
                    {
                        debugln!(flags, "Java executable does not point to any file, aborting...");
                        exit(1);
                    }

                    flags.use_java_executable = Some(version.to_string());
                    silentln!(flags, "Using custom Java home path {}", version.to_string());
                } else {
                    silentln!(flags, "Java executable flag given with no filepath, aborting...");
                    exit(1);
                }
            },
            "--bland" | "-b" => flags.bland = true,
            "--dry" | "-dr" => flags.dry = true,
            "--recompile-all" | "-r" => flags.recompile_all = true, 
            "--use-manifest" | "-m" => flags.use_existing_manifest = true,
            _ => {
                wisteria_args.push(a.to_string());
            }
        }
    }

    arguments = wisteria_args;

    if arguments.len() == 1
    {
        if !flags.silent
        {
            if !flags.no_header
            {
                println!("{}", HEADER);
            }
            println!("No operation given.");
            println!("{}", NO_OPERATION_ERROR);
        }
        exit(2)
    }

    // License details take priority
    match arguments.get(1).unwrap().to_lowercase().as_str()
    {
        "warranty" => {
            println!("{}", gpl::WARRANTY_TEXT);
            exit(0);
        }
        "copying" => {
            println!("{}", gpl::REDISTRIBUTING_TEXT);
            exit(0);
        }
        _ => {}
    }

    if !flags.no_header
    {
        println!("{}", HEADER);
    }

    // Attempt to find javac
    let mut java_version = java::MajorVersion { version: 0 };
    silentln!(flags, "Java version: {}", java::get_java_version(&flags, &mut java_version));

    // Create an operation for us to perform
    let operation: Operation = match arguments.get(1).unwrap_or(&String::from("")).to_lowercase().as_str()
    {
        // Build project
        //-------------------------------------------------------------------------------- 
        "build" => {
            let task: String = arguments.get(2)
                .unwrap_or(&String::from("(DEFAULT)"))
                .to_string();

            Operation::BuildProject { task }
        }

        // Run project
        //-------------------------------------------------------------------------------- 
        "execute" | "run" => {
            let task: String = arguments.get(2)
                .unwrap_or(&String::from("(DEFAULT)"))
                .to_string();

            Operation::RunProject { task }
        }
        "listtasks" | "tasks" => {
            Operation::ListTasks
        }

        // Update dependencies
        //-------------------------------------------------------------------------------- 
        "update" | "dependencies" => Operation::UpdateDependencies { task: arguments.get(2).unwrap_or(&String::from("default")).to_string() },

        // Create new project
        //-------------------------------------------------------------------------------- 
        "create" | "new" if arguments.len() >= 4 => Operation::CreateNewProject { name: arguments.get(2).unwrap().to_string(), package: arguments.get(3).unwrap().to_string() },
        "create" | "new" => {
            if !flags.silent
            {
                println!("Not enough arguments.");
                println!("\tUsage: wisteria new <name> <package>");
            }
            exit(2)
        }

        // Initialize existing project
        //-------------------------------------------------------------------------------- 
        "init" => Operation::InitializeExistingProject,

        //-------------------------------------------------------------------------------- 
        "help" => {
            println!("{}", NO_OPERATION_ERROR);
            exit(0);
        }
        "convert" => Operation::ConvertExistingProject,
        _ => {
            if !flags.silent
            {
                println!("Unknown operation.");
                println!("{}", NO_OPERATION_ERROR);
            }
            exit(2)
        }
    };

    // Run operation
    match operation
    {
        Operation::BuildProject { task } => {
            let metadata = metadata::load_metadata(&flags);

            let project: Project = match project::read_project(&flags) {
                Ok(p) => p,
                Err(e) => {
                    silentln!(flags, "Could not parse project.toml. Error: {}", e);
                    exit(0b1111_0001)
                }
            };

            let task: String = match task.as_str()
            {
                "(DEFAULT)" => project.default_task_name().to_string(),
                _ => task
            };

            let last_built_task = task.clone();

            let task = validate_task(&project, &task, &java_version, &flags);
            let build_information = BuildInformation::from(&task, &project, &metadata, &flags);

            // Build
            let targets: Vec<String>;
            let elapsed_time: u128;
            match builder::build_task(&task, &project, &build_information, &runtime_args, &metadata, &flags) 
            {
                (t, compilation_time) => {
                    targets = t;
                    elapsed_time = compilation_time;
                }
            }

            // Calculate average compilation times
            let mut compilation_times: Vec<u32> = Vec::from(metadata.compilation_times());
            if compilation_times.len() == 30
            {
                compilation_times.remove(0);
            }
            if elapsed_time > 10
            {
                compilation_times.push(elapsed_time as u32);
            } else {
                silentln!(flags, "Skipping registering this compilation time, as no files were compiled");
            }
            let mut average: u32 = 0;
            for time in &compilation_times
            {
                average += time;
            }
            average /= compilation_times.len() as u32;
        
            let hash = builder::package(&task, &project, &targets, &flags);

            if let Some(h) = hash
            {
                if !flags.silent
                {
                    println!("Project hash: #{}", h);
                    println!("Successfully built and packaged task {} in {} ms (average of last {} compilations: {} ms)", &task.get_name(), elapsed_time, &compilation_times.len(), average);
                }
            }

            // Write metadata
            let new_metadata = Metadata::from(last_built_task, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64, compilation_times);
            let metadata_string: String = toml::to_string(&new_metadata.to_table()).unwrap();

            match fs::write(".wisteria", metadata_string) {
                Ok(_) => debugln!(flags, "Wrote updated wisteria metadata"),
                Err(e) => silentln!(flags, "Failed to write updated metadata, this isn't a fatal error. Error: {}", e)
            }

            if !flags.bland && !flags.silent
            {
                let flavor = template::FLAVOR_TEXT[rand::thread_rng().gen_range(0..template::FLAVOR_TEXT.len())];
                println!("{}", flavor);
            }
        },
        Operation::RunProject { task } => {
            let metadata = metadata::load_metadata(&flags);

            let project: Project = match project::read_project(&flags) {
                Ok(p) => p,
                Err(e) => {
                    silentln!(flags, "Could not parse project.toml. Error: {}", e);
                    exit(1)
                }
            };

            let task: String = match task.as_str()
            {
                "(DEFAULT)" => project.default_task_name().to_string(),
                _ => task
            };

            let last_built_task = task.clone();

            let task = validate_task(&project, &task, &java_version, &flags);
            if task.get_entry().is_none()
            {
                silentln!(flags, "No entry point has been defined for task {} in project {}. Configure one in your project.toml with your favorite text editor and try again. Aborting...", task.get_name(), project.get_name());
                exit(3);
            }
            let build_information = BuildInformation::from(&task, &project, &metadata, &flags);


            let targets: Vec<String>;
            let elapsed_time: u128;
            match builder::build_task(&task, &project, &build_information, &runtime_args, &metadata, &flags) {
                (t, compilation_time) => {
                    targets = t;
                    elapsed_time = compilation_time;
                }
            }
            
            // Calculate average compilation times
            let mut compilation_times: Vec<u32> = Vec::from(metadata.compilation_times());
            if compilation_times.len() == 30
            {
                compilation_times.remove(0);
            }
            if elapsed_time > 10 
            {
                compilation_times.push(elapsed_time as u32);
            } else {
                silentln!(flags, "Skipping registering this compilation time, as no files were compiled");
            }
            let mut average: u32 = 0;
            for time in &compilation_times
            {
                average += time;
            }
            average /= compilation_times.len() as u32;

            let hash = builder::package(&task, &project, &targets, &flags);
            if !flags.use_existing_manifest
            {
                source::clean_manifest(&flags);
            }

            if let Some(h) = hash
            {
                silentln!(flags, "Project hash: #{}", h);
                silentln!(flags, "Successfully built task {} in {} ms (average of last {} compilations: {} ms), running now...", &task.get_name(), elapsed_time, &compilation_times.len(), average);
            }

            // Write metadata
            let new_metadata = Metadata::from(last_built_task, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64, compilation_times);
            let metadata_string: String = toml::to_string(&new_metadata.to_table()).unwrap();

            match fs::write(".wisteria", metadata_string) {
                Ok(_) => debugln!(flags, "Wrote updated wisteria metadata"),
                Err(e) => silentln!(flags, "Failed to write updated metadata, this isn't a fatal error. Error: {}", e)
            }

            if !flags.bland && !flags.silent
            {
                let flavor = template::FLAVOR_TEXT[rand::thread_rng().gen_range(0..template::FLAVOR_TEXT.len())];
                println!("{}", flavor);
            }

            builder::run_target(&targets[0], runtime_args, &flags);
        }
        Operation::ListTasks => {
            let project: Project = match project::read_project(&flags) {
                Ok(p) => p,
                Err(e) => {
                    silentln!(flags, "Could not parse project.toml. Error: {}", e);
                    exit(1)
                }
            };
            println!("{} task(s) in project {}", &project.get_tasks().len(), &project.get_name());
            for task in project.get_tasks().values()
            {
                task.print_information();
            }
        }
        Operation::CreateNewProject { name, package } => {
            // Create source
            let mut source_path = format!("{}/src/{}/", &name, &package.replace(".", "/"));
            match fs::create_dir_all(&source_path)
            {
                Ok(_) => debugln!(flags, "Created new source tree {}", &source_path),
                Err(e) => {
                    silentln!(flags, "Could not create source tree. Check that you have permission to write to this directory and try again, error: {}", e);
                    exit(1);
                }
            }

            let project: Project = project::generate_template_project(&name);

            source_path.push_str(format!("{}.java", &name).as_str());

            // We should have write permissions if we haven't failed by now
            fs::write(&source_path, template::generate_starter(&project, &package)).unwrap();
            debugln!(flags, "Created new Java source file at {}", &source_path);

            fs::create_dir(format!("{}/lib/", &name)).unwrap();
            debugln!(flags, "Created new libraries folder");

            fs::create_dir(format!("{}/.settings/", &name)).unwrap();
            debugln!(flags, "Created new Eclipse .settings folder");
            
            // Write files
            fs::write(format!("{}/.settings/org.eclipse.jdt.core.pref", &name), &template::generate_eclipse_core_perfs(&project.default_task().unwrap())).unwrap();
            debugln!(flags, "Created new Eclipse core preferences file");

            fs::write(format!("{}/.settings/org.eclipse.m2e.core.pref", &name), &template::generate_maven_core_prefs()).unwrap();
            debugln!(flags, "Created new Maven2Eclipse preferences file");

            fs::write(format!("{}/project.toml", &name), &template::generate_project_toml(&project)).unwrap();
            debugln!(flags, "Created new Wisteria project configuration");

            fs::write(format!("{}/.project", &name), &template::generate_project(&project)).unwrap();
            debugln!(flags, "Created new Eclipse .project file");

            fs::write(format!("{}/.classpath", &name), &template::generate_classpath(&project.default_task().unwrap(), &flags)).unwrap();
            debugln!(flags, "Created new Eclipse .classpath file");

            fs::write(format!("{}/pom.xml", &name), &template::generate_pom_xml(&project, &package)).unwrap();
            debugln!(flags, "Created new Maven project configuration");
            
            silentln!(flags, "Created new Wisteria project. You should modify the generated project.toml to suit your exact project requirements.");

            if !flags.bland && !flags.silent
            {
                let flavor = template::FLAVOR_TEXT[rand::thread_rng().gen_range(0..template::FLAVOR_TEXT.len())];
                println!("{}", flavor);
            }
        }
        Operation::UpdateDependencies{ task } => {
            // TODO This should update other applicable files other than just .classpath
            debugln!(flags, "# Reading project");
            let project: Project = match project::read_project(&flags) {
                Ok(p) => p,
                Err(e) => {
                    silentln!(flags, "Could not parse project.toml. Error: {}", e);
                    exit(1)
                }
            };

            debugln!(flags, "# Pulling default or specified task, if applicable");
            let task = match task.as_str() {
                "default" => match project.default_task() {
                    Some(t) => t,
                    None => {
                        silentln!(flags, "Tried to update project dependencies, but project {} does not specify a default task and no task was specified. Create a default task in your project.toml or specify an existing task.", project.get_name());
                        exit(2);
                    }
                },
                _ => match project.get_tasks().get(&task) {
                    Some(t) => t,
                    None => {
                        silentln!(flags, "No such task named {} exists in project {}. Aborting...", &task, &project.get_name());
                        exit(2);
                    }
                }
            };

            if flags.debug
            {
                println!("Libraries: {} total", task.get_relative_libraries().len());
                for lib in task.get_relative_libraries()
                {
                    println!("\t{}", lib);
                }
            }

            fs::write(".classpath", &template::generate_classpath(task, &flags)).unwrap();
            silentln!(flags, "Operation complete!");
        }
        Operation::ConvertExistingProject => {
            compatibility::update_wisteria_1_project(&flags);
            silentln!(flags, "Operation complete! A backup of the Wisteria 1.x.x project.toml has been saved to project.toml.bak.");
            silentln!(flags, "NOTE: Some task options have moved, you should check the new project.toml to correct any issues.");
            exit(0);
        }
        Operation::InitializeExistingProject => {
            compatibility::load_eclipse_project(&flags);
            silentln!(flags, "Operation complete! You should open the new project.toml in your favorite text editor to correct any issues.");
            exit(0);
        }
    }
}

fn validate_task(project: &Project, task_name: &String, java_version: &MajorVersion, flags: &Flags) -> Task
{
    let task = match task::get_task(task_name, &project) {
        Some(t) => t,
        None => {
            silentln!(flags, "No such task named {}. Aborting...", task_name);
            exit(2);
        }
    };

    match task.can_build(java_version)
    {
        Ok(_) => {},
        Err(msg) => {
            silentln!(flags, "{}, aborting...", msg);
            exit(2);
        }
    }
    
    task
}
