use std::{collections::HashMap, fs::{self, File}, path::PathBuf, process::Command};

use regex::Regex;
use sha256::digest;
use toml::{map::Map, Value};
use zip::ZipArchive;

use crate::{configuration::Configuration, dependency::UpdateContext, java::manifest::{Manifest, ManifestEntry}, project::{Project, ProjectInfo}, util::{consts, files::{self, resolve_filepath}}};

#[derive(Clone)]
pub struct DefinedTask
{
    name: String,
    phases: HashMap<String, Vec<String>>,
    phase_order: Vec<String>,
    on_fail: Option<String>,
    chain_task: Option<Vec<String>>
}

pub trait TaskRunner
{
    fn invoke(&self, info: &ProjectInfo, project: &Project, configuration: &Configuration) -> Result<(), (String, u8)>; 

    fn phase_order(&self) -> &[String];
}

//-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=- 
// Base task
//-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=- 

impl DefinedTask 
{
    pub fn new(name: &str, toml: &Map<String, Value>) -> Result<Self, (String, u8)>
    {
        let phases: HashMap<String, Vec<String>> = match toml.get("phase") {
            Some(t) if t.is_table() => {
                let mut phases: HashMap<String, Vec<String>> = HashMap::new();
                for (key, value) in t.as_table().unwrap()
                {
                    match value.as_array()
                    {
                        Some(value) => {
                            let mut phase_components: Vec<String> = Vec::new();

                            for v in value 
                            {
                                match v.as_str()
                                {
                                    Some(s) => phase_components.push(s.to_string()),
                                    None => return Err((format!("Mismatched type for phase element in phase \"{}\", expected a string, found {}", key, v.type_str()), 15))
                                }
                            }

                            phases.insert(key.clone(), phase_components);
                        },
                        None => return Err((format!("Mismatched type for phase \"{}\", expected an array of strings, found {}", key, value.type_str()), 13))
                    }
                }

                phases
            },
            Some(v) => return Err((format!("Mismatched type for phase, expected a table, found {}", v.type_str()), 16)),
            None => return Err((String::from("Missing key \"phase\" which should be a table"), 10))
        };

        let phase_order: Vec<String> = match toml.get("phases") {
            Some(a) if a.is_array() => {
                let mut phase_order: Vec<String> = Vec::new();
                for v in a.as_array().unwrap()
                {
                    match v.as_str()
                    {
                        Some(s) => phase_order.push(s.to_string()),
                        None => {
                            return Err((format!("Mismatched type for phase order element, expected a string, found {}", v.type_str()), 15))
                        }
                    }
                }

                phase_order
            }
            Some(v) => return Err((format!("Mismatched type for phase order, expected an array of strings, found {}", v.type_str()), 13)),
            None => return Err((String::from("Missing key \"phases\", which should be an array of strings"), 10))
        };

        let on_fail: Option<String> = match toml.get("on_fail") {
            Some(v) if v.is_str() => Some(v.as_str().unwrap().to_string()),
            Some(v) => return Err((format!("Mismatched type for on_fail, expected a string, found {}", v.type_str()), 11)),
            None => None
        };

        let chain_task: Option<Vec<String>> = match toml.get("chain_task") {
            Some(v) if v.is_array() => {
                let mut data: Vec<String> = Vec::new();

                for v in v.as_array().unwrap()
                {
                    match v.as_str()
                    {
                        Some(s) => data.push(s.to_string()),
                        None => return Err((format!("Mismatched type for chain task element, expected a string, found {}", v.type_str()), 15))
                    }
                }

                Some(data)
            },
            Some(v) => return Err((format!("Mismatched type for \"chain_task\", expected a string array, found {}", v.type_str()), 13)),
            None => None
        };

        Ok(DefinedTask{ name: name.to_string(), phases, phase_order, on_fail, chain_task })
    }

    pub fn phases(&self) -> &HashMap<String, Vec<String>> 
    {
        &self.phases
    }

    pub fn phase_order(&self) -> &[String] 
    {
        &self.phase_order
    }

    pub fn on_fail(&self) -> Option<&String> 
    {
        self.on_fail.as_ref()
    }

    pub fn chain_task(&self) -> Option<&Vec<String>> 
    {
        self.chain_task.as_ref()
    }

    pub fn combine_task(&self, task: &DefinedTask) -> DefinedTask 
    {
        todo!()
    }
}

impl TaskRunner for DefinedTask
{
    fn invoke(&self, info: &ProjectInfo, project: &Project, configuration: &Configuration) -> Result<(), (String, u8)>
    {
        println!("# Running task {}", self.name);
        for (index, phase) in self.phase_order.iter().enumerate()
        {
            println!("[Phase {}/{}] Running phase {phase}", index + 1, self.phase_order.len());
            let phase_actions = match self.phases.get(phase) {
                Some(a) => a,
                None => return Err((format!("No phase \"{phase}\" has been defined"), 1))
            };

            for a in phase_actions
            {
                // TODO Build a command from actions
            }
        }

        Ok(())
    }

    fn phase_order(&self) -> &[String]
    {
        self.phase_order.as_ref()
    }
}

//-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=- 
// Implicit tasks
//-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=- 

#[derive(Clone)]
pub struct ImplicitBuildTask 
{
    order: Vec<String>
}

#[derive(Clone)]
pub struct ImplicitCleanTask 
{
    order: Vec<String>
}

impl ImplicitBuildTask
{
    pub fn new() -> Self 
    {
        ImplicitBuildTask { order: vec![ String::from("collect"), String::from("compile"), String::from("shade"), String::from("package") ] }
    }
}

impl TaskRunner for ImplicitBuildTask
{
    fn invoke(&self, info: &ProjectInfo, project: &Project, configuration: &Configuration) -> Result<(), (String, u8)> 
    {
        // Build

        let mut regexes: HashMap<&str, Regex> = HashMap::new();
        regexes.insert("envvars", Regex::new(r#"\{(.+?)}"#).unwrap());

        // Update dependencies
        let mut dep_paths: Vec<PathBuf> = Vec::new();
        let mut shaded_jars: Vec<PathBuf> = Vec::new();
        let mut dep_string: Option<String> = None;

        let mut failed_downloads: Vec<(String, String)> = Vec::new();
        if let Some(dependencies) = configuration.dependencies()
        {
            let mut width: usize = usize::MIN;
            for name in dependencies.iter()
            {
                width = usize::max(name.len(), width);
            }

            width += 5;
            let size = dependencies.len();

            for (index, d) in dependencies.iter().enumerate()
            {
                if let Some((name, dep)) = project.dependencies().get_key_value(d)
                {
                    print!("({}/{size}) Updating {:width$}", index + 1, format!("{name} ... "));
                    let mut updated = match dep.resolve(name, configuration.environment(), &regexes, UpdateContext::TaskInvoked)
                    {
                        Ok(p) => p,
                        Err(e) => { 
                            println!("Could not download {name}: {}", e.0);
                            failed_downloads.push((name.clone(), e.0));
                            continue
                        }
                    };

                    if dep.is_shaded(name, configuration).is_some_and(| s | s)
                    {
                        shaded_jars.append(&mut updated.clone());
                    }

                    dep_paths.append(&mut updated);
                }
            }

            if !failed_downloads.is_empty()
            {
                println!("Failed to resolve {} {}:", failed_downloads.len(), { if failed_downloads.len() == 1 { "dependency" } else { "dependencies" } });
                for (name, error) in failed_downloads
                {
                    println!("- {name}: {error}");
                }

                return Err((String::from("Could not resolve all dependencies"), 1))
            }

            println!("Successfully resolved all dependencies!");
            let mut buffer: String = String::new();
            for dep in &dep_paths
            {
                buffer.push_str(&dep.to_string_lossy());
                buffer.push(consts::java_seperator());
            }

            buffer.pop();
            dep_string = Some(buffer);
        }

        // Collect sources
        match configuration.sources()
        {
            Some(sources) => {
                if sources.is_empty()
                {
                    return Err((String::from("No source folders given, nothing to compile"), 1))
                }

                // Copy sources to combined source folder
                let mut copied_files: Vec<String> = Vec::new();

                let _ = fs::remove_dir_all(".wisteria/work/src/");

                for source in sources
                {
                    let files = files::collect_files_with_extension(&PathBuf::from(source), "java");
                    if files.is_empty()
                    {
                        continue;
                    }

                    for f in &files
                    {
                        let copy_path = format!(".wisteria/work/src/{}", f.to_string_lossy().replacen(source, "", 1));
                        let mut path = PathBuf::from(&copy_path);
                        path.pop();
                        fs::create_dir_all(path).unwrap();
                        File::create(&copy_path).unwrap();

                        match fs::copy(f, &copy_path)
                        {
                            Ok(_) => {},
                            Err(e) => {
                                println!("{e}");
                                continue
                            },
                        }

                        copied_files.push(copy_path);
                    }
                }

                let mut javac_command: Command = Command::new("javac");
                javac_command.args(["-d", "./.wisteria/work/bin/"]);
                javac_command.args(["--source-path", ".wisteria/work/src/"]);

                if let Some(deps) = &dep_string
                {
                    javac_command.args(["--class-path", deps]);
                }

                if let Some(flags) = configuration.compiler_flags()
                {
                    for flag in flags 
                    {
                        javac_command.args(flag.get_canon_flag());
                    }
                }

                for file in copied_files
                {
                    javac_command.arg(file);
                }

                println!("Compiling sources");
                match javac_command.output()
                {
                    Ok(out) => {
                        if !out.stdout.is_empty()
                        {
                            println!("{}", String::from_utf8(out.stdout).unwrap());
                        }

                        if !out.stderr.is_empty()
                        {
                            let stderr = String::from_utf8(out.stderr).unwrap();
                            println!("{stderr}");

                            if !stderr.starts_with("Note: ")
                            {
                                return Err((String::from("Could not compile project"), 1))
                            }
                        }
                    }
                    Err(e) => return Err((format!("{e}"), 1))
                }
            }
            None => return Err((String::from("No source folders given, nothing to compile"), 1))
        }
        
        // Shade
        for shaded in &shaded_jars
        {
            let file: File = match File::open(shaded) {
                Ok(f) => f,
                Err(e) => return Err((format!("Failed to open jar {}: {e}", shaded.to_string_lossy()), 1))
            };

            let mut archive = match ZipArchive::new(file) {
                Ok(a) => a,
                Err(e) => return Err((format!("Failed to open jar {}: {e}", shaded.to_string_lossy()), 1))
            };

            let shaded_jar_path = PathBuf::from(".wisteria/work/shaded/");
            if !shaded_jar_path.exists()
            {
                if let Err(e) = fs::create_dir_all(&shaded_jar_path)
                {
                    return Err((format!("Could not create shaded work folder: {e}"), 1))
                }
            }

            if let Err(e) = archive.extract(".wisteria/work/shaded/") 
            {
                return Err((format!("Could not extract {}: {e}", shaded.to_string_lossy()), 1))
            }

            let read = shaded_jar_path.read_dir().unwrap();
            for entry in read.flatten()
            {
                if entry.path().is_file()
                {
                    fs::remove_file(entry.path()).unwrap();
                    continue
                }

                if entry.file_name() == "META-INF"
                {
                    continue
                }

                if files::collect_files_with_extension(&entry.path(), "class").is_empty()
                {
                    fs::remove_dir_all(entry.path()).unwrap();
                    continue
                }

                //if let Err(e) = fs::copy(entry.path(), ".wisteria/work/bin/")
                //{
                //    return Err((format!("Could not copy the contents of {}: {e}", entry.path().to_string_lossy()), 1))
                //}
            }
        }
        
        // Package
    
        // Insert manifest
        let mut manifest: Manifest = Manifest::new();
        manifest.add_entry(ManifestEntry::CreatedBy { signature: String::from("Wisteria 3") });

        if let Some(entry) = configuration.entry()
        {
            manifest.add_entry(ManifestEntry::MainClass { class: entry.clone() })
        }

        if !dep_paths.is_empty()
        {
            let dep_strings: Vec<String> = dep_paths.iter()
                .map(| p | p.to_string_lossy().to_string())
                .collect();

            manifest.add_entry(ManifestEntry::ClassPath { path: dep_strings })
        }

        let manifest_path = PathBuf::from(".wisteria/work/bin/META-INF/");
        if manifest_path.exists()
        {
            fs::remove_dir_all(".wisteria/work/bin/META-INF/").unwrap();
        }
        fs::create_dir_all(manifest_path).unwrap();
        fs::write(".wisteria/work/bin/META-INF/MANIFEST.MF", &manifest.to_file()).unwrap();
        
        let mut jar_command = Command::new("jar");
        jar_command.args(["-cMf", ".wisteria/work/target.jar"]);
        if let Some(includes) = configuration.includes()
        {
            for i in includes
            {
                jar_command.arg(i);
            }
        }

        jar_command.args(["-C", ".wisteria/work/bin/", "."]);

        match jar_command.output()
        {
            Ok(output) => {
                let stdout = String::from_utf8(output.stdout).unwrap();
                let stderr = String::from_utf8(output.stderr).unwrap();
                if !stdout.is_empty()
                {
                    println!("{stdout}")
                }
                if !stderr.is_empty()
                {
                    println!("{stderr}")
                }
            }
            Err(e) => return Err((format!("Failed to package: {e}"), 1))
        }

        if !shaded_jars.is_empty()
        {
            let mut jar_update_command: Command = Command::new("jar");
            jar_update_command.args(["-uf", ".wisteria/work/target.jar", "-C", ".wisteria/work/shaded/", "."]);

            let _ = jar_update_command.output();

            //if let Err(e) = fs::remove_dir_all(".wisteria/work/shaded/")
            //{
            //    return Err((format!("Could not remove shaded work folder: {e}"), 1))
            //}
        }

        let bytes: Vec<u8> = fs::read(".wisteria/work/target.jar").unwrap();
        let hash = digest(bytes);
        println!("Packaged, hash: #{hash}");

        if let Some(targets) = configuration.targets()
        {
            for target in targets 
            {
                let target = resolve_filepath(target, configuration.environment(), &regexes)?;
                let target_path: PathBuf = PathBuf::from(&target);
                
                if !target_path.exists()
                {
                    let parent = target_path.parent().unwrap();
                    
                    fs::create_dir_all(parent).map_err(| e | (format!("Could not create parent folder {}: {e}", parent.to_string_lossy()), 1))?;
                }

                fs::write(&target, fs::read(".wisteria/work/target.jar").unwrap()).map_err(| e | (format!("Failed to write to target {target}: {e}"), 1))?;
                println!("Successfully written target {target}");
            }
        }

        Ok(())
    }

    fn phase_order(&self) -> &[String] 
    {
        self.order.as_ref()
    }
}

impl ImplicitCleanTask
{
    pub fn new() -> Self 
    {
        ImplicitCleanTask { order: vec![ String::from("target") ] }
    }
}

impl TaskRunner for ImplicitCleanTask
{
    fn invoke(&self, info: &ProjectInfo, project: &Project, configuration: &Configuration) -> Result<(), (String, u8)> 
    {
        todo!()
    }

    fn phase_order(&self) -> &[String] 
    {
        self.order.as_ref()
    }
}
