use std::env::consts;

pub const USAGE_TEXT: &str = 
r#"Usage: wisteria <(tasks...) | refresh | new | update | info | switch | warranty | license> 
    (tasks...)
        Runs the specified tasks, in order, and at least one task must be given

        Note that if any task fails, the process will not continue. 
        More complex behavior ("on-fail") can be configured in project.toml.
    refresh 
        Configures the project environment using the current configuration
    new <name>
        Creates a new project with the given name
    update <(dependencies...) | all>
        Re-fetches the given dependencies, or all dependencies in a project file
    info
        Displays project information in a human-friendly format
    switch <configuration>
        Switches the current project configuration and configures the project environment

Flags:
    --no-refresh
        Skips refreshing the project configuration
        (switch)
    --minimal
        Uses a minimal project.toml template
        (new)
    --project <project file>
        Uses a specific project file"#;

pub fn java_seperator() -> char
{
    match consts::OS
    {
        "windows" => ';',
        _ => ':'
    }
}

pub fn print_action_header(message: &str, action: u32, total: u32)
{
    println!("-=-=-=-=-=-=-=-=-=-=-=-=-=-=-<( {:^19} <{:>2}/{:<2}> )>-=-=-=-=-=-=-=-=-=-=-=-=-=-=-", message, action, total);
}
