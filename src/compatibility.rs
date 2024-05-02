use std::{env, fs::{self, File}, path::PathBuf, process::exit};

use toml::{Table, Value};
use xml::EventReader;

use crate::{debugln, silentln, Flags};

pub fn update_wisteria_1_project(flags: &Flags)
{
    let mut working_directory = env::current_dir().unwrap();
    working_directory.push("project.toml");

    if !working_directory.exists()
    {
        silentln!(flags, "No project.toml exists at the current directory. Aborting...");
        exit(1);
    }

    let raw_project_string = match fs::read_to_string("project.toml") {
        Ok(s) => s,
        Err(_) => {
            silentln!(flags, "Could not read project.toml. Check that you have read access to the file and try again.");
            exit(1);
        }
    };

    let wisteria_project = match raw_project_string.parse::<Table>() {
        Ok(t) => t,
        Err(e) => {
            silentln!(flags, "Invalid or corrupt toml file. Open the file in your favorite text editor to correct it and try again. Error: {}", e);
            exit(1);
        }
    };

    let mut updated_project = Table::new();
    let mut project_table = Table::new();
    let mut default_task_table = Table::new();

    for (key, value) in &wisteria_project
    {
        match key.as_str()
        {
            // Read project header
            //-------------------------------------------------------------------------------- 
            "project" if value.is_table() => {
                let value = value.as_table().unwrap();

                for (key, v) in value
                {
                    match key.as_str()
                    {
                        "name" if v.is_str() => { 
                            debugln!(flags, "Set project name as {}", v.as_str().unwrap());
                            project_table.insert(String::from("name"), v.clone());
                        },
                        "name" => {
                            silentln!(flags, "Expected a string for project name, received a {}. Skipping...", v.type_str());
                        }
                        "dsecription" if v.is_str() => {
                            debugln!(flags, "Set default task description as {}", v.as_str().unwrap());
                            default_task_table.insert(String::from("description"), v.clone());
                        }
                        "description" => {
                            silentln!(flags, "Expected a string for project description, received a {}. Skipping...", v.type_str());
                        }
                        "libraries" if v.is_array() => {
                            let mut libraries: Vec<Value> = Vec::new();
                            for a in v.as_array().unwrap()
                            {
                                if let Some(a) = a.as_str()
                                {
                                    debugln!(flags, "Added library {}", a);
                                    libraries.push(Value::String(a.to_string()));
                                    continue;
                                }
                            }

                            debugln!(flags, "Added ({}) library(ies) to default task", &libraries.len());
                            default_task_table.insert(String::from("libraries"), Value::Array(libraries));
                        }
                        _ => todo!()
                    }
                }
            }
            // Read tasks and subtasks
            //-------------------------------------------------------------------------------- 
            "task" if value.is_table() => {
                let value = value.as_table().unwrap();
                parse_task(&value, &mut default_task_table, flags);

                for (k, v) in value
                {
                    match k.as_str()
                    {
                        _ if v.is_table() => {
                            // Read subtask
                            let mut task = Table::new();
                            //task.insert(String::from("name"), Value::String(k.clone()));

                            parse_task(v.as_table().unwrap(), &mut task, flags);

                            updated_project.insert(format!("task_{}", k), Value::Table(task));
                            debugln!(flags, "Registered new task {}", k);
                        },
                        _ => silentln!(flags, "Skipping unknown key task.{} of type {}", k, v.type_str())
                    }
                }
            }
            _ => silentln!(flags, "Skipping unknown key {} of type {}", key, value.type_str())
        }
    }

    updated_project.insert(String::from("project"), Value::Table(project_table));
    updated_project.insert(String::from("task"), Value::Table(default_task_table));

    let new_project = toml::to_string(&updated_project).unwrap();
    let project_file = format!("# Project automatically converted from an existing Wisteria 1.x.x project\n# For more configuration options, see the Wisteria wiki\n{}", &new_project);

    debugln!(flags, "{}", &new_project);

    if flags.dry
    {
        silentln!(flags, "{}", &new_project);
        exit(0);
    }

    debugln!(flags, "Creating a backup of existing project.toml");
    match fs::copy("project.toml", "project.toml.bak")
    {
        Ok(_) => (),
        Err(e) => {
            silentln!(flags, "Could not create a backup of existing project.toml. Leaving it intact, error: {}", e);
            silentln!(flags, "You can perform the backup and write yourself if you so desire:");
            silentln!(flags, "{}", project_file);

            exit(1);
        }
    }

    debugln!(flags, "Writing new project.toml");
    match fs::write("project.toml", &project_file)
    {
        Ok(_) => (),
        Err(e) => {
            silentln!(flags, "Could not write to existing project.toml, error: {}", e);
            silentln!(flags, "You can perform the write yourself if you so desire:");
            silentln!(flags, "{}", project_file);
            
            exit(1);
        }
    }
}

fn parse_task(data: &Table, task: &mut Table, flags: &Flags)
{
    for (k, v) in data
    {
        match k.as_str()
        {
            "source" if v.is_str() => {
                debugln!(flags, "\tRegistering source folders");
                let string_value = Value::String(v.as_str().unwrap().to_string());
                let array_value = Value::Array(vec!(string_value));

                task.insert(String::from("source"), array_value);
            },
            "source" => silentln!(flags, "Invalid source {:?}, expected a string, received {}, skipping...", &v, &v.type_str()),

            // String array options
            "output" if v.is_array() => {
                debugln!(flags, "\tRegistering output folders");
                let updated_array: Vec<Value> = v.as_array().unwrap()
                    .iter()
                    .map(| v | match v {
                        Value::String(s) => Value::String(s.replace("{NAME}", "{PROJECT_NAME}")), // Update older special {NAME}
                        _ => v.clone()
                    })
                    .collect();

                task.insert(String::from(k), Value::Array(updated_array));
            },
            "output" => silentln!(flags, "Invalid outputs {:?}, expected an array, received {}, skipping...", &v, &v.type_str()),
            "input" if v.is_array() => {
                debugln!(flags, "\tRegistering includes files");
                let array_value = v.clone();

                task.insert(String::from("include"), array_value);
            },
            "input" => silentln!(flags, "Invalid inputs {:?}, expected an array, received {}, skipping...", &v, &v.type_str()),

            // Single-string options
            "description" | "entry" | "arguments" if v.is_str() => {
                debugln!(flags, "\tRegistering task {}", &k);
                let string_value = Value::String(v.as_str().unwrap().to_string());

                task.insert(String::from(k), string_value);
            }
            "entry" => silentln!(flags, "Invalid entry point {:?}, expected a string, received {}, skipping...", &v, &v.type_str()),
            "description" => silentln!(flags, "Invalid description {:?}, expected a string, received {}, skipping...", &v, &v.type_str()),
            "arguments" => silentln!(flags, "Invalid compile arguments {:?}, expected a string, received {}, skipping...", &v, &v.type_str()),
            _ => () // We can skip empty keys here
        }
    }
}

pub fn load_eclipse_project(flags: &Flags)
{
    let working_directory = env::current_dir().unwrap();
    if !working_directory.join(".project").exists()
    {
        silentln!(flags, "No Eclipse project exists at the current directory. Aborting...");
        exit(1);
    }

    if !working_directory.join(".classpath").exists()
    {
        silentln!(flags, "Could not load project classpath. Aborting...");
        exit(1);
    }

    let mut project_name: String = String::new();
    let mut sources: Vec<String> = Vec::new();
    let mut libraries: Vec<String> = Vec::new();

    // Read .project file
    let project_file: File = match File::open(".project") {
        Ok(f) => f,
        Err(e) => {
            silentln!(flags, "Could not open .project file. Error: {}, aborting...", e);
            exit(1);
        }
    };

    debugln!(flags, "Parsing .project file");
    let mut elements: Vec<String> = Vec::new();
    for i in EventReader::new(project_file)
    {
        if let Ok(event) = i 
        {
            match event
            {
                xml::reader::XmlEvent::StartDocument { version, encoding, standalone: _ } => debugln!(flags, "XML Version: {}, encoding: {}", version, encoding),
                xml::reader::XmlEvent::EndDocument => debugln!(flags, "End of document reached"),
                xml::reader::XmlEvent::StartElement { name, attributes, namespace: _ } => {
                    elements.push(name.to_string());

                    debugln!(flags, "Attribute: {}", name);
                    for a in attributes
                    {
                        debugln!(flags, "\tSub-attribute: \"{}\": \"{}\"", a.name, a.value);
                    }
                },
                xml::reader::XmlEvent::EndElement { name } => {
                    debugln!(flags, "\tEnd current attribute {}", name);
                    elements.pop();
                },
                xml::reader::XmlEvent::Comment( comment ) => silentln!(flags, "\tComment: {}", comment),
                xml::reader::XmlEvent::Characters( data ) => {
                    debugln!(flags, "\tData: \"{}\", current attribute: {}", data, elements.last().unwrap_or(&String::from("(None)")));

                    if let Some(current) = elements.last()
                    {
                        match current.as_str()
                        {
                            "name" if elements.len() == 2 => project_name = data,
                            _ => ()
                        }
                    }
                },
                _ => ()
            }
        }
    }

    // Read .classpath file
    let classpath_file: File = match File::open(".classpath") {
        Ok(f) => f,
        Err(e) => {
            silentln!(flags, "Could not open .project file. Error: {}, aborting...", e);
            exit(1);
        }
    };

    debugln!(flags, "Parsing .classpath file");
    for i in EventReader::new(classpath_file)
    {
        if let Ok(event) = i 
        {
            match event
            {
                xml::reader::XmlEvent::StartDocument { version, encoding, standalone: _ } => debugln!(flags, "XML Version: {}, encoding: {}", version, encoding),
                xml::reader::XmlEvent::EndDocument => debugln!(flags, "End of document reached"),
                xml::reader::XmlEvent::Comment( comment ) => silentln!(flags, "\tComment: {}", comment),
                xml::reader::XmlEvent::Characters( data ) => debugln!(flags, "\tData: \"{}\"", data),
                xml::reader::XmlEvent::EndElement { name } => debugln!(flags, "\tEnd current attribute {}", name),

                xml::reader::XmlEvent::StartElement { name, attributes, namespace: _ } => {
                    debugln!(flags, "Attribute: {}", name);
                    if name.to_string().as_str() == "classpath"
                    {
                        continue;
                    }

                    let mut entry_kind: String = String::new();

                    for a in attributes
                    {
                        debugln!(flags, "\tSub-attribute: \"{}\": \"{}\"", a.name, a.value);
                        match a.name.to_string().as_str()
                        {
                            "kind" => entry_kind = a.value,
                            "path" => {
                                match entry_kind.as_str()
                                {
                                    "src" => {
                                        if a.value.starts_with("target/") // Skip target source folders
                                        {
                                            continue;
                                        }

                                        sources.push(a.value.to_string());
                                    },
                                    "lib" => libraries.push(a.value.to_string()),
                                    _ => ()
                                }
                            }
                            _ => ()
                        }
                    }
                },
                _ => ()
            }
        }
    }

    // If the user wants to clean up their libraries section in their project.toml, they're welcome to do so
    
    // We can now build the project table from the given information
    //--------------------------------------------------------------------------------
    let mut project = Table::new();

    // Header
    let mut project_header = Table::new();
    project_header.insert(String::from("name"), Value::String(project_name));
    project.insert(String::from("project"), Value::Table(project_header));

    // Default task
    let mut default_task = Table::new();
    default_task.insert(String::from("output"), Value::Array(vec!(Value::String(String::from("targets/{TASK_NAME}/{PROJECT_NAME}.jar")))));

    let source_values: Vec<Value> = sources.iter()
        .map(| s | Value::String(s.to_string()))
        .collect();

    let library_values: Vec<Value> = libraries.iter()
        .map(| l | Value::String(l.to_string()))
        .collect();

    default_task.insert(String::from("source"), Value::Array(source_values));
    default_task.insert(String::from("libraries"), Value::Array(library_values));

    project.insert(String::from("task"), Value::Table(default_task));
    
    // Main task
    project.insert(String::from("task_main"), Value::Table(Table::new()));

    // Finally, we can write the task to a project.toml
    //-------------------------------------------------------------------------------- 
    let project_data = format!("# Automatically derived from existing Eclipse project\n# For more configuration options, see the Wisteria wiki\n{}", toml::to_string(&project).unwrap());

    if flags.dry
    {
        silentln!(flags, "{}", &project_data);
        exit(0);
    }
    
    if PathBuf::from("project.toml").exists()
    {
        silentln!(flags, "Creating a backup of existing project.toml");
        match fs::copy("project.toml", "project.toml.bak")
        {
            Ok(_) => debugln!(flags, "Created a backup of existing project.toml"),
            Err(e) => {
                silentln!(flags, "Could not create a backup of existing project.toml. Error: {}", e);
                silentln!(flags, "You can perform the backup and write yourself if you so desire:");
                silentln!(flags, "{}", project_data);
                exit(1);
            }
        }
    }

    match fs::write("project.toml", &project_data)
    {
        Ok(_) => debugln!(flags, "Wrote project configuration to project.toml"),
        Err(e) => {
            silentln!(flags, "Could not write to project.toml, error: {}", e);
            silentln!(flags, "You can perform the write yourself if you so desire:");
            silentln!(flags, "{}", project_data);
            exit(1);

        }
    }
}
