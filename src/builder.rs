use std::{fs, path::PathBuf, process::{exit, Command}, time::SystemTime};

use crate::{debugln, java, metadata::Metadata, project::Project, silentln, source, task::Task, Flags};

pub struct BuildInformation
{
    source_paths: String,
    source_files: String,
    libraries: Option<String>
}

impl BuildInformation 
{
    pub fn from(task: &Task, project: &Project, metadata: &Metadata, flags: &Flags) -> Self
    {
        debugln!(flags, "Collecting sources...");

        let sources = source::generate_source(&task, project, metadata, flags);
        debugln!(flags, "Sources: {}", &sources.0);

        // Build libraries

        let mut libraries: Option<String> = None;
        if task.get_libraries().is_some()
        {
            debugln!(flags, "Collecting libraries...");
            libraries = Some(source::generate_libraries(&task, project, flags));

            debugln!(flags, "Libraries: {}", libraries.as_ref().unwrap())
        }

        BuildInformation { source_paths: sources.0, source_files: sources.1, libraries }
    }

    pub fn source_paths(&self) -> &str 
    {
        self.source_paths.as_ref()
    }

    pub fn source_files(&self) -> &str 
    {
        self.source_files.as_ref()
    }

    pub fn libraries(&self) -> Option<&String> 
    {
        self.libraries.as_ref()
    }
}

pub fn build_task(task: &Task, project: &Project, information: &BuildInformation, options: &Vec<String>, metadata: &Metadata, flags: &Flags) -> (Vec<String>, u128)
{
    if let Some(last) = metadata.last_compiled_task()
    {
        if last != task.get_name()
        {
            clean_binary_folder_noerr(flags);
        }
    }

    if !flags.silent
    {
        println!("Building project {} with task {}", &project.get_name(), task.get_name());
        if let Some(desc) = task.get_description()
        {
            println!("\tTask description: {}", desc);
        }
    }

    // Build command
    let java_executable = match flags.use_java_executable.as_ref()
    {
        Some(j) => format!("{}/bin/javac", j).replace("//", "/"),
        None => String::from("javac")
    };

    let mut compile_command = Command::new(java_executable);
    compile_command.args(["-d", "./bin/"]);
    compile_command.args(["--source-path", information.source_paths()]);

    if let Some(libraries) = information.libraries()
    {
        compile_command.args(["--class-path", libraries]);
    }

    for flag in task.compiler_options().to_compiler_arguments()
    {
        compile_command.arg(flag);
    }

    debugln!(flags, "Compiler options: {:?}", task.compiler_options().to_compiler_arguments());

    for option in options
    {
        compile_command.arg(option);
    }

    let platform_seperator = java::get_seperator();

    let mut targets = Vec::new();
    for target in task.output()
    {
        targets.push(target.to_string());
    }

    if information.source_files().is_empty() 
    {
        silentln!(flags, "No source files modified since last compilation, only repackaging targets. To compile anyways, add a \"--recompile-all\" flag to your wisteria command.");
        return (targets, 0u128);
    }

    for s in information.source_files().split(platform_seperator)
    {
        compile_command.arg(s);
    }

    debugln!(flags, "Running Javac command {:?}", compile_command);

    // Build 
    debugln!(flags, "Compiling project...");

    let now = SystemTime::now();
    match compile_command.output()
    {
        Ok(out) => {
            if !out.stdout.is_empty()
            {
                println!("{}", String::from_utf8(out.stdout).unwrap());
            }

            if !out.stderr.is_empty()
            {
                let stderr = String::from_utf8(out.stderr).unwrap();
                println!("{}", &stderr);
                if !stderr.starts_with("Note: ")
                {
                    clean_binary_folder_noerr(flags);
                    exit(3);
                }
            }
        },
        Err(e) => {
            clean_binary_folder_noerr(flags);
            silentln!(flags, "Could not compile project. Error: {}", e);
            exit(3);
        }
    }

    let duration = SystemTime::now().duration_since(now).unwrap().as_millis();

    (targets, duration)
}

pub fn package(task: &Task, project: &Project, targets: &Vec<String>, flags: &Flags) -> Option<String>
{ 
    let mut hash: Option<String> = None;
    let mut includes = source::generate_inputs(&task);

    silentln!(flags, "Packaging ({}) target(s) (Task {} in {})", &targets.len(), &task.get_name(), &project.get_name());

    if task.get_entry().is_some()
    {
        debugln!(flags, "Creating MANIFEST.MF");
        if !flags.use_existing_manifest
        {
            source::generate_manifest(task, flags);
        }

        if !includes.is_empty()
        {
            includes.push(' ');
        }
        includes.push_str("META-INF");
    }

    debugln!(flags, "Includes: {}", &includes);

    if task.get_shaded_jars().is_some()
    {
        source::extract_shaded_jars(task, flags);
    }

    for target in targets
    {
        let target_path = PathBuf::from(target);
        debugln!(flags, "Packaging target {}", &target);

        match target_path.parent()
        {
            Some(p) if !p.exists() => {
                match fs::create_dir_all(p)
                {
                    Ok(_) => (),
                    Err(e) => {
                        silentln!(flags, "Could not create parent directories for target {}, error: {}", target, e);
                        exit(4);
                    }
                }
            },
            Some(_) => debugln!(flags, "Parent directories exist for target"),
            None => {
                match fs::create_dir_all(target)
                {
                    Ok(_) => (),
                    Err(e) => {
                        silentln!(flags, "Could not create parent directories for target {}, error: {}", target, e);
                        exit(4);
                    }
                }
            }
        }

        if target_path.exists()
        {
            match fs::remove_file(&target_path)
            {
                Ok(_) => {
                    debugln!(flags, "Removed existing target");
                }
                Err(e) => {
                    silentln!(flags, "Could not remove existing target {}, error: {}", target_path.to_string_lossy(), e);
                    continue;
                }
            }
        }

        let jar_executable = match flags.use_java_executable.as_ref()
        {
            Some(j) => format!("{}/bin/jar", j).replace("//", "/"),
            None => String::from("jar")
        };

        let mut command = Command::new(jar_executable);
        command.arg("-cMf")
            .arg(&target);

        if !includes.is_empty()
        {
            debugln!(flags, "Included in package: {}", &includes);

            for i in includes.split(" ")
            {
                command.arg(i);
            }
        }

        command.args(["-C", "bin/", "."]);

        debugln!(flags, "Running jar command {:?}", &command);

        match command.output()
        {
            Ok(out) => {
                let output = String::from_utf8(out.stdout).unwrap();
                let errors = String::from_utf8(out.stderr).unwrap();
                if !output.is_empty()
                {
                    silentln!(flags, "{}", output);
                }
                if !errors.is_empty()
                {
                    silentln!(flags, "{}", errors);
                }

                silentln!(flags, "Packaged {}", &target);

                if hash.is_none()
                {
                    let mut hash_command = Command::new("sha256sum");
                    hash_command.arg(target);

                    match hash_command.output()
                    {
                        Ok(output) => {
                            let hash_string = String::from_utf8(output.stdout).unwrap();

                            hash = Some(
                                hash_string.split(' ')
                                    .collect::<Vec<&str>>()[0].to_string()
                            );
                        }
                        Err(e) => {
                            silentln!(flags, "Could not generate hash for target {} ({}), continuing without hashes...", &target, e);
                            hash = Some(String::new())
                        }
                    }
                }
            },
            Err(e) => {
                silentln!(flags, "Could not package {}, error: {}", target, e);
                exit(4);
            }
        }

        // Package shaded
        if task.get_shaded_jars().is_some()
        {
            // Shade
            let mut command = Command::new("jar");
            command.args(["-uf", target, "-C", "target/shaded-classes/", "."]);
        }
    }

    source::clean_shaded_classes(flags);

    hash
}

pub fn run_target(target: &String, options: Vec<String>, flags: &Flags)
{
    clean_binary_folder(flags);
    let java_executable = match flags.use_java_executable.as_ref()
    {
        Some(s) => format!("{}/bin/java", s).replace("//", "/"),
        None => String::from("java")
    };

    let mut command = Command::new(&java_executable);
    command.args(["-jar", target]);

    for option in options
    {
        command.arg(option);
    }

    let mut child_process = match command.spawn()
    {
        Ok(child) => child,
        Err(e) => {
            silentln!(flags, "Could not execute target {}, error: {}", target, e);
            exit(-1);
        }
    };


    let exit_code = match child_process.wait()
    {
        Ok(e) => e,
        Err(err) => {
            silentln!(flags, "Failed to wait for target {} to exit, error: {}", target, err);
            exit(-2);
        }
    };

    silentln!(flags, "Target {} exited with status code {}", target, exit_code);
    match exit_code.code()
    {
        Some(c) => exit(c),
        None => exit(130)
    }
}

pub fn clean_binary_folder(flags: &Flags)
{
    debugln!(flags, "Removing binary output folder");
    match fs::remove_dir_all("./bin/")
    {
        Ok(_) => {},
        Err(e) => {
            silentln!(flags, "Could not remove binary output folder. Error: {}", e);
            exit(3);
        }
    }
}

pub fn clean_binary_folder_noerr(flags: &Flags)
{
    debugln!(flags, "Silently removing binary output folder");

    match fs::remove_dir_all("./bin/")
    {
        Ok(_) => {},
        Err(_) => {}
    }
}
