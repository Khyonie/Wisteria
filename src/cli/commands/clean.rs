use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    process::exit,
};

use crate::{
    cli::{args::StartupFlags, commands::envvar_regexes},
    generators::generate_metadata,
    model::{Configuration, Metadata, Project},
    output::{self, OutputRenderer},
    util::consts,
    workspace::{nature::Nature, paths::resolve_filepath},
};

const VALID_CLEAN_TARGETS: &str =
    "[ classes, dependencies, targets, javadocs, metadata, natures, all ]";

pub fn trigger_clean(project: Result<Project, String>, args: &[String], flags: &StartupFlags) {
    let mut output = output::renderer(flags.output_mode);
    let result = match args[2].to_lowercase().as_str() {
        "classes" => clean_single_directory(
            output.as_mut(),
            "classes",
            consts::BINARY_OUT_PATH,
            "Could not remove classes folder",
        ),
        "dependencies" => clean_single_directory(
            output.as_mut(),
            "dependency cache",
            consts::CACHE_PATH,
            "Could not remove dependency folder",
        ),
        "targets" | "jars" | "jar" => clean_configuration_targets(project, output.as_mut()),
        "javadocs" | "javadoc" => clean_configuration_javadocs(project, output.as_mut()),
        "metadata" => clean_single_metadata(output.as_mut()),
        "natures" => clean_single_natures(output.as_mut()),
        "all" => clean_all(project, output.as_mut()),
        _ => unknown_clean_target(output.as_mut(), &args[2]),
    };

    match result {
        Ok(message) => {
            output.operation_completed("clean", &message);
            exit(0)
        }
        Err(_message) => {
            output.operation_completed("clean", "Clean finished with errors.");
            exit(1)
        }
    }
}

fn unknown_clean_target(output: &mut dyn OutputRenderer, target: &str) -> Result<String, String> {
    let message =
        format!("Unknown clean target {target}\nValid clean targets: one of {VALID_CLEAN_TARGETS}");

    output.operation_started("clean", 1);
    output.step_failed("clean", "Checking", "target", 1, 1, &message);

    Err(message)
}

fn clean_all(
    project: Result<Project, String>,
    output: &mut dyn OutputRenderer,
) -> Result<String, String> {
    let paths = resolve_clean_paths_or_report(project, true, true, output)?;
    let paths = unique_paths(paths);
    let total = 2 + clean_path_steps(&paths) + 1 + Nature::values().len();
    let mut step = 1;

    output.operation_started("clean", total);
    clean_directory(
        output,
        &mut step,
        total,
        "classes",
        consts::BINARY_OUT_PATH,
        "Could not remove classes folder",
    )?;
    clean_directory(
        output,
        &mut step,
        total,
        "dependency cache",
        consts::CACHE_PATH,
        "Could not remove dependency folder",
    )?;
    clean_paths(
        output,
        &mut step,
        total,
        paths,
        "configured targets or javadocs",
    )?;
    clean_metadata(output, &mut step, total)?;
    clean_natures(output, &mut step, total)?;

    Ok(String::from("Cleaned project"))
}

fn clean_configuration_targets(
    project: Result<Project, String>,
    output: &mut dyn OutputRenderer,
) -> Result<String, String> {
    let paths = resolve_clean_paths_or_report(project, true, false, output)?;
    let paths = unique_paths(paths);
    let summary_count = paths.len();
    let total = clean_path_steps(&paths);
    let mut step = 1;

    output.operation_started("clean", total);
    clean_paths(output, &mut step, total, paths, "configured jar targets")?;

    Ok(format!(
        "Cleaned {summary_count} configured jar {}",
        target_label(summary_count)
    ))
}

fn clean_configuration_javadocs(
    project: Result<Project, String>,
    output: &mut dyn OutputRenderer,
) -> Result<String, String> {
    let paths = resolve_clean_paths_or_report(project, false, true, output)?;
    let paths = unique_paths(paths);
    let summary_count = paths.len();
    let total = clean_path_steps(&paths);
    let mut step = 1;

    output.operation_started("clean", total);
    clean_paths(output, &mut step, total, paths, "configured javadocs")?;

    Ok(format!(
        "Cleaned {summary_count} configured {}",
        javadocs_label(summary_count)
    ))
}

fn resolve_clean_paths_or_report(
    project: Result<Project, String>,
    include_targets: bool,
    include_javadocs: bool,
    output: &mut dyn OutputRenderer,
) -> Result<Vec<String>, String> {
    match configured_clean_paths(project, include_targets, include_javadocs) {
        Ok(paths) => Ok(paths),
        Err(error) => {
            output.operation_started("clean", 1);
            output.step_failed("clean", "Resolving", "configured paths", 1, 1, &error);
            Err(error)
        }
    }
}

fn configured_clean_paths(
    project: Result<Project, String>,
    include_targets: bool,
    include_javadocs: bool,
) -> Result<Vec<String>, String> {
    let project: Project = project.map_err(|e| {
        format!("Could not read a Wisteria project.toml file in this directory. ({e})")
    })?;
    let metadata = load_clean_metadata()?;
    let configuration = project
        .info()
        .configurations()
        .get(&metadata.configuration)
        .ok_or_else(|| format!("No such configuration \"{}\".", metadata.configuration))?;

    resolve_configuration_paths(configuration, include_targets, include_javadocs)
}

fn load_clean_metadata() -> Result<Metadata, String> {
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
) -> Result<Vec<String>, String> {
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

fn clean_single_directory(
    output: &mut dyn OutputRenderer,
    item: &str,
    path: &str,
    error_prefix: &str,
) -> Result<String, String> {
    let total = 1;
    let mut step = 1;

    output.operation_started("clean", total);
    clean_directory(output, &mut step, total, item, path, error_prefix)?;

    Ok(format!("Cleaned {item}"))
}

fn clean_directory(
    output: &mut dyn OutputRenderer,
    step: &mut usize,
    total: usize,
    item: &str,
    path: &str,
    error_prefix: &str,
) -> Result<(), String> {
    output.step_started("clean", "Removing", item, *step, total);

    if !PathBuf::from(path).exists() {
        output.step_completed("clean", "Removing", item, *step, total, "Not found");
        *step += 1;
        return Ok(());
    }

    match fs::remove_dir_all(path) {
        Ok(()) => {
            output.step_completed("clean", "Removing", item, *step, total, "Removed");
            *step += 1;
            Ok(())
        }
        Err(error) => {
            let message = format!("{error_prefix}: {error}");
            output.step_failed("clean", "Removing", item, *step, total, &message);
            Err(message)
        }
    }
}

fn clean_single_metadata(output: &mut dyn OutputRenderer) -> Result<String, String> {
    let total = 1;
    let mut step = 1;

    output.operation_started("clean", total);
    clean_metadata(output, &mut step, total)?;

    Ok(String::from("Reset metadata"))
}

fn clean_metadata(
    output: &mut dyn OutputRenderer,
    step: &mut usize,
    total: usize,
) -> Result<(), String> {
    output.step_started("clean", "Resetting", "metadata", *step, total);

    match reset_metadata_file() {
        Ok(()) => {
            output.step_completed("clean", "Resetting", "metadata", *step, total, "Done");
            *step += 1;
            Ok(())
        }
        Err(error) => {
            output.step_failed("clean", "Resetting", "metadata", *step, total, &error);
            Err(error)
        }
    }
}

fn reset_metadata_file() -> Result<(), String> {
    fs::create_dir_all(consts::WISTERIA_DIR)
        .map_err(|e| format!("Could not create Wisteria metadata folder: {e}"))?;
    fs::write(
        consts::METADATA_FILE,
        generate_metadata(&Metadata::default()),
    )
    .map_err(|e| format!("Could not reset metadata: {e}"))
}

fn clean_single_natures(output: &mut dyn OutputRenderer) -> Result<String, String> {
    let total = Nature::values().len();
    let mut step = 1;

    output.operation_started("clean", total);
    clean_natures(output, &mut step, total)?;

    Ok(format!("Removed {total} {}", nature_label(total)))
}

fn clean_natures(
    output: &mut dyn OutputRenderer,
    step: &mut usize,
    total: usize,
) -> Result<(), String> {
    for nature in Nature::values() {
        let item = format!("{} nature", nature.type_str());
        output.step_started("clean", "Removing", &item, *step, total);

        match nature.remove_nature() {
            Ok(_) => {
                output.step_completed("clean", "Removing", &item, *step, total, "Done");
                *step += 1;
            }
            Err(e) => {
                let message = format!("Failed to remove nature: {e}");
                output.step_failed("clean", "Removing", &item, *step, total, &message);
                return Err(message);
            }
        }
    }

    Ok(())
}

fn clean_paths(
    output: &mut dyn OutputRenderer,
    step: &mut usize,
    total: usize,
    paths: Vec<String>,
    label: &str,
) -> Result<(), String> {
    let paths = unique_paths(paths);

    if paths.is_empty() {
        output.step_started("clean", "Inspecting", label, *step, total);
        output.step_completed(
            "clean",
            "Inspecting",
            label,
            *step,
            total,
            "Nothing to clean",
        );
        *step += 1;
        return Ok(());
    }

    for path in paths {
        output.step_started("clean", "Removing", &path, *step, total);

        match remove_path(&path) {
            Ok(true) => output.step_completed("clean", "Removing", &path, *step, total, "Removed"),
            Ok(false) => {
                output.step_completed("clean", "Removing", &path, *step, total, "Not found")
            }
            Err(error) => {
                output.step_failed("clean", "Removing", &path, *step, total, &error);
                return Err(error);
            }
        }
        *step += 1;
    }

    Ok(())
}

fn unique_paths(paths: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    paths
        .into_iter()
        .filter(|path| seen.insert(path.clone()))
        .collect()
}

fn clean_path_steps(paths: &[String]) -> usize {
    paths.len().max(1)
}

fn target_label(count: usize) -> &'static str {
    match count {
        1 => "target",
        _ => "targets",
    }
}

fn javadocs_label(count: usize) -> &'static str {
    match count {
        1 => "javadoc path",
        _ => "javadoc paths",
    }
}

fn nature_label(count: usize) -> &'static str {
    match count {
        1 => "nature",
        _ => "natures",
    }
}

fn remove_path(path: &str) -> Result<bool, String> {
    let path = PathBuf::from(path);
    if path.as_os_str().is_empty() {
        return Err(String::from("Refusing to clean an empty path"));
    }

    if !path.exists() {
        return Ok(false);
    }

    if is_dangerous_clean_path(&path) {
        return Err(format!(
            "Refusing to clean dangerous path {}",
            path.to_string_lossy()
        ));
    }

    if path.is_dir() {
        fs::remove_dir_all(&path)
            .map_err(|e| format!("Could not remove directory {}: {e}", path.to_string_lossy()))?;
    } else {
        fs::remove_file(&path)
            .map_err(|e| format!("Could not remove file {}: {e}", path.to_string_lossy()))?;
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
    use crate::test_support::{TempDir, with_current_dir};
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

            let mut output = crate::output::renderer(crate::output::OutputMode::Plain);
            let mut step = 1;
            clean_paths(
                output.as_mut(),
                &mut step,
                2,
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
            let mut output = crate::output::renderer(crate::output::OutputMode::Plain);
            let mut step = 1;
            let error = clean_paths(
                output.as_mut(),
                &mut step,
                1,
                vec![String::from(".")],
                "test paths",
            )
            .unwrap_err();

            assert!(error.contains("Refusing to clean dangerous path"));
            assert!(temp.path().exists());
        });
    }
}
