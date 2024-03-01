use std::{collections::HashMap, env, fs, path::PathBuf, process::exit};

use toml::Table;

use crate::{debugln, silentln, task::{self, Task}, Flags};

pub struct Project
{
    name: String,
    tasks: HashMap<String, Task>,
    default_task: Option<Task>
}

impl Project
{
    pub fn get_name(&self) -> &str
    {
        self.name.as_ref()
    }

    pub fn get_tasks(&self) -> &HashMap<String, Task>
    {
        &self.tasks
    }

    pub fn default_task(&self) -> Option<&Task> 
    {
        self.default_task.as_ref()
    }
}

/// Creates an example project that matches the template project.toml
pub fn generate_template_project(name: &String) -> Project
{
    let default_task = task::generate_template_task();
    let tasks: HashMap<String, Task> = HashMap::from([(String::from("main"), default_task.clone()); 1]);

    Project { 
        name: name.to_string(), 
        tasks, 
        default_task: Some(default_task)
    }
}

pub fn read_project(flags: &Flags) -> Result<Project, String>
{
    // Attempt to read tasks
    let working_directory: PathBuf = match env::current_dir() {
        Ok(path) => path,
        Err(_) => {
            silentln!(flags, "Could not read the working directory. Check that you have read access to the file and try again.");
            exit(1)
        }
    };

    let project_file: PathBuf = working_directory.join("project.toml");
    if !project_file.exists()
    {
        silentln!(flags, "No project.toml file in the working directory. If this is a valid Java project, consider configuring it with \"wisteria init\"");
        exit(1);
    }

    let raw_project_toml: String = match fs::read_to_string(project_file) {
        Ok(data) => data,
        Err(_) => {
            silentln!(flags, "Could not read project.toml. Check that you have read access to the file and try again.");
            exit(1);
        }
    };

    let toml = match raw_project_toml.parse::<Table>() {
        Ok(t) => t,
        Err(e) => {
            return Err(format!("Invalid or corrupt toml file. Open the file in your favorite text editor to correct it and try again. Error: {}", e))
        }
    };

    // Set up project data
    //-------------------------------------------------------------------------------- 
    let project_table = match toml.get("project") {
        Some(t) if t.is_table() => t.as_table().unwrap().clone(),
        Some(_) => {
            silentln!(flags, "Invalid project header, project name and libraries will be unavailable.");
            Table::new()
        },
        None => Table::new()
    };
    let mut project: Project = Project { 
        name: match project_table.get("name") {
            Some(v) if v.is_str() => v.as_str().unwrap().to_string(),
            Some(v) => {
                silentln!(flags, "Unexpected value for project name, expected string, received {}. Project name is unavailable.", v.type_str());
                "(Untitled)".to_string()
            },
            None => { 
                silentln!(flags, "No project name specified. Project name is unavailable.");
                "(Untitled)".to_string()
            }
        }, 
        tasks: HashMap::new(),
        default_task: None
    };

    // Start sequentially reading off tasks
    //-------------------------------------------------------------------------------- 

    let mut default_task: Option<Task> = None;
    match toml.get("task")
    {
        Some(s) if s.is_table() => { 
            default_task = Some(Task::parse("default".to_string(), s.as_table().unwrap(), &None, &flags).resovlve_files(&project, &flags))
        }
        Some(s) => silentln!(flags, "Unexpected type for default task, expected a table with the header \"task\", received {}. Task defaults are unavailable.", s.type_str()),
        None => silentln!(flags, "No default task given. Task defaults are unavailable.")
    }

    project.default_task = default_task.clone();

    for key in toml.keys()
    {
        if key.starts_with("task_")
        {
            let name: String = key.trim_start_matches("task_").to_string();
            match toml.get(key)
            {
                Some(s) if s.is_table() => {
                    debugln!(flags, "Parsing task {}", &key);
                    project.tasks.insert(name.clone(), Task::parse(name, s.as_table().unwrap(), &default_task, &flags).resovlve_files(&project, &flags));
                    debugln!(flags, "Successfully added new task {}", &key);
                }
                Some(s) => silentln!(flags, "Invalid task {}, expected a table with the name {}, received {}.", &key, &key, s),
                None => continue
            }
        }
    }

    if project.tasks.is_empty()
    {
        match default_task
        {
            Some(s) => {
                project.tasks.insert("default".to_string(), s);
            },
            None => {
                silentln!(flags, "No tasks given, and default task is not available, project cannot build");
            }
        }
    }

    if flags.debug
    {
        let mut known_tasks: String = String::new();
        for (name, _) in project.get_tasks()
        {
            known_tasks.push_str(name);
            known_tasks.push_str(", ");
        }

        known_tasks.pop();
        known_tasks.pop();

        println!("Tasks in project {}: [ {} ]", &project.get_name(), known_tasks);
    }

    Ok(project)
}
