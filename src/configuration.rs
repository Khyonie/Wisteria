#![allow(dead_code)]

use std::{collections::HashMap, rc::Rc};

use toml::Table;

use crate::{compiler::CompilerFlags, task::{DefinedTask, ImplicitBuildTask, ImplicitCleanTask, TaskRunner}, util::toml_utils};

#[derive(Clone)]
pub struct Configuration
{
    name: String,
    sources: Option<Vec<String>>,
    dependencies: Option<Vec<String>>,
    shaded: Option<Vec<String>>,
    includes: Option<Vec<String>>,
    targets: Option<Vec<String>>,
    
    entry: Option<String>,
    java_version: u8,

    tasks: HashMap<String, Rc<dyn TaskRunner>>,
    compiler_flags: Option<Vec<CompilerFlags>>,
    environment: HashMap<String, String>,
    natures: Vec<String>,
    inherit: Option<String>
}

impl Configuration
{
    pub fn from(name: String, toml: &Table, project_name: String, version: String) -> Result<Self, (String, u8)>
    {
        let sources = toml_utils::read_string_array("sources", toml).ok();
        let dependencies = toml_utils::read_string_array("dependencies", toml).ok();
        let shaded = toml_utils::read_string_array("shaded", toml).ok();
        let includes = toml_utils::read_string_array("includes", toml).ok();
        let targets = toml_utils::read_string_array("targets", toml).ok();

        let entry = toml_utils::read_string("entry", toml).ok();
        let java_version = toml_utils::read_integer("java_version", toml).unwrap_or(8);
        let inherit: Option<String> = toml_utils::read_string("inherit", toml).ok();

        let mut tasks: HashMap<String, Rc<dyn TaskRunner>> = HashMap::new();

        match toml.get("task")
        {
            Some(v) if v.is_table() => {
                let v = v.as_table().unwrap();

                for key in v.keys()
                {
                    match v.get(key)
                    {
                        Some(t) if t.is_table() => tasks.insert(key.clone(), Rc::new(DefinedTask::new(key, t.as_table().unwrap())?)),
                        Some(t) => return Err((format!("Mismatched type for task \"{key}\", expected a table, found {}", t.type_str()), 16)),
                        None => panic!()
                    };
                }
            }
            Some(v) => return Err((format!("Mismatched type for \"task\", expected a table, found {}", v.type_str()), 16)),
            None => {}
        }

        let mut environment: HashMap<String, String> = HashMap::new();
        environment.insert(String::from("project_name"), project_name);
        environment.insert(String::from("configuration"), name.clone());
        environment.insert(String::from("version"), version);

        match toml.get("environment")
        {
            Some(t) if t.is_table() => {
                let t = t.as_table().unwrap();

                for (key, value) in t
                {
                    match value.as_str()
                    {
                        Some(s) => environment.insert(key.clone(), s.to_string()),
                        None => return Err((format!("Mismatched type for environment variable \"{key}\", expected a string, found {}", value.type_str()), 15))
                    };
                }
            }
            Some(v) => return Err((format!("Mismatched type for \"environment\", expected a table, found {}", v.type_str()), 15)),
            None => {}
        }

        let mut natures: Vec<String> = Vec::new();
        if let Ok(mut n) = toml_utils::read_string_array("natures", toml)
        {
            natures.append(&mut n);
        }

        if !natures.contains(&String::from("wisteria"))
        {
            natures.insert(0, String::from("wisteria"));
        }

        let compiler_flags: Option<Vec<CompilerFlags>> = match toml.get("compiler_flags")
        {
            Some(t) if t.is_table() => {
                let t = t.as_table().unwrap();
                let mut flags: Vec<CompilerFlags> = Vec::new();

                for (key, value) in t 
                {
                    flags.push(CompilerFlags::from(key, value)?);
                }

                Some(flags)
            }
            Some(v) => return Err((format!("Mismatched type for \"{name}.compiler_flags\", expected a table, found {}", v.type_str()), 16)),
            None => None
        };

        Ok(Configuration { name, sources, dependencies, shaded, includes, targets, entry, java_version, tasks, compiler_flags, environment, natures, inherit })
    }

    pub fn sources(&self) -> Option<&Vec<String>>
    {
        self.sources.as_ref()
    }

    pub fn dependencies(&self) -> Option<&Vec<String>>
    {
        self.dependencies.as_ref()
    }

    pub fn shaded(&self) -> Option<&Vec<String>>
    {
        self.shaded.as_ref()
    }

    pub fn includes(&self) -> Option<&Vec<String>>
    {
        self.includes.as_ref()
    }

    pub fn targets(&self) -> Option<&Vec<String>>
    {
        self.targets.as_ref()
    }

    pub fn entry(&self) -> Option<&String>
    {
        self.entry.as_ref()
    }

    pub fn java_version(&self) -> u8 
    {
        self.java_version
    }

    pub fn tasks(&self) -> &HashMap<String, Rc<dyn TaskRunner>> 
    {
        &self.tasks
    }

    pub fn inherits(&self) -> Option<&String>
    {
        self.inherit.as_ref()
    }

    pub fn environment(&self) -> &HashMap<String, String>
    {
        &self.environment
    }

    pub fn compiler_flags(&self) -> Option<&Vec<CompilerFlags>>
    {
        self.compiler_flags.as_ref()
    }

    pub fn apply_implicit(&mut self)
    {
        if self.targets.is_some()
        {
            self.tasks.insert(String::from("clean"), Rc::new(ImplicitCleanTask::new()));

            if self.sources.is_some()
            {
                self.tasks.insert(String::from("build"), Rc::new(ImplicitBuildTask::new()));
            }
        }
    }

    pub fn inherit_from(&mut self, configuration: &Configuration)
    {
        self.sources = inherit_vec(self.sources.as_mut(), configuration.sources.as_ref());
        self.dependencies = inherit_vec(self.dependencies.as_mut(), configuration.dependencies.as_ref());
        self.includes = inherit_vec(self.includes.as_mut(), configuration.includes.as_ref());
        self.targets = inherit_vec(self.targets.as_mut(), configuration.targets.as_ref());
        if self.entry.is_none() && configuration.entry.is_some()
        {
            self.entry = configuration.entry.clone();
        }
        self.java_version = configuration.java_version;
        for (k, task) in configuration.tasks()
        {
            if !self.tasks.contains_key(k)
            {
                self.tasks.insert(k.clone(), task.clone());
            }
        }
        self.compiler_flags = inherit_vec(self.compiler_flags.as_mut(), configuration.compiler_flags.as_ref());
        for (k, v) in &configuration.environment
        {
            if !self.environment.contains_key(k)
            {
                self.environment.insert(k.clone(), v.clone());
            }
        }
    }

    pub fn print_info(&self)
    {
        println!("╞ Configuration \"{}\":", self.name);

        if let Some(s) = &self.sources
        {
            println!("│\tSources          {}", toml_utils::string_vec_to_string(s))
        }

        if let Some(d) = &self.dependencies
        {
            println!("│\tDependencies     {}", toml_utils::string_vec_to_string(d))
        }

        if let Some(i) = &self.includes
        {
            println!("│\tIncludes         {}", toml_utils::string_vec_to_string(i))
        }

        if let Some(e) = &self.entry 
        {
            println!("│\tMain class       {e}")
        }

        println!("│\tJava version     {}", self.java_version);
        
        let mut environment: String = String::new();
        for (k, v) in &self.environment
        {
            environment.push_str(k);
            environment.push_str(format!(": \"{v}\", ").as_str());
        }

        environment.pop();
        environment.pop();

        println!("│\tEnvironment      [ {environment} ]");

        if let Some(flags) = &self.compiler_flags
        {
            let mut string: String = String::new();

            for f in flags 
            {
                let mut flag = String::new();

                for component in f.get_canon_flag()
                {
                    flag.push_str(&component);
                    flag.push(' ');
                }
                flag.pop();

                string.push_str(&flag);
                string.push_str(", ");
            }

            string.pop();
            string.pop();

            println!("│\tCompiler flags   [ {string} ]")
        }

        println!("│\tTasks:*          {}", &self.tasks.len());
        for (key, task) in &self.tasks
        {
            let mut phases: String = String::new();

            for phase in task.phase_order()
            {
                phases.push_str(phase);
                phases.push_str(" > ");
            }
            phases.pop();
            phases.pop();
            phases.pop();

            println!("│\t│\t         {key} [ {phases} ]")
        }
    }
}

fn inherit_vec<T: Clone + Eq>(inheritor: Option<&mut Vec<T>>, host: Option<&Vec<T>>) -> Option<Vec<T>>
{
    match inheritor
    {
        Some(data) => {
            if let Some(host_data) = host 
            {
                for s in host_data
                {
                    if !data.contains(s)
                    {
                        data.push(s.clone());
                    }
                }
            }

            Some(data.clone())
        }
        None if host.is_some() => {
            Some(host.unwrap().clone())
        } 
        None => None
    }
}
