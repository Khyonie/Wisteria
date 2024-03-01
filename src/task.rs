use std::{collections::HashMap, process::exit};

use toml::{map::Map, Value};

use crate::{java::MajorVersion, project::Project, silentln, source, Flags};

#[derive(Clone)]
pub struct Task
{
    name: String,
    description: Option<String>,
    source: Vec<String>,
    libraries: Option<Vec<String>>,
    libraries_relative: Vec<String>,
    javadocs: Option<HashMap<String, String>>,
    entry: Option<String>,
    include: Option<Vec<String>>,
    output: Vec<String>,
    compiler_options: CompilerOptions,
    shaded_jars: Option<Vec<String>>,
    java_version: Option<u8>
}

#[derive(Default)]
#[derive(Clone)]
pub struct CompilerOptions 
{
    enable_preview_features: bool,
    show_deprecated: bool,
    no_warnings: bool,
    store_parameter_names: bool,
    lint_javadocs: bool,
    lint_all_warnings: bool,
    extra_flags: Option<String>
}

impl CompilerOptions
{
    pub fn from(data: &Map<String, Value>) -> Self
    {
        let mut options = CompilerOptions::default();

        for key in data.keys()
        {
            match key.as_str()
            {
                "enable_preview_features" => {
                    if let Some(b) = data.get(key).unwrap().as_bool()
                    {
                        options.enable_preview_features = b;
                    } 
                },
                "show_deprecated" => {
                    if let Some(b) = data.get(key).unwrap().as_bool()
                    {
                        options.show_deprecated = b;
                    } 
                },
                "no_warnings" => {
                    if let Some(b) = data.get(key).unwrap().as_bool()
                    {
                        options.no_warnings = b;
                    } 
                },
                "store_parameter_names" => {
                    if let Some(b) = data.get(key).unwrap().as_bool()
                    {
                        options.store_parameter_names = b;
                    } 
                },
                "lint_javadocs" => {
                    if let Some(b) = data.get(key).unwrap().as_bool()
                    {
                        options.lint_javadocs = b;
                    } 
                },
                "lint_all_warnings" => {
                    if let Some(b) = data.get(key).unwrap().as_bool()
                    {
                        options.lint_all_warnings = b;
                    } 
                },
                "extra_flags" => {
                    if let Some(s) = data.get(key).unwrap().as_str()
                    {
                        options.extra_flags = Some(s.to_string());
                    } 
                },
                _ => ()
            }
        }

        options
    }

    pub fn to_compiler_arguments(&self) -> Vec<String>
    {
        let mut compiler_args: Vec<String> = Vec::new();

        if self.enable_preview_features 
        {
            compiler_args.push(String::from("--enable-preview"));
        }
        if self.show_deprecated 
        {
            compiler_args.push(String::from("-deprecation"));
        }
        if self.no_warnings
        {
            compiler_args.push(String::from("-nowarn"));
        }
        if self.store_parameter_names
        {
            compiler_args.push(String::from("-parameters"));
        }
        if self.lint_javadocs
        {
            compiler_args.push(String::from("-Xdoclint:all"))
        }
        if self.lint_all_warnings
        {
            compiler_args.push(String::from("-Xlint:all"))
        }
        if let Some(extra) = self.extra_flags.as_ref()
        {
            for s in extra.split(" ")
            {
                compiler_args.push(String::from(s));
            }
        }

        compiler_args
    }
}

pub fn get_task(name: &String, project: &Project) -> Option<Task>
{
    match project.get_tasks().get(name)
    {
        Some(s) => Some(s.clone()),
        None => None
    }
}

impl Task
{
    pub fn parse(name: String, toml_data: &Map<String, Value>, default_task: &Option<Task>, flags: &Flags) -> Self 
    {
        let description = Self::parse_description(&name, toml_data.get("description"), default_task, flags);
        let source = Self::parse_source(&name, toml_data.get("source"), default_task, flags);
        let libraries: Option<Vec<String>>;
        let libraries_relative: Vec<String>;
        match Self::parse_libraries(&name, toml_data.get("libraries"), default_task, flags)
        {
            (canon, relative) => {
                libraries = canon;
                libraries_relative = relative;
            }
        };
        let shaded_jars = Self::parse_shaded_jars(&name, toml_data.get("shaded_jars"), default_task, flags);
        let javadocs = Self::parse_javadocs(&name, toml_data.get("javadocs"), default_task, flags);
        let entry = Self::parse_entry(&name, toml_data.get("entry"), default_task, flags);
        let include = Self::parse_included(&name, toml_data.get("include"), default_task, flags);
        let output = Self::parse_output(toml_data.get("output"), default_task, flags);
        let compiler_options = Self::parse_compiler_options(toml_data.get("compiler"), default_task, flags);
        let java_version = Self::parse_java_version(toml_data.get("java_version"), default_task, flags);
        
        Task { name, description, libraries, libraries_relative, javadocs, source, entry, include, output, compiler_options, shaded_jars, java_version }
    }
    
    pub fn resovlve_files(mut self, project: &Project, flags: &Flags) -> Self
    {
        self.libraries = match &self.libraries
        {
            Some(l) => Some(l.iter().map(| s | source::canonicalize_path(s.clone(), &self, project, flags)).collect()),
            None => None
        };

        self.output = self.output.iter().map(| s | source::canonicalize_path(s.clone(), &self, project, flags)).collect();

        self.include = match &self.include
        {
            Some(i) => Some(i.iter().map(| s | source::canonicalize_path(s.clone(), &self, project, flags)).collect()),
            None => None
        };

        self
    }

    fn parse_description(task: &String, value: Option<&Value>, default_task: &Option<Task>, flags: &Flags) -> Option<String>
    {
        match value
        {
            Some(v) if v.is_str() => Some(v.as_str().unwrap().to_string()),
            Some(v) => {
                silentln!(flags, "Invalid description in task {}, expected a string, received {}. Skipping...", task, v.type_str());
                None
            },
            None => match default_task {
                Some(t) => t.description.clone(),
                None => None
            }
        }
    }

    fn parse_source(task: &String, value: Option<&Value>, default_task: &Option<Task>, flags: &Flags) -> Vec<String>
    {
        match value
        {
            Some(v) if v.is_str() => vec!(v.to_string()),
            Some(v) if v.is_array() => {
                let mut sources: Vec<String> = Vec::new();
                for source in v.as_array().unwrap()
                {
                    if let Some(s) = source.as_str()
                    {
                        sources.push(s.to_string());
                        continue;
                    }

                    silentln!(flags, "Invalid source folder {} in task {}. Expected a string, received {}. Skipping...", task, source, source.type_str());
                }

                if sources.is_empty() && !flags.silent
                {
                    println!("No sources given for task {}, this task cannot compile.", task);
                }

                sources
            },
            Some(v) => {
                silentln!(flags, "Invalid source entry for task {}, expected a string or a string array, received {}. Aborting...", task, v.type_str());
                exit(2)
            },
            None => match default_task {
                Some(t) => t.source.clone(),
                None => Vec::new()
            }
        }
    }

    fn parse_libraries(task: &String, value: Option<&Value>, default_task: &Option<Task>, flags: &Flags) -> (Option<Vec<String>>, Vec<String>)
    {
        match value
        {
            Some(v) if v.is_array() => {
                let mut libraries: Vec<String> = Vec::new();
                let mut relative: Vec<String> = Vec::new();
                for value in v.as_array().unwrap()
                {
                    if let Some(i) = value.as_str()
                    {
                        libraries.push(i.to_string());
                        relative.push(i.to_string());
                        continue
                    }

                    silentln!(flags, "Invalid library {} for task {}. Expected a string, received {}. Skipping...", value, task, value.type_str());
                    continue;
                }

                (Some(libraries), relative)
            }
            Some(v) => {
                silentln!(flags, "Invalid library {} for task {}. Expected a string, received {}. Aborting...", v, task, v.type_str());
                exit(2)
            }
            None => match default_task {
                Some(t) => (t.libraries.clone(), t.libraries_relative.clone()),
                None => (None, Vec::new())
            }
        }
    }

    fn parse_shaded_jars(task: &String, value: Option<&Value>, default_task: &Option<Task>, flags: &Flags) -> Option<Vec<String>>
    {
        match value
        {
            Some(v) if v.is_array() => {
                let mut shaded: Vec<String> = Vec::new();
                for value in v.as_array().unwrap()
                {
                    if let Some(i) = value.as_str()
                    {
                        shaded.push(i.to_string());
                        continue
                    }

                    silentln!(flags, "Invalid shaded jar entry {} for task {}. Expected a string, received {}. Skipping...", value, task, value.type_str());
                    continue;
                }

                Some(shaded)
            }
            Some(v) => {
                silentln!(flags, "Invalid shaded jar entry {} for task {}. Expected a string, received {}. Aborting...", v, task, v.type_str());
                exit(2)
            }
            None => match default_task {
                Some(t) => t.shaded_jars.clone(),
                None => None
            }
        }
    }

    fn parse_javadocs(task: &String, value: Option<&Value>, default_task: &Option<Task>, flags: &Flags) -> Option<HashMap<String, String>>
    {
        match value
        {
            Some(v) if v.is_table() => {
                let mut data: HashMap<String, String> = HashMap::new();

                if let Some(t) = v.as_table()
                {
                    for (key, target) in t
                    {
                        if !target.is_str()
                        {
                            silentln!(flags, "Invalid Javadoc target {} in task {}. Expected a string, received {}. Skipping...", task, target, target.type_str());
                            continue;
                        }

                        data.insert(key.clone(), target.as_str().unwrap().to_string());
                    }
                }

                Some(data)
            },
            Some(v) => {
                silentln!(flags, "Invalid javadoc table {} for task {}. Expected a table, received {}. Aborting...", v, task, v.type_str());
                exit(2)
            }
            None => match default_task {
                Some(t) => t.javadocs().clone().cloned(),
                None => None
            }
        }
    }

    fn parse_entry(task: &String, value: Option<&Value>, default_task: &Option<Task>, flags: &Flags) -> Option<String>
    {
        match value
        {
            Some(v) if v.is_str() => Some(v.as_str().unwrap().to_string()),
            Some(v) => {
                silentln!(flags, "Invalid entry point for task {}, expected a string, received {}. Skipping...", task, v.type_str());
                None
            },
            None => match default_task {
                Some(t) => t.entry.clone(),
                None => None
            }
        }
    }

    fn parse_included(task: &String, value: Option<&Value>, default_task: &Option<Task>, flags: &Flags) -> Option<Vec<String>>
    {
        match value
        {
            Some(v) if v.is_array() => {
                let mut included: Vec<String> = Vec::new();
                for value in v.as_array().unwrap()
                {
                    if let Some(i) = value.as_str()
                    {
                        included.push(i.to_string());
                        continue
                    }

                    silentln!(flags, "Invalid include file {} for task {}. Expected a string, received {}. Skipping...", value, task, value.type_str());
                    continue;
                }

                Some(included)
            }
            Some(v) => {
                silentln!(flags, "Invalid include file entry {} for task {}. Expected a string, received {}. Aborting...", v, task, v.type_str());
                exit(2)
            }
            None => match default_task {
                Some(t) => t.include.clone(),
                None => None
            }
        }
    }

    fn parse_output(value: Option<&Value>, default_task: &Option<Task>, flags: &Flags) -> Vec<String>
    {
        match value
        {
            Some(v) if v.is_str() => vec!(v.to_string()),
            Some(v) if v.is_array() => {
                let mut targets: Vec<String> = Vec::new();
                for value in v.as_array().unwrap()
                {
                    if let Some(s) = value.as_str()
                    {
                        targets.push(s.to_string());
                        continue;
                    }

                    silentln!(flags, "Invalid output target {}. Expected a string, received {}. Skipping...", value, value.type_str());
                    continue;
                }

                targets
            },
            Some(v) => {
                silentln!(flags, "Invalid output target, expected a string or a string array, received {}. Aborting...", v.type_str());
                exit(2)
            },
            None => match default_task {
                Some(t) => t.output.clone(),
                None => Vec::new()
            }
        }
    }
    
    fn parse_compiler_options(value: Option<&Value>, default_task: &Option<Task>, flags: &Flags) -> CompilerOptions
    {
        match value
        {
            Some(v) => {
                match v 
                {
                    Value::Table(t) => CompilerOptions::from(t),
                    Value::String(s) => CompilerOptions { enable_preview_features: false, show_deprecated: false, no_warnings: false, store_parameter_names: false, lint_javadocs: false, lint_all_warnings: false, extra_flags: Some(s.clone()) },
                    _ => {
                        silentln!(flags, "Invalid compiler flags, expected a table of options or a string, received {}. Aborting...", v.type_str());
                        exit(2);
                    }
                }
            },
            None => match default_task {
                Some(t) => t.compiler_options.clone(),
                None => CompilerOptions::default()
            }
        }
    }

    fn parse_java_version(value: Option<&Value>, default_task: &Option<Task>, flags: &Flags) -> Option<u8>
    {
        match value
        {
            Some(v) if v.is_integer() => Some((v.as_integer().unwrap()) as u8),
            Some(v) => {
                silentln!(flags, "Invalid task compilation flags, expected a string, received {}. Skipping...", v.type_str());
                None
            },
            None => match default_task {
                Some(t) => t.java_version.clone(),
                None => None
            }
        }
    }

    pub fn can_build(&self, java_version: &MajorVersion) -> Result<(), String>
    {
        if self.get_sources().is_empty()
        {
            return Err(format!("Task {} does not specify any sources", self.get_name()));
        }

        if self.output().is_empty()
        {
            return Err(format!("Task {} does not specify any targets", self.get_name()));
        }

        match self.java_version()
        {
            Some(version) if (java_version.version < version) => {
                return Err(format!("Task {} requires at minimum Java {}, but version {} is installed", self.get_name(), version, java_version.version))
            },
            Some(_) | None => {}
        }

        Ok(())
    }

    pub fn print_information(&self)
    {
        println!("Task \"{}\"", self.name);
        println!("\tDescription: {}", self.description.as_ref().unwrap_or(&"Not set".to_string()));
        println!("\tSources: {:?}", self.source);
        println!("\tTargets: {:?}", self.output);
        println!("\tLibraries: {:?}", self.libraries.as_ref().unwrap_or(&vec!("".to_string())));
        println!("\tRelative libraries: {:?}", self.get_relative_libraries());
        println!("\tIncludes: {:?}", self.include.as_ref().unwrap_or(&vec!("".to_string())));
        println!("\tEntry: {}", self.entry.as_ref().unwrap_or(&"Not set".to_string()));
        println!("\tJava version: {}", self.java_version.as_ref().unwrap_or(&8));
        //println!("\tJava options: {}", self.compiler_options.as_ref().unwrap_or(&"Not set".to_string()));
    }

    pub fn get_name(&self) -> &String 
    {
        &self.name
    }

    pub fn get_description(&self) -> Option<&String>
    {
        self.description.as_ref()
    }

    pub fn get_sources(&self) -> &[String] 
    {
        self.source.as_ref()
    }

    pub fn get_entry(&self) -> Option<&String> {
        self.entry.as_ref()
    }

    pub fn get_includes(&self) -> Option<&Vec<String>> 
    {
        self.include.as_ref()
    }

    pub fn output(&self) -> &[String] 
    {
        self.output.as_ref()
    }

    pub fn compiler_options(&self) -> &CompilerOptions
    {
        &self.compiler_options
    }

    pub fn java_version(&self) -> Option<u8> 
    {
        self.java_version
    }

    pub fn get_libraries(&self) -> Option<&Vec<String>> 
    {
        self.libraries.as_ref()
    }

    pub fn get_relative_libraries(&self) -> &[String] 
    {
        &self.libraries_relative
    }

    pub fn javadocs(&self) -> Option<&HashMap<String, String>> 
    {
        self.javadocs.as_ref()
    }

    pub fn get_shaded_jars(&self) -> Option<&Vec<String>> 
    {
        self.shaded_jars.as_ref()
    }
}

pub fn generate_template_task() -> Task
{
    Task { 
        name: String::from("default"), 
        description: None, 
        source: vec!(String::from("src/")), 
        libraries: Some(vec!(String::from("lib/"))), 
        libraries_relative: Vec::new(), 
        javadocs: None, 
        entry: None, 
        include: None, 
        output: vec!(String::from("target/{TASK_NAME}/{PROJECT_NAME}.jar")), 
        compiler_options: CompilerOptions::default(), 
        shaded_jars: None,
        java_version: Some(8)
    }
}
