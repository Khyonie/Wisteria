use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use toml::{Table, Value};

use crate::util::consts;

#[derive(Debug, PartialEq, Eq)]
pub struct Wisteria2Conversion {
    pub project_toml: String,
    pub dependency_count: usize,
    pub configuration_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Wisteria2Migration {
    pub backup_path: PathBuf,
    pub dependency_count: usize,
    pub configuration_count: usize,
    pub warnings: Vec<String>,
}

enum LocalDependencyKind {
    Archive,
    Folder,
}

#[derive(Serialize)]
struct MigratedProjectToml {
    project: Table,
    #[serde(skip_serializing_if = "Option::is_none")]
    dependencies: Option<Table>,
    #[serde(skip_serializing_if = "Option::is_none")]
    configuration: Option<Table>,
}

pub fn migrate_wisteria2_project_file(
    project_file: &Path,
) -> Result<Wisteria2Migration, (String, u8)> {
    let project_toml_string = fs::read_to_string(project_file).map_err(|e| {
        (
            format!(
                "Failed to read Wisteria 2 project file {}: {e}",
                project_file.to_string_lossy()
            ),
            1,
        )
    })?;

    let project_root = project_file.parent().unwrap_or_else(|| Path::new("."));
    let conversion = convert_wisteria2_project_toml(&project_toml_string, project_root)?;
    let backup_path = next_backup_path(project_file);

    fs::copy(project_file, &backup_path).map_err(|e| {
        (
            format!(
                "Failed to create backup {}: {e}",
                backup_path.to_string_lossy()
            ),
            1,
        )
    })?;

    fs::write(project_file, conversion.project_toml).map_err(|e| {
        (
            format!(
                "Failed to write converted project file {}: {e}",
                project_file.to_string_lossy()
            ),
            1,
        )
    })?;

    Ok(Wisteria2Migration {
        backup_path,
        dependency_count: conversion.dependency_count,
        configuration_count: conversion.configuration_count,
        warnings: conversion.warnings,
    })
}

pub fn convert_wisteria2_project_toml(
    project_toml_string: &str,
    project_root: &Path,
) -> Result<Wisteria2Conversion, (String, u8)> {
    let legacy_project_toml = project_toml_string
        .parse::<Table>()
        .map_err(|e| (format!("Could not parse Wisteria 2 project.toml: {e}"), 70))?;

    let Some(project) = legacy_project_toml.get("project").and_then(Value::as_table) else {
        return Err((
            String::from("Cannot migrate Wisteria 2 project.toml without a [project] table"),
            70,
        ));
    };

    let mut warnings = Vec::new();
    let mut migrated_project = Table::new();
    migrated_project.insert(
        String::from("name"),
        Value::String(read_string(project, "name").unwrap_or_else(|| {
            warnings.push(String::from(
                "Missing project.name; inferred the project name from the project directory.",
            ));
            infer_project_name(project_root)
        })),
    );
    migrated_project.insert(
        String::from("version"),
        Value::String(read_string(project, "version").unwrap_or_else(|| String::from("0.1.0"))),
    );
    migrated_project.insert(
        String::from("description"),
        Value::String(
            read_string(project, "description")
                .unwrap_or_else(|| String::from("Migrated from Wisteria 2.")),
        ),
    );

    copy_optional_string_or_array(project, &mut migrated_project, "authors", &mut warnings);
    copy_optional_string_or_array(project, &mut migrated_project, "license", &mut warnings);
    copy_optional_string(project, &mut migrated_project, "homepage", &mut warnings);
    copy_optional_string(project, &mut migrated_project, "sourcepage", &mut warnings);

    migrated_project.insert(
        String::from("natures"),
        read_string_array(project, "natures")
            .map(string_array)
            .unwrap_or_else(|| string_array(vec![String::from("eclipse"), String::from("maven")])),
    );

    let libraries = read_project_libraries(project, project_root, &mut warnings);
    let (dependencies, dependency_names) = convert_libraries(&libraries, project_root);
    let configurations = convert_tasks(
        legacy_project_toml.get("task"),
        &dependency_names,
        &mut warnings,
    )?;

    let dependency_count = dependency_names.len();
    let configuration_count = configurations.len();
    let migrated_project_toml = MigratedProjectToml {
        project: migrated_project,
        dependencies: (!dependencies.is_empty()).then_some(dependencies),
        configuration: (!configurations.is_empty()).then_some(configurations),
    };

    let project_toml =
        toml::to_string_pretty(&migrated_project_toml).map_err(|e| (format!("{e}"), 70))?;

    Ok(Wisteria2Conversion {
        project_toml,
        dependency_count,
        configuration_count,
        warnings,
    })
}

fn read_project_libraries(
    project: &Table,
    project_root: &Path,
    warnings: &mut Vec<String>,
) -> Vec<String> {
    match project.get("libraries") {
        Some(_) => match read_string_array(project, "libraries") {
            Some(libraries) => libraries,
            None => {
                warnings.push(String::from(
                    "Skipped project.libraries; expected a string or string array.",
                ));
                Vec::new()
            }
        },
        None if project_root.join(consts::PROJECT_LIBRARY_DIR).exists() => {
            vec![format!("{}/", consts::PROJECT_LIBRARY_DIR)]
        }
        None => {
            warnings.push(String::from(
                "Missing project.libraries and no lib/ folder exists; no dependencies were generated.",
            ));
            Vec::new()
        }
    }
}

fn convert_libraries(libraries: &[String], project_root: &Path) -> (Table, Vec<String>) {
    let mut dependencies = Table::new();
    let mut archives = Table::new();
    let mut folders = Table::new();
    let mut used_names = HashSet::new();
    let mut dependency_names = Vec::new();

    for library in libraries {
        let name = unique_dependency_name(library, &mut used_names);
        let mut dependency = Table::new();
        dependency.insert(String::from("path"), Value::String(library.clone()));

        match infer_local_dependency_kind(library, project_root) {
            LocalDependencyKind::Archive => {
                archives.insert(name.clone(), Value::Table(dependency));
            }
            LocalDependencyKind::Folder => {
                dependency.insert(String::from("recursive"), Value::Boolean(true));
                folders.insert(name.clone(), Value::Table(dependency));
            }
        }

        dependency_names.push(name);
    }

    if !archives.is_empty() {
        dependencies.insert(String::from("archive"), Value::Table(archives));
    }
    if !folders.is_empty() {
        dependencies.insert(String::from("folder"), Value::Table(folders));
    }

    (dependencies, dependency_names)
}

fn convert_tasks(
    tasks: Option<&Value>,
    dependency_names: &[String],
    warnings: &mut Vec<String>,
) -> Result<Table, (String, u8)> {
    let Some(tasks) = tasks else {
        warnings.push(String::from(
            "No [task] table was found; no configurations were generated.",
        ));
        return Ok(Table::new());
    };

    let Some(tasks) = tasks.as_table() else {
        return Err((
            format!(
                "Cannot migrate Wisteria 2 task definitions; expected [task] to be a table, found {}",
                tasks.type_str()
            ),
            70,
        ));
    };

    let mut configurations = Table::new();

    for (task_name, task_value) in tasks {
        let Some(task) = task_value.as_table() else {
            warnings.push(format!(
                "Skipping task {task_name}; expected a table, found {}.",
                task_value.type_str()
            ));
            continue;
        };

        let mut configuration = Table::new();
        let sources = read_string_array(task, "source")
            .or_else(|| read_string_array(task, "sources"))
            .unwrap_or_else(|| vec![String::from("src/")]);
        configuration.insert(String::from("sources"), string_array(sources));

        if !dependency_names.is_empty() {
            configuration.insert(
                String::from("dependencies"),
                string_array(dependency_names.to_vec()),
            );
        }

        let targets = read_string_array(task, "output")
            .or_else(|| read_string_array(task, "targets"))
            .unwrap_or_else(|| vec![String::from("target/build/{TASK}/{NAME}.jar")])
            .into_iter()
            .map(convert_wisteria2_path_template)
            .collect();
        configuration.insert(String::from("targets"), string_array(targets));

        if let Some(entry) = read_string(task, "entry") {
            configuration.insert(String::from("entry"), Value::String(entry));
        }

        if let Some(includes) = read_string_array(task, "input").filter(|input| !input.is_empty()) {
            configuration.insert(String::from("includes"), string_array(includes));
        }

        let mut java_version = 17;
        if let Some(arguments) = read_string(task, "arguments") {
            let flags = convert_compiler_arguments(&arguments, &mut java_version, warnings);
            if !flags.is_empty() {
                configuration.insert(String::from("compiler_flags"), Value::Table(flags));
            }
        }

        configuration.insert(String::from("java_version"), Value::Integer(java_version));
        configurations.insert(task_name.clone(), Value::Table(configuration));
    }

    if configurations.is_empty() {
        warnings.push(String::from(
            "No task definitions could be converted; no configurations were generated.",
        ));
    }

    Ok(configurations)
}

fn convert_compiler_arguments(
    arguments: &str,
    java_version: &mut i64,
    warnings: &mut Vec<String>,
) -> Table {
    let mut flags = Table::new();
    let tokens: Vec<&str> = arguments.split_whitespace().collect();
    let mut index = 0;

    while index < tokens.len() {
        let token = tokens[index];

        match token {
            "-parameters" => {
                flags.insert(String::from("store_parameter_names"), Value::Boolean(true));
                index += 1;
            }
            "-nowarn" => {
                flags.insert(String::from("no_warnings"), Value::Boolean(true));
                index += 1;
            }
            "-deprecation" => {
                flags.insert(String::from("deprecation_info"), Value::Boolean(true));
                index += 1;
            }
            "--enable-preview" => {
                flags.insert(
                    String::from("enable_preview_features"),
                    Value::Boolean(true),
                );
                index += 1;
            }
            "--release" => {
                index += parse_u8_argument(
                    "release_target",
                    &tokens,
                    index,
                    &mut flags,
                    java_version,
                    warnings,
                );
            }
            "--encoding" | "-encoding" => {
                index +=
                    parse_string_argument("source_encoding", &tokens, index, &mut flags, warnings);
            }
            _ if token.starts_with("--release=") => {
                parse_release_value(
                    token.trim_start_matches("--release="),
                    &mut flags,
                    java_version,
                    warnings,
                );
                index += 1;
            }
            _ if token.starts_with("--encoding=") => {
                flags.insert(
                    String::from("source_encoding"),
                    Value::String(token.trim_start_matches("--encoding=").to_string()),
                );
                index += 1;
            }
            _ if token == "-Xlint:all" => {
                flags.insert(String::from("source_all_lints"), Value::Boolean(true));
                index += 1;
            }
            _ if token.starts_with("-Xlint:") => {
                insert_lint_argument(
                    "source_lints",
                    token,
                    token.trim_start_matches("-Xlint:"),
                    &mut flags,
                    warnings,
                );
                index += 1;
            }
            _ if token == "-Xdoclint:all" => {
                flags.insert(String::from("javadoc_all_lints"), Value::Boolean(true));
                index += 1;
            }
            _ if token.starts_with("-Xdoclint:") => {
                insert_lint_argument(
                    "javadoc_lints",
                    token,
                    token.trim_start_matches("-Xdoclint:"),
                    &mut flags,
                    warnings,
                );
                index += 1;
            }
            _ => {
                warnings.push(format!(
                    "Skipped unsupported javac argument \"{token}\" from Wisteria 2 task arguments."
                ));
                index += 1;
            }
        }
    }

    flags
}

fn insert_lint_argument(
    flag_name: &str,
    token: &str,
    value: &str,
    flags: &mut Table,
    warnings: &mut Vec<String>,
) {
    let lints = split_lints(value);
    if lints.is_empty() {
        warnings.push(format!(
            "Skipped javac argument \"{token}\" because it did not include any lints."
        ));
        return;
    }

    flags.insert(String::from(flag_name), string_array(lints));
}

fn parse_u8_argument(
    flag_name: &str,
    tokens: &[&str],
    index: usize,
    flags: &mut Table,
    java_version: &mut i64,
    warnings: &mut Vec<String>,
) -> usize {
    let Some(value) = tokens.get(index + 1) else {
        warnings.push(format!(
            "Skipped javac argument \"{}\" because it did not have a value.",
            tokens[index]
        ));
        return 1;
    };

    match value.parse::<u8>() {
        Ok(value) => {
            *java_version = i64::from(value);
            flags.insert(String::from(flag_name), Value::Integer(i64::from(value)));
            2
        }
        Err(_) => {
            warnings.push(format!(
                "Skipped javac argument \"{} {value}\" because {value} is not a Java version.",
                tokens[index]
            ));
            2
        }
    }
}

fn parse_string_argument(
    flag_name: &str,
    tokens: &[&str],
    index: usize,
    flags: &mut Table,
    warnings: &mut Vec<String>,
) -> usize {
    let Some(value) = tokens.get(index + 1) else {
        warnings.push(format!(
            "Skipped javac argument \"{}\" because it did not have a value.",
            tokens[index]
        ));
        return 1;
    };

    flags.insert(String::from(flag_name), Value::String((*value).to_string()));
    2
}

fn parse_release_value(
    value: &str,
    flags: &mut Table,
    java_version: &mut i64,
    warnings: &mut Vec<String>,
) {
    match value.parse::<u8>() {
        Ok(value) => {
            *java_version = i64::from(value);
            flags.insert(
                String::from("release_target"),
                Value::Integer(i64::from(value)),
            );
        }
        Err(_) => warnings.push(format!(
            "Skipped javac argument \"--release={value}\" because {value} is not a Java version."
        )),
    }
}

fn infer_local_dependency_kind(library: &str, project_root: &Path) -> LocalDependencyKind {
    let library_path = project_root.join(library);

    if library_path.is_file() {
        return LocalDependencyKind::Archive;
    }

    if library_path.is_dir() {
        return LocalDependencyKind::Folder;
    }

    if library.to_lowercase().ends_with(".jar") {
        LocalDependencyKind::Archive
    } else {
        LocalDependencyKind::Folder
    }
}

fn next_backup_path(project_file: &Path) -> PathBuf {
    let backup_name = format!(
        "{}.{}",
        project_file
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_else(|| consts::PROJECT_FILE.into()),
        consts::WISTERIA2_BACKUP_EXTENSION
    );
    let backup_path = project_file.with_file_name(backup_name);

    if !backup_path.exists() {
        return backup_path;
    }

    for index in 1.. {
        let candidate = backup_path.with_extension(format!(
            "{}.{index}",
            backup_path
                .extension()
                .map(|extension| extension.to_string_lossy())
                .unwrap_or_default()
        ));

        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!()
}

fn unique_dependency_name(library: &str, used_names: &mut HashSet<String>) -> String {
    let base_name = sanitize_dependency_name(library);
    let mut name = base_name.clone();
    let mut index = 2;

    while used_names.contains(&name) {
        name = format!("{base_name}-{index}");
        index += 1;
    }

    used_names.insert(name.clone());
    name
}

fn sanitize_dependency_name(library: &str) -> String {
    let mut component = library
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("library")
        .to_string();

    if component.to_lowercase().ends_with(".jar") {
        let new_len = component.len().saturating_sub(4);
        component.truncate(new_len);
    }

    let mut sanitized = String::new();
    for character in component.chars() {
        if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
            sanitized.push(character.to_ascii_lowercase());
        } else {
            sanitized.push('-');
        }
    }

    let sanitized = sanitized.trim_matches('-');
    if sanitized.is_empty() {
        String::from("library")
    } else {
        sanitized.to_string()
    }
}

fn convert_wisteria2_path_template(path: String) -> String {
    path.replace("{NAME}", "{project_name}")
        .replace("{TASK}", "{configuration}")
}

fn infer_project_name(project_root: &Path) -> String {
    project_root
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("MigratedProject"))
}

fn copy_optional_string(from: &Table, to: &mut Table, key: &str, warnings: &mut Vec<String>) {
    match from.get(key) {
        Some(value) if value.is_str() => {
            to.insert(key.to_string(), value.clone());
        }
        Some(value) => warnings.push(format!(
            "Skipped project.{key}; expected a string, found {}.",
            value.type_str()
        )),
        None => {}
    }
}

fn copy_optional_string_or_array(
    from: &Table,
    to: &mut Table,
    key: &str,
    warnings: &mut Vec<String>,
) {
    match from.get(key) {
        Some(value) if value.is_str() => {
            to.insert(key.to_string(), value.clone());
        }
        Some(value) if value.is_array() && read_string_array(from, key).is_some() => {
            to.insert(key.to_string(), value.clone());
        }
        Some(value) => warnings.push(format!(
            "Skipped project.{key}; expected a string or string array, found {}.",
            value.type_str()
        )),
        None => {}
    }
}

fn read_string(toml: &Table, key: &str) -> Option<String> {
    toml.get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn read_string_array(toml: &Table, key: &str) -> Option<Vec<String>> {
    match toml.get(key) {
        Some(value) if value.is_str() => Some(vec![value.as_str().unwrap().to_string()]),
        Some(value) if value.is_array() => {
            let mut values = Vec::new();
            for item in value.as_array().unwrap() {
                let item = item.as_str()?;
                values.push(item.to_string());
            }
            Some(values)
        }
        _ => None,
    }
}

fn string_array(values: Vec<String>) -> Value {
    Value::Array(values.into_iter().map(Value::String).collect())
}

fn split_lints(lints: &str) -> Vec<String> {
    lints
        .split(',')
        .filter(|lint| !lint.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn converts_wisteria2_project_to_grouped_wisteria3_config() {
        let temp = TempDir::new("migrate-wisteria2-convert");
        fs::create_dir_all(temp.path().join("lib/nested")).unwrap();
        fs::write(temp.path().join("lib/nested/local.jar"), "").unwrap();
        fs::create_dir_all(temp.path().join("external")).unwrap();
        fs::create_dir_all(temp.path().join("plugins")).unwrap();
        fs::write(temp.path().join("plugins/example.jar"), "").unwrap();

        let conversion = convert_wisteria2_project_toml(
            r#"
            [project]
            name = "Legacy"
            libraries = [ "lib/", "plugins/example.jar", "external" ]

            [task.main]
            source = "src/"
            output = [ "target/{TASK}/{NAME}.jar" ]
            input = [ "plugin.yml" ]
            entry = "example.Main"
            arguments = "--release 17 -parameters -Xlint:unchecked -unsupported"
            "#,
            temp.path(),
        )
        .unwrap();

        let migrated = conversion.project_toml.parse::<Table>().unwrap();
        let project = migrated.get("project").unwrap().as_table().unwrap();
        assert_eq!(project.get("name").and_then(Value::as_str), Some("Legacy"));
        assert_eq!(
            project.get("version").and_then(Value::as_str),
            Some("0.1.0")
        );
        assert_eq!(
            project.get("description").and_then(Value::as_str),
            Some("Migrated from Wisteria 2.")
        );

        let dependencies = migrated.get("dependencies").unwrap().as_table().unwrap();
        assert!(dependencies
            .get("folder")
            .unwrap()
            .as_table()
            .unwrap()
            .contains_key("lib"));
        assert!(dependencies
            .get("folder")
            .unwrap()
            .as_table()
            .unwrap()
            .contains_key("external"));
        assert!(dependencies
            .get("archive")
            .unwrap()
            .as_table()
            .unwrap()
            .contains_key("example"));

        let main = migrated
            .get("configuration")
            .unwrap()
            .as_table()
            .unwrap()
            .get("main")
            .unwrap()
            .as_table()
            .unwrap();
        assert_eq!(
            main.get("sources"),
            Some(&string_array(vec![String::from("src/")]))
        );
        assert_eq!(
            main.get("targets"),
            Some(&string_array(vec![String::from(
                "target/{configuration}/{project_name}.jar"
            )]))
        );
        assert_eq!(
            main.get("dependencies"),
            Some(&string_array(vec![
                String::from("lib"),
                String::from("example"),
                String::from("external")
            ]))
        );
        assert_eq!(
            main.get("entry").and_then(Value::as_str),
            Some("example.Main")
        );
        assert_eq!(
            main.get("java_version").and_then(Value::as_integer),
            Some(17)
        );

        let compiler_flags = main.get("compiler_flags").unwrap().as_table().unwrap();
        assert_eq!(
            compiler_flags
                .get("release_target")
                .and_then(Value::as_integer),
            Some(17)
        );
        assert_eq!(
            compiler_flags
                .get("store_parameter_names")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(conversion
            .warnings
            .iter()
            .any(|warning| warning.contains("-unsupported")));
    }

    #[test]
    fn migrates_project_file_and_keeps_non_overwriting_backup() {
        let temp = TempDir::new("migrate-wisteria2-file");
        fs::create_dir_all(temp.path().join("lib")).unwrap();
        let project_file = temp.path().join("project.toml");
        fs::write(
            &project_file,
            r#"
            [project]
            name = "Legacy"
            libraries = [ "lib/" ]

            [task.main]
            source = "src/"
            "#,
        )
        .unwrap();
        fs::write(temp.path().join("project.toml.wisteria2.bak"), "existing").unwrap();

        let migration = migrate_wisteria2_project_file(&project_file).unwrap();

        assert_eq!(
            migration.backup_path,
            temp.path().join("project.toml.wisteria2.bak.1")
        );
        assert!(migration.backup_path.exists());
        assert_eq!(
            fs::read_to_string(temp.path().join("project.toml.wisteria2.bak")).unwrap(),
            "existing"
        );
        assert!(fs::read_to_string(&project_file)
            .unwrap()
            .contains("[configuration.main]"));
    }
}
