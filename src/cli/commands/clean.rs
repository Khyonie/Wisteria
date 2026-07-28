use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    process::exit,
};

use crate::{
    cli::commands::envvar_regexes,
    generators::generate_metadata,
    model::{Configuration, Metadata, Project},
    util::consts,
    workspace::{nature::Nature, paths::resolve_filepath},
};

const VALID_CLEAN_TARGETS: &str =
    "[ classes, dependencies, targets, javadocs, metadata, natures, all ]";

pub fn trigger_clean(project: Result<Project, (String, u8)>, args: &[String]) {
    let result = match args[2].to_lowercase().as_str() {
        "classes" => clean_directory(
            consts::BINARY_OUT_PATH,
            "Binary folder does not exist, nothing to do.",
            "Could not remove classes folder",
        ),
        "dependencies" => clean_directory(
            consts::CACHE_PATH,
            "Dependency cache folder does not exist, nothing to do.",
            "Could not remove dependency folder",
        ),
        "targets" | "jars" | "jar" => clean_configuration_targets(project),
        "javadocs" | "javadoc" => clean_configuration_javadocs(project),
        "metadata" => clean_metadata(),
        "natures" => clean_natures(),
        "all" => clean_all(project),
        _ => Err((
            format!(
                "Unknown clean target {}\nValid clean targets: one of {VALID_CLEAN_TARGETS}",
                args[2]
            ),
            1,
        )),
    };

    match result {
        Ok(_) => {
            println!("Operation complete.");
            exit(0)
        }
        Err((message, code)) => {
            println!("{message}");
            exit(code.into())
        }
    }
}

fn clean_all(project: Result<Project, (String, u8)>) -> Result<(), (String, u8)> {
    let paths = configured_clean_paths(project, true, true)?;

    clean_directory(
        consts::BINARY_OUT_PATH,
        "Binary folder does not exist, nothing to do.",
        "Could not remove classes folder",
    )?;
    clean_directory(
        consts::CACHE_PATH,
        "Dependency cache folder does not exist, nothing to do.",
        "Could not remove dependency folder",
    )?;
    clean_paths(paths, "configured targets or javadocs")?;
    clean_metadata()?;
    clean_natures()
}

fn clean_configuration_targets(project: Result<Project, (String, u8)>) -> Result<(), (String, u8)> {
    let paths = configured_clean_paths(project, true, false)?;

    clean_paths(paths, "configured jar targets")
}

fn clean_configuration_javadocs(
    project: Result<Project, (String, u8)>,
) -> Result<(), (String, u8)> {
    let paths = configured_clean_paths(project, false, true)?;

    clean_paths(paths, "configured javadocs")
}

fn configured_clean_paths(
    project: Result<Project, (String, u8)>,
    include_targets: bool,
    include_javadocs: bool,
) -> Result<Vec<String>, (String, u8)> {
    let project: Project = project.map_err(|e| {
        (
            format!(
                "Could not read a Wisteria project.toml file in this directory. ({})",
                e.0
            ),
            e.1,
        )
    })?;
    let metadata = load_clean_metadata()?;
    let configuration = project
        .info()
        .configurations()
        .get(&metadata.configuration)
        .ok_or((
            format!("No such configuration \"{}\".", metadata.configuration),
            1,
        ))?;

    resolve_configuration_paths(configuration, include_targets, include_javadocs)
}

fn load_clean_metadata() -> Result<Metadata, (String, u8)> {
    if PathBuf::from(consts::METADATA_FILE).exists() {
        Metadata::load()
    } else {
        Ok(Metadata::default())
    }
}

fn resolve_configuration_paths(
    configuration: &Configuration,
    include_targets: bool,
    include_javadocs: bool,
) -> Result<Vec<String>, (String, u8)> {
    let regexes = envvar_regexes();
    let mut paths = Vec::new();

    if include_targets {
        if let Some(targets) = configuration.targets() {
            for target in targets {
                paths.push(resolve_filepath(
                    target,
                    configuration.environment(),
                    &regexes,
                )?);
            }
        }

        if let Some(target) = configuration.javadoc_target() {
            paths.push(resolve_filepath(
                target,
                configuration.environment(),
                &regexes,
            )?);
        }
    }

    if include_javadocs {
        paths.push(resolve_filepath(
            configuration.javadoc_output_dir(),
            configuration.environment(),
            &regexes,
        )?);

        if let Some(target) = configuration.javadoc_target() {
            paths.push(resolve_filepath(
                target,
                configuration.environment(),
                &regexes,
            )?);
        }
    }

    Ok(paths)
}

fn clean_directory(
    path: &str,
    missing_message: &str,
    error_prefix: &str,
) -> Result<(), (String, u8)> {
    if !PathBuf::from(path).exists() {
        println!("{missing_message}");
        return Ok(());
    }

    fs::remove_dir_all(path).map_err(|e| (format!("{error_prefix}: {e}"), 1))
}

fn clean_metadata() -> Result<(), (String, u8)> {
    fs::create_dir_all(consts::WISTERIA_DIR)
        .map_err(|e| (format!("Could not create Wisteria metadata folder: {e}"), 1))?;
    fs::write(
        consts::METADATA_FILE,
        generate_metadata(&Metadata::default()),
    )
    .map_err(|e| (format!("Could not reset metadata: {e}"), 1))
}

fn clean_natures() -> Result<(), (String, u8)> {
    for (index, nature) in Nature::values().iter().enumerate() {
        print!(
            "{}/{} Removing nature {}... ",
            index + 1,
            Nature::values().len(),
            nature.type_str()
        );
        match nature.remove_nature() {
            Ok(_) => println!("Done!"),
            Err(e) => return Err((format!("Failed to remove nature: {e}"), 1)),
        }
    }

    Ok(())
}

fn clean_paths(paths: Vec<String>, label: &str) -> Result<(), (String, u8)> {
    let mut seen = HashSet::new();
    let mut removed = 0;

    for path in paths {
        if !seen.insert(path.clone()) {
            continue;
        }

        if remove_path(&path)? {
            removed += 1;
            println!("Removed {path}");
        }
    }

    if removed == 0 {
        println!("No {label} found, nothing to do.");
    }

    Ok(())
}

fn remove_path(path: &str) -> Result<bool, (String, u8)> {
    let path = PathBuf::from(path);
    if path.as_os_str().is_empty() {
        return Err((String::from("Refusing to clean an empty path"), 1));
    }

    if !path.exists() {
        return Ok(false);
    }

    if is_dangerous_clean_path(&path) {
        return Err((
            format!(
                "Refusing to clean dangerous path {}",
                path.to_string_lossy()
            ),
            1,
        ));
    }

    if path.is_dir() {
        fs::remove_dir_all(&path).map_err(|e| {
            (
                format!("Could not remove directory {}: {e}", path.to_string_lossy()),
                1,
            )
        })?;
    } else {
        fs::remove_file(&path).map_err(|e| {
            (
                format!("Could not remove file {}: {e}", path.to_string_lossy()),
                1,
            )
        })?;
    }

    Ok(true)
}

fn is_dangerous_clean_path(path: &Path) -> bool {
    let current_dir = match env::current_dir().and_then(fs::canonicalize) {
        Ok(path) => path,
        Err(_) => return true,
    };
    let canonical_path = match fs::canonicalize(path) {
        Ok(path) => path,
        Err(_) => return true,
    };

    canonical_path == current_dir || canonical_path.parent().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{with_current_dir, TempDir};
    use std::fs;
    use toml::Table;

    fn configuration(toml: &str) -> Configuration {
        Configuration::from(
            String::from("main"),
            &toml.parse::<Table>().unwrap(),
            String::from("Demo"),
            String::from("1.2.3"),
        )
        .unwrap()
    }

    #[test]
    fn resolves_build_and_javadoc_clean_paths() {
        let configuration = configuration(
            r#"
            sources = [ "src/" ]
            targets = [ "target/{configuration}/demo.jar" ]

            [javadoc]
            output-dir = "target/docs/{configuration}/"
            target = "target/{version}/demo-javadocs.jar"
            "#,
        );

        assert_eq!(
            resolve_configuration_paths(&configuration, true, true).unwrap(),
            vec![
                String::from("target/main/demo.jar"),
                String::from("target/1.2.3/demo-javadocs.jar"),
                String::from("target/docs/main/"),
                String::from("target/1.2.3/demo-javadocs.jar"),
            ]
        );
    }

    #[test]
    fn clean_paths_removes_files_and_directories_once() {
        let temp = TempDir::new("clean-paths");

        with_current_dir(temp.path(), || {
            fs::create_dir_all("target/docs").unwrap();
            fs::write("target/docs/index.html", "docs").unwrap();
            fs::create_dir_all("target/jars").unwrap();
            fs::write("target/jars/demo.jar", "jar").unwrap();

            clean_paths(
                vec![
                    String::from("target/docs"),
                    String::from("target/jars/demo.jar"),
                    String::from("target/jars/demo.jar"),
                ],
                "test paths",
            )
            .unwrap();

            assert!(!PathBuf::from("target/docs").exists());
            assert!(!PathBuf::from("target/jars/demo.jar").exists());
        });
    }

    #[test]
    fn clean_paths_refuses_current_directory() {
        let temp = TempDir::new("clean-dangerous-path");

        with_current_dir(temp.path(), || {
            let error = clean_paths(vec![String::from(".")], "test paths").unwrap_err();

            assert!(error.0.contains("Refusing to clean dangerous path"));
            assert_eq!(error.1, 1);
            assert!(temp.path().exists());
        });
    }
}
