use std::{collections::HashMap, fs, process::Command};

use regex::Regex;

use crate::{
    build::{
        resolve::{ResolvedDependencies, resolve_dependencies},
        sources,
    },
    java::compiler_flags::CompilerFlags,
    model::{Configuration, Project, ProjectInfo},
    project::TaskRunner,
    util::{consts, exit_code},
    workspace::paths::resolve_filepath,
};

pub struct ImplicitJavadocTask {
    order: Vec<String>,
}

impl ImplicitJavadocTask {
    pub fn new() -> Self {
        ImplicitJavadocTask {
            order: vec![
                String::from("collect"),
                String::from("resolve"),
                String::from("javadoc"),
            ],
        }
    }
}

impl Default for ImplicitJavadocTask {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskRunner for ImplicitJavadocTask {
    fn invoke(
        &self,
        _info: &ProjectInfo,
        project: &Project,
        configuration: &Configuration,
    ) -> Result<(), String> {
        let mut regexes: HashMap<&str, Regex> = HashMap::new();
        regexes.insert("envvars", Regex::new(r#"\{(.+?)}"#).unwrap());

        let copied_files = sources::collect_sources(configuration)?;
        let dependencies = resolve_dependencies(project, configuration, &regexes)?;

        run_javadoc(
            project,
            configuration,
            &dependencies,
            copied_files,
            &regexes,
        )
    }

    fn phase_order(&self) -> &[String] {
        &self.order
    }
}

fn run_javadoc(
    project: &Project,
    configuration: &Configuration,
    dependencies: &ResolvedDependencies,
    copied_files: Vec<String>,
    regexes: &HashMap<&str, Regex>,
) -> Result<(), String> {
    let classpath = dependencies.classpath();
    let javadoc_links = dependency_javadoc_links(project, configuration);
    let output_dir = prepare_javadoc_output_dir(configuration, regexes)?;
    let args = build_javadoc_command_args(
        configuration,
        &copied_files,
        classpath.as_deref(),
        &javadoc_links,
        &output_dir,
    )?;

    let mut javadoc_command = Command::new("javadoc");
    javadoc_command.args(&args);

    println!(
        "Generating javadocs for {} source files",
        copied_files.len()
    );
    match javadoc_command.output() {
        Ok(out) => {
            if !out.stdout.is_empty() {
                println!("{}", String::from_utf8_lossy(&out.stdout));
            }

            if !out.stderr.is_empty() {
                println!("{}", String::from_utf8_lossy(&out.stderr));
            }

            if !out.status.success() {
                exit_code::record_external_process_exit_code(out.status);
                return Err(format!("javadoc failed with status {}", out.status));
            }
        }
        Err(e) => return Err(format!("Failed to run javadoc command: {e}")),
    }

    if let Some(target) = configuration.javadoc_target() {
        package_javadocs(configuration, &output_dir, target, regexes)?;
    }

    Ok(())
}

fn build_javadoc_command_args(
    configuration: &Configuration,
    copied_files: &[String],
    classpath: Option<&str>,
    javadoc_links: &[String],
    output_dir: &str,
) -> Result<Vec<String>, String> {
    if copied_files.is_empty() {
        return Err(String::from("No source files found, nothing to document"));
    }

    let mut args = vec![
        String::from("-d"),
        output_dir.to_string(),
        String::from("--source-path"),
        String::from(consts::SOURCE_OUT_PATH),
    ];

    if let Some(classpath) = classpath.filter(|classpath| !classpath.is_empty()) {
        args.push(String::from("--class-path"));
        args.push(classpath.to_string());
    }

    for link in javadoc_links {
        args.push(String::from("-link"));
        args.push(link.clone());
    }

    if let Some(flags) = configuration.compiler_flags() {
        for flag in flags {
            args.extend(javadoc_flag(flag));
        }
    }

    args.extend(copied_files.iter().cloned());

    Ok(args)
}

fn prepare_javadoc_output_dir(
    configuration: &Configuration,
    regexes: &HashMap<&str, Regex>,
) -> Result<String, String> {
    let output_dir = resolve_filepath(
        configuration.javadoc_output_dir(),
        configuration.environment(),
        regexes,
    )?;
    fs::create_dir_all(&output_dir)
        .map_err(|e| format!("Could not create javadoc output directory {output_dir}: {e}"))?;

    Ok(output_dir)
}

fn package_javadocs(
    configuration: &Configuration,
    output_dir: &str,
    target: &str,
    regexes: &HashMap<&str, Regex>,
) -> Result<(), String> {
    let target = resolve_filepath(target, configuration.environment(), regexes)?;
    let target_path = std::path::PathBuf::from(&target);
    if let Some(parent) = target_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Could not create parent folder {}: {e}",
                parent.to_string_lossy()
            )
        })?;
    }

    let mut jar_command = Command::new("jar");
    jar_command.args(["-cf", &target, "-C", output_dir, "."]);

    match jar_command.output() {
        Ok(out) => {
            if !out.stdout.is_empty() {
                println!("{}", String::from_utf8_lossy(&out.stdout));
            }

            if !out.stderr.is_empty() {
                println!("{}", String::from_utf8_lossy(&out.stderr));
            }

            if !out.status.success() {
                exit_code::record_external_process_exit_code(out.status);
                return Err(format!(
                    "javadoc jar packaging failed with status {}",
                    out.status
                ));
            }
        }
        Err(e) => return Err(format!("Failed to package javadocs: {e}")),
    }

    println!("Successfully written javadoc target {target}");
    Ok(())
}

fn dependency_javadoc_links(project: &Project, configuration: &Configuration) -> Vec<String> {
    let Some(dependencies) = configuration.dependencies() else {
        return Vec::new();
    };

    let mut links = Vec::new();
    for reference in dependencies {
        let Some(javadoc) = project
            .dependencies()
            .get(reference.name())
            .and_then(|dependency| dependency.javadoc())
        else {
            continue;
        };

        if !links.contains(javadoc) {
            links.push(javadoc.clone());
        }
    }

    links
}

fn javadoc_flag(flag: &CompilerFlags) -> Vec<String> {
    match flag {
        CompilerFlags::ReleaseTarget { version } => {
            vec![String::from("--release"), version.to_string()]
        }
        CompilerFlags::EnablePreviewFeatures { setting } => {
            if *setting {
                vec![String::from("--enable-preview")]
            } else {
                Vec::new()
            }
        }
        CompilerFlags::JavadocAllLints { setting } => {
            if *setting {
                vec![String::from("-Xdoclint:all")]
            } else {
                Vec::new()
            }
        }
        CompilerFlags::JavadocLints { lints } => {
            let mut flag = String::from("-Xdoclint:");
            flag.push_str(&lints.join(","));
            vec![flag]
        }
        CompilerFlags::Encoding { encoding } => vec![String::from("-encoding"), encoding.clone()],
        CompilerFlags::SourceLintAll { .. }
        | CompilerFlags::SourceLints { .. }
        | CompilerFlags::NoWarnings { .. }
        | CompilerFlags::DeprecationInfo { .. }
        | CompilerFlags::StoreParameterNames { .. } => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TempDir, with_current_dir};
    use std::fs;
    use toml::Table;

    fn configuration(toml: &str) -> Configuration {
        Configuration::from(
            String::from("docs"),
            &toml.parse::<Table>().unwrap(),
            String::from("Demo"),
            String::from("1.0.0"),
        )
        .unwrap()
    }

    fn regexes() -> HashMap<&'static str, Regex> {
        let mut regexes = HashMap::new();
        regexes.insert("envvars", Regex::new(r#"\{(.+?)}"#).unwrap());
        regexes
    }

    #[test]
    fn javadoc_command_args_include_output_sources_classpath_and_supported_flags() {
        let temp = TempDir::new("javadoc-command-args");

        with_current_dir(temp.path(), || {
            let configuration = configuration(
                r#"
                sources = [ "src/" ]

                [compiler_flags]
                release_target = 17
                enable_preview_features = true
                javadoc_lints = [ "missing", "reference" ]
                source_encoding = "UTF-8"
                store_parameter_names = true
                source_all_lints = true
                "#,
            );

            let args = build_javadoc_command_args(
                &configuration,
                &[String::from(".wisteria/work/src/example/Main.java")],
                Some("lib/example.jar"),
                &[String::from("https://example.com/docs/")],
                "target/javadoc/docs/",
            )
            .unwrap();

            assert!(
                args.windows(2)
                    .any(|window| window == ["-d", "target/javadoc/docs/"])
            );
            assert!(
                args.windows(2)
                    .any(|window| window == ["--source-path", consts::SOURCE_OUT_PATH])
            );
            assert!(
                args.windows(2)
                    .any(|window| window == ["--class-path", "lib/example.jar"])
            );
            assert!(
                args.windows(2)
                    .any(|window| window == ["-link", "https://example.com/docs/"])
            );
            assert!(args.windows(2).any(|window| window == ["--release", "17"]));
            assert!(args.contains(&String::from("--enable-preview")));
            assert!(args.contains(&String::from("-Xdoclint:missing,reference")));
            assert!(
                args.windows(2)
                    .any(|window| window == ["-encoding", "UTF-8"])
            );
            assert!(args.contains(&String::from(".wisteria/work/src/example/Main.java")));
            assert!(!args.contains(&String::from("-parameters")));
            assert!(!args.contains(&String::from("-Xlint:all")));
        });
    }

    #[test]
    fn javadoc_command_args_reject_empty_source_file_list() {
        let temp = TempDir::new("javadoc-empty-source-list");

        with_current_dir(temp.path(), || {
            let configuration = configuration(r#"sources = [ "src/" ]"#);

            let error =
                build_javadoc_command_args(&configuration, &[], None, &[], "target/javadoc/docs/")
                    .unwrap_err();

            assert_eq!(error, "No source files found, nothing to document");
        });
    }

    #[test]
    fn prepare_javadoc_output_dir_uses_configuration_path() {
        let temp = TempDir::new("javadoc-output-dir");

        with_current_dir(temp.path(), || {
            let configuration = configuration(
                r#"
                sources = [ "src/" ]

                [javadoc]
                output-dir = "target/docs/{configuration}/"
                "#,
            );

            let output_dir = prepare_javadoc_output_dir(&configuration, &regexes()).unwrap();

            assert_eq!(output_dir, "target/docs/docs/");
            assert!(temp.path().join("target/docs/docs").exists());
        });
    }

    #[test]
    fn dependency_javadoc_links_reads_configured_dependency_docs() {
        let temp = TempDir::new("javadoc-dependency-links");
        let project_file = temp.path().join("project.toml");
        fs::write(
            &project_file,
            r#"
            [project]
            name = "Demo"
            version = "1.0.0"
            description = "Demo"

            [dependencies.archive]
            library = { path = "lib/library.jar", javadoc = "https://example.com/docs/" }

            [configuration.docs]
            sources = [ "src/" ]
            dependencies = [ "library" ]
            "#,
        )
        .unwrap();

        let project = Project::from(Some(project_file.to_string_lossy().to_string())).unwrap();
        let configuration = project.info().configurations().get("docs").unwrap();

        assert_eq!(
            dependency_javadoc_links(&project, configuration),
            vec![String::from("https://example.com/docs/")]
        );
    }
}
