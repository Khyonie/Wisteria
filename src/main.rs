use std::{env, process::{exit, Command}, fs::{self, File}, path::PathBuf, io::ErrorKind};

use toml::{Table, map::Map, Value};

struct Task {
    name: String,
    source: String,
    entry: Option<String>,
    description: String,
    output: Vec<String>,
    compile_options: Option<String>,
    input: Vec<String>
}

impl Task 
{
    fn read_tasks(data: &Map<String, Value>) -> Vec<Task>
    {
        let mut tasks = Vec::new();

        if data.is_empty()
        {
            return tasks;
        }

        match data.get("task")
        {
            Some(task_data) if task_data.is_table() => {
                for (k, _) in task_data.as_table().unwrap()
                {
                    tasks.push(Self::parse(k, data));
                }
            }
            Some(_) | None => { }
        }

        tasks
    }

    fn parse(name: &String, data: &Map<String, Value>) -> Self
    {
        let task: &Map<String, Value> = match data.get("task")
        {
            Some(all_tasks) if all_tasks.is_table() => {
                match all_tasks.as_table().unwrap().get(name) {
                    Some(t) if t.is_table() => {
                        t.as_table().unwrap()
                    },
                    Some(_) | None => {
                        println!("Unknown task {}.", name);
                        exit(1);
                    }
                }
            },
            Some(_) | None => {
                println!("No tasks have been defined. Create a task with the heading [task] with the following properties:\nsource = \"source_path\"\n(Optional) output = [\"{{NAME}}.jar\"]\n(Optional) input = [\"input.yml\"]");
                exit(1);
            }
        };

        Task {
            name: name.to_string(),
            source: match task.get("source") {
                Some(s) => match s.as_str() {
                    Some(o) => String::from(o),
                    None => {
                        println!("Unexpected type for source in task {}. Expected string, got {}", name, s.type_str());
                        exit(1);
                    }
                },
                None => {
                    String::from("src/")
                }
            },
            entry: match task.get("entry") {
                Some(s) => match s.as_str() {
                    Some(o) => Some(String::from(o)),
                    None => None
                },
                None => None
            },
            description: match task.get("description") {
                Some(s) => match s.as_str() {
                    Some(o) => String::from(o),
                    None => {
                        s.to_string()
                    }
                },
                None => {
                    String::from("A Wisteria build task.")
                }
            },
            output: match task.get("output") {
                Some(s) => match s.as_array() {
                    Some(o) => o.iter()
                        .map(|v| match v.as_str() {
                            Some(s) => String::from(s),
                            None => {
                                println!("Unexpected type for output file in output file array in task {}. Expected string, got {}", name, s.type_str());
                                exit(1);
                            }
                        })
                        .collect::<Vec<String>>(),
                    None => {
                        println!("Unexpected output file array in task {}. Expected string array, got {}", name, s.type_str());
                        exit(1);
                    }
                },
                None => {
                    vec!(String::from("target/build/{TASK}/{NAME}.jar"))
                }
            },
            input: match task.get("input") {
                Some(val) => {
                    match val.as_array() {
                        Some(s) => s.iter()
                            .map(|v| match v.as_str() {
                                Some(st) => String::from(st),
                                None => {
                                    println!("Unexpected type for input file in input file array in task {}. Expected string, got {}", name, v.type_str());
                                    exit(1);
                                }
                            })
                            .collect(),
                        None => {
                            println!("Unexpected input file array in task {}. Expected string array, got {}", name, val.type_str());
                            exit(1);
                        }
                    }
                }
                None => Vec::new()
            },
            compile_options: match task.get("arguments") {
                Some(val) => {
                    match val.as_str() {
                        Some(s) => Some(String::from(s)),
                        None => {
                            println!("Unexpected type for javac arguments in task {}. Expected string, got {}", name, val.type_str());
                            exit(1);
                        }
                    }
                },
                None => None
            },
        }
    }
}

fn main() 
{
    let program_arguments: Vec<String> = env::args()
        .collect();

    if program_arguments.len() == 1
    {
        println!("Missing operand.\nUsage: wisteria [ build | run | update | tasks | new | config ]
     \tbuild <task>: builds the project with the specified task
     \trun <task>: builds the project with the specified task and runs the resulting jar
     \tupdate: updates dependencies in project.toml, pom.xml, and .classpath
     \ttasks: lists all defined tasks
     \tnew <name> <package>: creates a new Wisteria Java project with the given name, creating a project.toml for it
     \tconfig: creates a new Wisteria project configuration file for an existing project");
        exit(1);
    }

    match program_arguments.get(1).unwrap().as_str()
    {
        "new" if program_arguments.len() >= 4 => {
            create_project(program_arguments.get(2).unwrap(), program_arguments.get(3).unwrap());
            exit(0);
        }
        "new" => {
            println!("wisteria new <name> <package>: creates a new Java project with the given name.\nUsage: wisteria new <name> <package>");
            exit(1);
        }
        "build" => {
            let task_name: String = program_arguments.get(2)
                .unwrap_or(&String::from("main"))
                .to_string();

            build(&task_name, false);
            exit(0);
        }
        "run" => {
            let task_name: String = program_arguments.get(2)
                .unwrap_or(&String::from("main"))
                .to_string();

            build(&task_name, true);
            exit(0);
        }
        "tasks" => {
            let mut working_directory = env::current_dir().unwrap();
            working_directory.push("project.toml");

            if !working_directory.exists()
            {
                println!("No project.toml file exists. Consider creating one with [$ wisteria config].");
                exit(0);
            }

            let project_toml = match fs::read_to_string(&working_directory)
            {
                Ok(o) => o,
                Err(e) => {
                    println!("Failed to read project.toml. Error: {}", e);
                    exit(1);
                }
            };

            let data = project_toml.parse::<Table>().unwrap();
            let tasks = Task::read_tasks(&data);
    
            if tasks.is_empty()
            {
                println!("No tasks exist for this project. Consider creating a new task.");
                exit(0);
            }

            println!("All tasks for this project:");
            for t in tasks 
            {
                println!("\t{} - {}", t.name, t.description);
            }
            exit(0);
        }
        "config" => {
            let working_directory = env::current_dir().unwrap();
            let project_name = working_directory.file_name().unwrap().to_string_lossy().to_string();
            
            let project_toml = 
r#"[project]
name = "%PROJECT_NAME%"
libraries = ["lib/"]

[task]

[task.main]
source = "src/"
output = ["target/{NAME}/{NAME}.jar"]
input = []"#
    .replace("%PROJECT_NAME%", &project_name);

            fs::write("project.toml", project_toml).ok().unwrap_or(());
            println!("Created new project.toml.");
        },
        "update" => {
            let mut working_directory = env::current_dir().unwrap();
            working_directory.push("project.toml");

            if !working_directory.exists()
            {
                println!("No project.toml file exists. Consider creating one with [-$ wisteria config].");
                exit(0);
            }

            let project_toml = match fs::read_to_string(&working_directory)
            {
                Ok(o) => o,
                Err(e) => {
                    println!("Failed to read project.toml. Error: {}", e);
                    exit(1);
                }
            };
            working_directory.pop();

            println!("Updating with {} as working directory", working_directory.to_string_lossy());

            let data = project_toml.parse::<Table>().unwrap();
            let data = data.get(&String::from("project")).unwrap().as_table().unwrap();

            let libraries_data = match data.get(&String::from("libraries"))
            {
                Some(lib_data) if lib_data.is_array() => {
                    lib_data.as_array().unwrap()
                        .iter()
                        .map(|v| match v.as_str() {
                            Some(s) => String::from(s),
                            None => {
                                println!("Unexpected object {} in library data of type {}", &v, &v.type_str());
                                exit(1);
                            }
                        })
                        .collect()
                }
                Some(_) | None => Vec::new(),
            };

            if libraries_data.is_empty()
            {
                println!("No library objects could be found. Aborting.");
                exit(1);
            }

            println!("Individual library objects: {:?}", &libraries_data);

            let mut libraries = Vec::new();
            
            for l in libraries_data 
            {
                working_directory.push(l);
                println!("Updating with target object {}", &working_directory.to_string_lossy());

                if working_directory.is_file()
                {
                    println!("- Object: {:?}", &working_directory.file_name().unwrap());
                    libraries.push(working_directory.to_string_lossy().to_string());
                    working_directory.pop();
                    continue;
                }

                populate_vec(&working_directory, "jar", &mut libraries);
                working_directory.pop();
            }

            create_classpath(&libraries);
            println!("Updated classpath with {} object(s)", &libraries.len());
        }
        _ => {
            println!("Unknown operand.\nUsage: wisteria [ build | update | tasks | new | config ]
     \tbuild <task>: builds the project with the specified task
     \tupdate: updates dependencies in project.toml, pom.xml, and .classpath
     \ttasks: lists all defined tasks
     \tnew <name> <package>: creates a new Wisteria Java project with the given name, creating a project.toml for it
     \tconfig: creates a new Wisteria project configuration file for an existing project");
            exit(1);
        }
    }
}

fn read_project(working_directory: &mut PathBuf) -> Map<String, Value>
{
    working_directory.push("project.toml");

    let project_string = match fs::read_to_string(&working_directory)
    {
        Ok(o) => o,
        Err(e) => {
            println!("\x1b[0m\x1b[48;5;161m\x1b[38;5;15mFailed to open project.toml. Check that a project.toml exists and try again.");
            println!("\x1b[0m\x1b[48;5;161m\x1b[38;5;15mError: {}", e);
            exit(1);
        }
    };

    working_directory.pop();

    project_string.parse::<Table>().unwrap()
    
}

fn build(task_name: &String, run: bool)
{
    println!(
"\x1b[38;5;13m⚘ \x1b[38;5;177mWisteria \x1b[0m| Java Project Management ☕
\x1b[38;5;69m@Discord: khyonie
\x1b[38;5;39m@Email: khyonie@proton.me");

    let mut working_directory = match env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            println!("\x1b[0m\x1b[48;5;161m\x1b[38;5;15mFailed to get working directory.");
            println!("\x1b[0m\x1b[48;5;161m\x1b[38;5;15mError: {}", e);
            exit(1);
        }
    };
    
    let project_data = read_project(&mut working_directory);
    let project_info = match project_data.get("project").unwrap().as_table() {
        Some(s) => {
            s
        },
        None => {
            todo!();
        }
    };

    let task = Task::parse(&task_name, &project_data);

    println!("\x1b[38;5;7m- Building project \x1b[38;5;10m{}\x1b[0m with task \x1b[38;5;11m{}\x1b[0m", project_info.get("name").unwrap().as_str().unwrap(), &task.name);

    let mut percent_string = String::from("#");
    print!("\x1b[2K\rProgress: [ {:-20} ]", &percent_string);

    // Populate source
    //working_directory.pop();
    working_directory.push(&task.source);

    let mut source_files: Vec<String> = Vec::new();

    populate_vec(&working_directory, "java", &mut source_files);

    percent_string.push_str("##");
    print!("\x1b[2K\rProgress: [ {:-20} ]", &percent_string);

    working_directory.pop();

    let mut library_files: Vec<String> = Vec::new();
    for s in project_info.get("libraries").unwrap_or(&Value::Array(vec!(Value::String(String::from("lib/"))))).as_array().unwrap()
    {
        working_directory.push(PathBuf::from(s.as_str().unwrap().to_string()));
        if working_directory.is_dir()
        {
            populate_vec(&working_directory, "jar", &mut library_files);
            working_directory.pop();
            continue
        }

        library_files.push(working_directory.to_string_lossy().to_string());
        working_directory.pop();
    }
    percent_string.push_str("##");
    print!("\x1b[2K\rProgress: [ {:-20} ]", &percent_string);

    let mut source_file_string = String::new();
    let mut source_file_iter = source_files.iter();
    loop {
        match source_file_iter.next()
        {
            Some(s) => {
                source_file_string.push_str(s);
                source_file_string.push_str(" ");
            },
            None => break
        }
    }
    source_file_string.pop();
    percent_string.push_str("##");
    print!("\x1b[2K\rProgress: [ {:-20} ]", &percent_string);

    let mut library_file_string = String::new();
    let mut library_file_iter = library_files.iter();
    loop {
        match library_file_iter.next()
        {
            Some(s) => {
                library_file_string.push_str(s);
                library_file_string.push_str(":");
            },
            None => break
        }
    }
    library_file_string.pop();
    percent_string.push_str("##");
    print!("\x1b[2K\rProgress: [ {:-20} ]", &percent_string);

    let mut input_file_string = String::new();
    for s in task.input
    {
        input_file_string.push_str(&s);
        input_file_string.push(' ');
    }

    percent_string.push_str("##");
    print!("\x1b[2K\rProgress: [ {:-20} ]", &percent_string);

    input_file_string.pop();
    
    let home_var = env::var("HOME").ok().unwrap_or(String::from("./"));
    percent_string.push_str("##");
    print!("\x1b[2K\rProgress: [ {:-20} ]", &percent_string);

    for output in task.output
    {
        let mut command_string = String::from("javac -d ./bin/ --source-path %SOURCE_PATH% --class-path %LIBRARIES% %SOURCES%")
            .replace("%SOURCE_PATH%", &task.source)
            .replace("%LIBRARIES%", &library_file_string)
            .replace("%SOURCES%", &source_file_string);    

        match &task.compile_options
        {
            Some(s) => {
                command_string.push(' ');
                command_string.push_str(s);
            },
            None => {}
        }

        let mut javac_command = &mut Command::new("sh");
        javac_command = javac_command
            .arg("-c")
            .arg(command_string);

        match javac_command.output()
        {
            Ok(out) => {
                if !out.stdout.is_empty()
                {
                    println!("javac out: {}", String::from_utf8(out.stdout).unwrap());
                }

                if !out.stderr.is_empty()
                {
                    let message = String::from_utf8(out.stderr).unwrap();
                    println!("javac err: {}", &message);

                    if !message.starts_with("Note:")
                    {
                        exit(1);
                    }

                }
            }   
            Err(e) => {
                println!("\n\x1b[0m\x1b[48;5;161m\x1b[38;5;15mCompilation failure. Error: {}", e);
                exit(1);
            }
        }

        let fixed_output = output.clone()
            .replace("{NAME}", project_info.get("name").unwrap().as_str().unwrap())
            .replace("~", &home_var);

        let mut output_path = PathBuf::from(&fixed_output);
        match fs::create_dir_all(fixed_output)
        {
            Ok(_) => {}
            Err(e) => {
                if e.kind() != ErrorKind::AlreadyExists
                {
                    println!("Failed to create parent directories. Error: {}", e);
                    exit(1);
                }
            }
        }

        output_path = match output_path.canonicalize()
        {
            Ok(p) => {
                p
            },
            Err(e) => {
                println!("\n\x1b[0m\x1b[48;5;161m\x1b[38;5;15mCould not canonicalize output path for {}, error: {}", &output, e);
                exit(1);
            }
        };

        if !output_path.exists()
        {
            File::create(&*output_path.to_string_lossy()).unwrap();
            //           ^^ Idk what this voodoo magic is but it isn't mad anymore
        }
        
        let jar_string = String::from("jar -cMf %OUTPUT% %INPUT% -C bin/ .")
            .replace("%OUTPUT%", &output_path.to_string_lossy())
            .replace("%INPUT%", &input_file_string);

        let mut jar_command = &mut Command::new("sh");
        jar_command = jar_command
            .arg("-c")
            .arg(jar_string);

        percent_string.push_str("###");
        print!("\x1b[2K\rProgress: [ {:-20} ]", &percent_string);

        match jar_command.output()
        {
            Ok(_) => {
                percent_string.push_str("#####");
                print!("\x1b[2K\rProgress: [ {:-20} ]", &percent_string);
                println!("\n\x1b[38;5;47mBUILD SUCCESS");

                match fs::remove_dir_all("./bin")
                {
                    Ok(_) => { }
                    Err(e) => println!("\x1b[0m\x1b[48;5;161m\x1b[38;5;15mCould not clean up binary folder. Error: {}", e)
                }
            }
            Err(e) => {
                println!("\x1b[0m\x1b[48;5;161m\x1b[38;5;15mFailed to package jar. Error: {}", e);
                exit(1);
            }
        }

        if run 
        {
            let java_run_string = String::from("java -jar %OUTPUT%")
                .replace("%OUTPUT%", &output_path.to_string_lossy());

            let mut java_run_command = &mut Command::new("sh");
            java_run_command = java_run_command
                .arg("-c")
                .arg(java_run_string);

            match java_run_command.spawn()
            {
                Ok(_) => {
                    println!("Running project.");
                }
                Err(e) => {
                    println!("Could not run jar. Error: {}", e);
                    exit(1);
                }
            }
        }
    }
}

fn populate_vec(path: &PathBuf, filetype: &str, data: &mut Vec<String>)
{
    if path.is_file()
    {
        data.push(String::from(path.to_string_lossy()));
        return;
    }

    match fs::read_dir(path)
    {
        Ok(readdir) => {
            for entryr in readdir 
            {
                if let Ok(entry) = entryr 
                {
                    if entry.path().is_dir() || entry.path().to_string_lossy().ends_with(&filetype)
                    {
                        // Recurse
                        populate_vec(&entry.path(), filetype, data)
                    }
                }
            }
        }
        Err(e) => {
            println!("{}", e)
        }
    }
}

fn create_project(project_name: &String, package: &String)
{
    // Create project data (project.toml, pom.xml)
    let project_toml = 
r#"[project]
name = "%PROJECT_NAME%"
libraries = ["lib/"]

[task]

[task.main]
source = "src/"
output = ["target/{NAME}/{NAME}.jar"]
input = []"#
    .replace("%PROJECT_NAME%", project_name);

    let classpath_xml = 
r#"<?xml version="1.0" encoding="UTF-8"?>
<classpath>
	<classpathentry kind="con" path="org.eclipse.jdt.launching.JRE_CONTAINER/org.eclipse.jdt.internal.debug.ui.launcher.StandardVMType/JavaSE-17">
		<attributes>
			<attribute name="maven.pomderived" value="true"/>
		</attributes>
	</classpathentry>
	<classpathentry kind="src" output="target/classes" path="src">
		<attributes>
			<attribute name="optional" value="true"/>
			<attribute name="maven.pomderived" value="true"/>
		</attributes>
	</classpathentry>
	<classpathentry kind="con" path="org.eclipse.m2e.MAVEN2_CLASSPATH_CONTAINER">
		<attributes>
			<attribute name="maven.pomderived" value="true"/>
		</attributes>
	</classpathentry>
	<classpathentry kind="output" path="target/classes"/>
</classpath>"#;

    let project_xml = 
r#"<?xml version="1.0" encoding="UTF-8"?>
<projectDescription>
    <name>%PROJECT_NAME%</name>
    <comment></comment>
    <projects></projects>
    <buildSpec>
        <buildCommand>
            <name>org.eclipse.jdt.core.javabuilder</name>
            <arguments></arguments>
        </buildCommand>
        <buildCommand>
            <name>org.eclipse.m2e.core.maven2builder</name>
            <arguments></arguments>
        </buildCommand>
    </buildSpec>
    <natures>
        <nature>org.eclipse.m2e.core.maven2Nature</nature>
        <nature>org.eclipse.jdt.core.javanature</nature>
    </natures>
</projectDescription>"#
    .replace("%PROJECT_NAME%", project_name);

    let pom_xml = 
r#"<project xmlns="http://maven.apache.org/POM/4.0.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema/instance" xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 https://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>
    <groupId>%PACKAGE%</groupId>
    <artifactId>%PROJECT_NAME%</artifactId>
    <version>0.0.1-SNAPSHOT</version>
    <build>
        <sourceDirectory>src</sourceDirectory>
        <plugins>
            <plugin>
                <artifactId>maven-compiler-plugin</artifactId>
                <version>3.8.1</version>
                <configuration>
                    <release>17</release>
                </configuration>
            </plugin>
        </plugins>
    </build>
</project>"#
    .replace("%PROJECT_NAME%", project_name).replace("%PACKAGE%", package);

    let eclipse_prefs = 
r#"eclipse.preferences.version=1
org.eclipse.jdt.core.compiler.codegen.inlineJsrBytecode=enabled
org.eclipse.jdt.core.compiler.codegen.targetPlatform=17
org.eclipse.jdt.core.compiler.codegen.unusedLocal=preserve
org.eclipse.jdt.core.compiler.compliance=17
org.eclipse.jdt.core.compiler.debug.lineNumber=generate
org.eclipse.jdt.core.compiler.debug.localVariable=generate
org.eclipse.jdt.core.compiler.debug.sourceFile=generate
org.eclipse.jdt.core.compiler.problem.assertIdentifier=error
org.eclipse.jdt.core.compiler.problem.enablePreviewFeatures=disabled
org.eclipse.jdt.core.compiler.problem.enumIdentifier=error
org.eclipse.jdt.core.compiler.problem.forbiddenReference=warning
org.eclipse.jdt.core.compiler.problem.reportPreviewFeatures=warning
org.eclipse.jdt.core.compiler.release=enabled
org.eclipse.jdt.core.compiler.source=17"#;

    let eclipse_core_prefs = 
r#"activeProfiles=
eclipse.preferences.version=1
resolveWorkspaceProjects=true
version=1"#;

    let starter_file = 
r#"package %PACKAGE%;

public class %PROJECT_NAME%
{

}"#
    .replace("%PACKAGE%", package).replace("%PROJECT_NAME%", project_name);

    let mut current_directory = env::current_dir().unwrap();
    match fs::create_dir(PathBuf::from(project_name.to_owned()))
    {
        Ok(_) => { },
        Err(e) => {
            println!("Could not create project directory. Error: {}", e);
            exit(1);
        }
    }

    current_directory.push(PathBuf::from(project_name.to_owned()));
    env::set_current_dir(&current_directory).unwrap();

    let mut source_path = String::from("src/");
    source_path.push_str(&package.replace(".", "/"));
    source_path.push('/');

    fs::create_dir_all(PathBuf::from(&source_path)).unwrap();
    fs::create_dir(PathBuf::from(String::from("lib/"))).unwrap();
    fs::create_dir(PathBuf::from(String::from(".settings/"))).unwrap();

    source_path.push_str(&project_name);
    source_path.push_str(".java");

    match fs::write(&source_path.as_str(), starter_file)
    {
        Ok(_) => { },
        Err(_e) => todo!()
    };

    match fs::write("project.toml", project_toml)
    {
        Ok(_) => println!("Created a new project.toml in project directory."),
        Err(_e) => {
            todo!()
        }
    };

    match fs::write(".project", project_xml)
    {
        Ok(_) => println!("Created a new .project file in project directory."),
        Err(_e) => {
            todo!()
        }
    };

    match fs::write(".classpath", classpath_xml)
    {
        Ok(_) => println!("Created a new .classpath file in project directory."),
        Err(_e) => {
            todo!()
        }
    };

    match fs::write("pom.xml", pom_xml)
    {
        Ok(_) => println!("Created a new Maven pom.xml in the project directory."),
        Err(_e) => {
            todo!()
        }
    };

    match fs::write(".settings/org.eclipse.jdt.core.prefs", eclipse_prefs)
    {
        Ok(_) => println!("Created new org.eclipse.jdt.core.prefs in .settings directory."),
        Err(_e) => {
            todo!()
        }
    };

    match fs::write(".settings/org.eclipse.m2e.core.prefs", eclipse_core_prefs)
    {
        Ok(_) => println!("Created new org.eclipse.m2e.core.prefs in .settings directory."),
        Err(_e) => {
            todo!()
        }
    };

    println!("Successfully created new project {}", project_name);
}

fn create_classpath(libraries: &Vec<String>)
{
    fs::remove_file(".classpath").unwrap_or(());
    
    let mut library_string = String::new();
    for l in libraries
    {
        library_string.push_str(&format!("<classpathentry kind=\"lib\" path=\"{}\"/>\n", l).to_owned());
    }

    library_string.pop();

    let classpath_xml = 
r#"<?xml version="1.0" encoding="UTF-8"?>
<classpath>
	<classpathentry kind="con" path="org.eclipse.jdt.launching.JRE_CONTAINER/org.eclipse.jdt.internal.debug.ui.launcher.StandardVMType/JavaSE-17">
		<attributes>
			<attribute name="maven.pomderived" value="true"/>
		</attributes>
	</classpathentry>
	<classpathentry kind="src" output="target/classes" path="src">
		<attributes>
			<attribute name="optional" value="true"/>
			<attribute name="maven.pomderived" value="true"/>
		</attributes>
	</classpathentry>
    %LIBRARIES%
	<classpathentry kind="con" path="org.eclipse.m2e.MAVEN2_CLASSPATH_CONTAINER">
		<attributes>
			<attribute name="maven.pomderived" value="true"/>
		</attributes>
	</classpathentry>
	<classpathentry kind="output" path="target/classes"/>
</classpath>"#
    .replace("%LIBRARIES%", &library_string);

    fs::write(".classpath", classpath_xml).unwrap_or(());
}
