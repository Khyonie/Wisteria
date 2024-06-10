use std::{env, ffi::OsStr, fs::{self, File}, path::PathBuf, process::exit, time::{Duration, UNIX_EPOCH}};

use zip::ZipArchive;

use crate::{debugln, java, metadata::Metadata, project::Project, silentln, task::Task, template, Flags};

pub fn collect_files_relative(directory: PathBuf, filetype: &str, sources: &mut Vec<String>, flags: &Flags)
{
    if directory.is_file()
    {
        if directory.to_string_lossy().ends_with(filetype)
        {
            sources.push(String::from(directory.to_string_lossy()));
        }

        return;
    }

    match directory.read_dir()
    {
        Ok(entries) => {
            for e in entries 
            {
                match e
                {
                    Ok(entry) => {
                        if entry.path().is_dir()
                        {
                            
                            collect_files_relative(entry.path(), filetype, sources, flags);
                            continue;
                        }

                        if entry.path().to_string_lossy().ends_with(filetype)
                        {
                            if !entry.path().is_file()
                            {
                                continue
                            }

                            sources.push(String::from(entry.path().to_string_lossy()));
                        }
                    },
                    Err(err) => {
                        silentln!(flags, "Could not read file, error: {}, skipping...", err);
                        continue;
                    } 
                }
            }
        },
        Err(e) => {
            silentln!(flags, "Could not read directory {}. Error: {}, skipping...", directory.to_string_lossy(), e);
        }
    }
}

pub fn collect_files(directory: PathBuf, filetype: &str, sources: &mut Vec<String>, task: &Task, project: &Project, flags: &Flags)
{
    collect_files_relative(directory, filetype, sources, flags);

    let mut new_sources: Vec<String> = sources.iter()
        .map(| s | canonicalize_path(s.clone(), task, project, flags))
        .collect();

    sources.clear();
    sources.append(&mut new_sources);
}

/// Canonicalizes the given path, and resolves ~ to the user's home directory.
pub fn canonicalize_path(path_string: String, task: &Task, project: &Project, flags: &Flags) -> String
{
    debugln!(flags, "Attempting to resolve path {}", &path_string);

    let mut canon_path: String = path_string.clone();
    if path_string.contains("~")
    {
        match env::consts::OS
        {
            "macos" | "linux" => {
                canon_path = canon_path.replace("~", &env::var_os("HOME").unwrap().to_string_lossy());
            }
            "windows" => {
                canon_path = canon_path.replace("~", &env::var_os("HOMEPATH").unwrap().to_string_lossy());
            }
            _ => {
                silentln!(flags, "Unrecognized OS. Home directory resolution is unavailable.");
            }
        }
    }

    let working_directory = env::current_dir().unwrap();

    canon_path = canon_path.replace("{PROJECT_NAME}", &project.get_name())
        .replace("{TASK_NAME}",  &task.get_name())
        .replace("./", "");

    let current_path = working_directory.join(PathBuf::from(&canon_path));

    if flags.debug
    {
        println!("\tFull path: {}", &current_path.to_string_lossy());
        println!("\tFile: {}", &current_path.file_name().unwrap_or(&OsStr::new("(Is directory)")).to_string_lossy());
        println!("\tParents: {}", &current_path.parent().unwrap().to_string_lossy());
    }

    debugln!(flags, "Canonicalized {} into {}", &path_string, &current_path.to_string_lossy());

    current_path.to_string_lossy().to_string()
}

pub fn generate_source(task: &Task, project: &Project, metadata: &Metadata, flags: &Flags) -> (String, String)
{
    let platform_seperator: char = java::get_seperator();
    let mut source_files: String = String::new();
    let mut source_paths: String = String::new();

    // Build sources
    let mut is_first_source: bool = true;
    for source in task.get_sources()
    {
        if is_first_source
        {
            is_first_source = false;
        } else {
            source_files.push(platform_seperator);
        }
        source_paths.push_str(&source);
        source_paths.push(' ');
        let source_path = PathBuf::from(source);
        let mut sources = Vec::new();
        collect_files(source_path, ".java", &mut sources, task, project, flags);

        // Timing
        let compilation_time = Duration::from_millis(metadata.last_compilation_time() as u64);
        let task_matches = match metadata.last_compiled_task() {
            Some(s) => s == task.get_name(),
            None => false
        };

        for s in sources
        {
            if !flags.recompile_all && task_matches
            {
                if let Ok(meta) = PathBuf::from(&s).metadata() 
                {
                    if let Ok(time) = meta.modified()
                    {
                        if time.duration_since(UNIX_EPOCH).unwrap().lt(&compilation_time)
                        {
                            debugln!(flags, "Skipping compilation of {}, since it wasn't modified since task was last compiled", s);
                            continue;
                        }
                    }
                }
            }

            source_files.push_str(&s);
            source_files.push(platform_seperator);
        }

        source_files.pop();
    }
    source_paths.pop();

    (source_paths, source_files)
}

pub fn generate_libraries(task: &Task, project: &Project, flags: &Flags) -> String 
{
    let platform_seperator: char = java::get_seperator();

    let mut libraries: Vec<String> = Vec::new();
    let mut libraries_string: String = String::new();
    if let Some(libs) = task.get_libraries()
    {
        for lib in libs
        {
            let path: PathBuf = PathBuf::from(lib);
            collect_files(path, ".jar", &mut libraries, task, project, &flags);
        }

        for lib in &libraries
        {
            libraries_string.push_str(lib);
            libraries_string.push(platform_seperator);
        }
        libraries_string.pop();
    }

    libraries_string
}

pub fn generate_classpath(task: &Task, flags: &Flags) -> String
{
    let mut classpath = String::new();
    for lib in task.get_relative_libraries()
    {
        let mut libs: Vec<String> = Vec::new();
        collect_files_relative(PathBuf::from(lib), ".jar", &mut libs, flags);
        for file in libs
        {
            classpath.push_str(&file);
            classpath.push(' ')
        }
    }

    classpath.pop();

    debugln!(flags, "Classpath: {}", &classpath);

    classpath
}

pub fn generate_classpath_exclude_shaded(task: &Task, flags: &Flags) -> String
{
    let mut classpath = generate_classpath(task, flags);

    if let Some(jars) = task.get_shaded_jars()
    {
        for j in jars
        {
            classpath = classpath.replace(&format!("{} ", j), "");
        }
    }

    classpath.replace("  ", " ")
}

pub fn generate_inputs(task: &Task) -> String
{
    let mut includes: String = String::new();

    if let Some(inputs) = task.get_includes()
    {
        for include in inputs
        {
            includes.push_str(include);
            includes.push(' ');
        }

        includes.pop();
    }

    includes
}

pub fn extract_shaded_jars(task: &Task, flags: &Flags)
{
    if let Some(jars) = task.get_shaded_jars()
    {
        debugln!(flags, "Attempting to extract shaded jars");
        if !PathBuf::from("target/shaded-classes/").exists()
        {
            match fs::create_dir_all("target/shaded-classes/") {
                Ok(_) => debugln!(flags, "Created shaded classes folder"),
                Err(e) => {
                    silentln!(flags, "Could not create directory for shaded classes. Error: {}, aborting...", e);
                    exit(4);
                }
            }
        } else {
            debugln!(flags, "Cleaning shaded classes folder");
            clean_shaded_classes(flags);
        }

        for j in jars
        {
            debugln!(flags, "Extracting .jar file {} for shading", j);
            let file = match File::open(j) {
                Ok(f) => f,
                Err(e) => {
                    silentln!(flags, "Could not open .jar file {} for shading. Error: {}, aborting...", j, e);
                    exit(2);
                }
            };

            let mut archive = match ZipArchive::new(file) {
                Ok(a) => a,
                Err(e) => {
                    silentln!(flags, "Could not open file {}, error: {}. Aborting...", j, e);
                    exit(3);
                }
            };
            debugln!(flags, "Successfully opened .jar file {}", j);

            match archive.extract("target/shaded-classes/") {
                Ok(_) => debugln!(flags, "Successfully extracted .jar file {}", j),
                Err(e) => {
                    silentln!(flags, "Could not extract {} into target/shaded-classes/, error: {}. Aborting...", j, e);
                    exit(5);
                }
            }
        }
    }
}

pub fn clean_shaded_classes(flags: &Flags)
{
    if PathBuf::from("target/shaded-classes/").exists()
    {
        match fs::remove_dir_all("target/shaded-classes/")
        {
            Ok(_) => (),
            Err(e) => {
                silentln!(flags, "Could not removes shaded classes folder, this is not a fatal error. Error: {}", e);
            }
        }
    }
}

pub fn generate_manifest(task: &Task, flags: &Flags)
{
    let manifest_folder = PathBuf::from("META-INF/");
    if !manifest_folder.exists()
    {
        match fs::create_dir(manifest_folder)
        {
            Ok(_) => (),
            Err(e) => {
                silentln!(flags, "Could not create META-INF/ folder, error: {}", e);
                exit(4);
            }
        }

        let manifest = template::generate_manifest(task, flags);

        match fs::write("META-INF/MANIFEST.MF", &manifest)
        {
            Ok(_) => (),
            Err(e) => {
                clean_manifest(flags);
                silentln!(flags, "Could not write to MANIFEST.MF file, error: {}", e);
                exit(4)
            }
        }

        debugln!(flags, "MANIFEST.MF created");
    }
}

pub fn clean_manifest(flags: &Flags)
{
    match fs::remove_dir_all("./META-INF/")
    {
        Ok(_) => {},
        Err(e) => {
            silentln!(flags, "Could not remove Manifest output folder. Error: {}", e);
            exit(3);
        }
    }
}
