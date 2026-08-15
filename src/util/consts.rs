use std::env::consts;

pub const USAGE_TEXT: &str = r#"Usage: wisteria <(tasks...) | refresh | new | sync | fetch | verify | update | info | switch | migrate > 
    --output <auto | plain | terminal | json>
        Controls command output. JSON output is emitted as newline-delimited JSON events.
    (tasks...)
        Runs the specified tasks, in order, and at least one task must be given

        Note that if any task fails, the process will not continue. 
        More complex behavior ("on-fail") can be configured in project.toml.
    refresh 
        Configures the project environment using the current configuration
    new <name>
        Creates a new project with the given name
    sync [(dependencies...) | all]
        Updates wisteria.lock from project.toml without downloading artifacts
    fetch [(dependencies...) | all]
        Downloads cached artifacts to match wisteria.lock without changing the lockfile
    verify
        Checks that project.toml, wisteria.lock, and cached dependencies agree
    update <(dependencies...) | all>
        Re-fetches the given dependencies, or all dependencies in a project file
    clean <classes | dependencies | targets | javadocs | metadata | natures | all>
        Removes generated Wisteria output for the selected target
    info
        Displays project information in a human-friendly format
    switch <configuration>
        Switches the current project configuration and configures the project environment
    migrate wisteria2
        Converts a Wisteria 2 project.toml to the current format and writes a backup first

Flags:
    --norefresh
        Skips refreshing the project configuration
        (switch)
    --minimal
        Uses a minimal project.toml template
        (new)
    --project <project file>
        Uses a specific project file"#;

pub const PROJECT_FILE: &str = "project.toml";
pub const WISTERIA2_BACKUP_EXTENSION: &str = "wisteria2.bak";

pub const WISTERIA_DIR: &str = ".wisteria";
pub const LOCKFILE: &str = "wisteria.lock";
pub const LOCKFILE_TEMP: &str = "wisteria.lock.tmp";
pub const TEMP_FILE_EXTENSION: &str = "tmp";
pub const METADATA_FILE: &str = ".wisteria/metadata.toml";
pub const CACHE_PATH: &str = ".wisteria/cache";
pub const WORK_DIR: &str = ".wisteria/work";
pub const SOURCE_OUT_PATH: &str = ".wisteria/work/src";
pub const BINARY_OUT_PATH: &str = ".wisteria/work/bin";
pub const SHADED_OUT_PATH: &str = ".wisteria/work/shaded";
pub const TARGET_JAR_PATH: &str = ".wisteria/work/target.jar";
pub const MANIFEST_DIR: &str = ".wisteria/work/bin/META-INF";
pub const MANIFEST_FILE: &str = ".wisteria/work/bin/META-INF/MANIFEST.MF";

pub const PROJECT_SOURCE_DIR: &str = "src";
pub const LEGACY_PROJECT_LIBRARY_DIR: &str = "lib";

pub const DEFAULT_JAVADOC_DIR: &str = "target/javadoc/{configuration}/";

pub const ECLIPSE_SETTINGS_DIR: &str = ".settings";
pub const ECLIPSE_JDT_PREFS_FILE: &str = ".settings/org.eclipse.jdt.core.prefs";
pub const ECLIPSE_M2E_PREFS_FILE: &str = ".settings/org.eclipse.m2e.core.prefs";
pub const ECLIPSE_PROJECT_FILE: &str = ".project";
pub const ECLIPSE_CLASSPATH_FILE: &str = ".classpath";
pub const ECLIPSE_TARGET_CLASSES_PATH: &str = "target/classes";
pub const ECLIPSE_TARGET_CLASSES_DIR: &str = "target/classes/";

pub const MAVEN_POM_FILE: &str = "pom.xml";

pub fn java_seperator() -> char {
    match consts::OS {
        "windows" => ';',
        _ => ':',
    }
}
