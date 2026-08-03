#![allow(dead_code)]

use std::{collections::HashMap, rc::Rc};

use toml::{Table, Value};

use crate::{
    build::{
        javadoc::ImplicitJavadocTask,
        run::ImplicitRunTask,
        task::{DefinedTask, ImplicitBuildTask, TaskRunner},
    },
    cli::args::StartupFlags,
    config::toml_utils::{self, read_optional_string, read_string},
    dependency::{DependencyReference, DependencyScope, PackagingType},
    java::compiler_flags::CompilerFlags,
    util::consts,
};

#[derive(Clone, PartialEq, Eq)]
pub struct JavadocConfiguration {
    output_dir: Option<String>,
    target: Option<String>,
}

impl JavadocConfiguration {
    fn from(configuration_name: &str, toml: &Table) -> Result<Self, String> {
        Ok(Self {
            output_dir: read_optional_javadoc_string(configuration_name, toml, "output-dir")?,
            target: read_optional_javadoc_string(configuration_name, toml, "target")?,
        })
    }

    pub fn output_dir(&self) -> Option<&String> {
        self.output_dir.as_ref()
    }

    pub fn target(&self) -> Option<&String> {
        self.target.as_ref()
    }

    fn inherit_from(&mut self, configuration: &JavadocConfiguration) {
        if self.output_dir.is_none() {
            self.output_dir = configuration.output_dir.clone();
        }
        if self.target.is_none() {
            self.target = configuration.target.clone();
        }
    }
}

#[derive(Clone)]
pub struct Configuration {
    name: String,
    sources: Option<Vec<String>>,
    dependencies: Option<Vec<DependencyReference>>,
    includes: Option<Vec<String>>,
    targets: Option<Vec<String>>,
    javadoc: Option<JavadocConfiguration>,

    entry: Option<String>,
    java_version: u8,

    tasks: HashMap<String, Rc<dyn TaskRunner>>,
    compiler_flags: Option<Vec<CompilerFlags>>,
    environment: HashMap<String, String>,
    inherit: Option<String>,
}

impl Configuration {
    pub fn from(
        name: String,
        toml: &Table,
        project_name: String,
        version: String,
    ) -> Result<Self, String> {
        let sources = read_optional_string_array_for_configuration(&name, "sources", toml)?;
        let dependencies =
            read_optional_dependency_array_for_configuration(&name, "dependencies", toml)?;
        if toml.contains_key("shaded") {
            return Err(format!(
                "Invalid [configuration.{name}].shaded: `shaded` was removed.\nFix: move each shaded dependency into `dependencies` as `{{ name = \"dependency-name\", package = \"shade\" }}`."
            ));
        }
        let includes = read_optional_string_array_for_configuration(&name, "includes", toml)?;
        let targets = read_optional_string_array_for_configuration(&name, "targets", toml)?;
        let javadoc = match toml.get("javadoc") {
            Some(v) if v.is_table() => {
                Some(JavadocConfiguration::from(&name, v.as_table().unwrap())?)
            }
            Some(v) => {
                return Err(format!(
                    "Invalid [configuration.{name}].javadoc: expected a table, found {}.\nFix: define javadoc settings under `[configuration.{name}.javadoc]`, or remove `javadoc`.",
                    v.type_str()
                ));
            }
            None => None,
        };

        let entry = read_optional_string_for_configuration(&name, "entry", toml)?;
        let java_version =
            read_optional_integer_for_configuration(&name, "java_version", toml)?.unwrap_or(8);
        let inherit: Option<String> =
            read_optional_string_for_configuration(&name, "inherit", toml)?;

        let mut tasks: HashMap<String, Rc<dyn TaskRunner>> = HashMap::new();

        match toml.get("task") {
            Some(v) if v.is_table() => {
                let v = v.as_table().unwrap();

                for key in v.keys() {
                    match v.get(key) {
                        Some(t) if t.is_table() => tasks.insert(
                            key.clone(),
                            Rc::new(DefinedTask::new(key, t.as_table().unwrap())?),
                        ),
                        Some(t) => {
                            return Err(format!(
                                "Invalid task [configuration.{name}.task.{key}]: expected a table, found {}.\nFix: define custom task phases under `[configuration.{name}.task.{key}]`, or remove this task entry.",
                                t.type_str()
                            ));
                        }
                        None => panic!(),
                    };
                }
            }
            Some(v) => {
                return Err(format!(
                    "Invalid [configuration.{name}].task: expected a table, found {}.\nFix: custom tasks must be defined under `[configuration.{name}.task.<task-name>]` tables, or remove `task`.",
                    v.type_str()
                ));
            }
            None => {}
        }

        let mut environment: HashMap<String, String> = HashMap::new();
        environment.insert(String::from("project_name"), project_name);
        environment.insert(String::from("configuration"), name.clone());
        environment.insert(String::from("version"), version);

        match toml.get("environment") {
            Some(t) if t.is_table() => {
                let t = t.as_table().unwrap();

                for (key, value) in t {
                    match value.as_str() {
                        Some(s) => environment.insert(key.clone(), s.to_string()),
                        None => {
                            return Err(format!(
                                "Invalid [configuration.{name}.environment].{key}: expected a string, found {}.\nFix: environment values must be quoted strings, for example `{key} = \"value\"`.",
                                value.type_str()
                            ));
                        }
                    };
                }
            }
            Some(v) => {
                return Err(format!(
                    "Invalid [configuration.{name}].environment: expected a table, found {}.\nFix: define environment variables under `[configuration.{name}.environment]`, for example `channel = \"stable\"`, or remove `environment`.",
                    v.type_str()
                ));
            }
            None => {}
        }

        let compiler_flags: Option<Vec<CompilerFlags>> = match toml.get("compiler_flags") {
            Some(t) if t.is_table() => {
                let t = t.as_table().unwrap();
                let mut flags: Vec<CompilerFlags> = Vec::new();

                for (key, value) in t {
                    flags.push(CompilerFlags::from(key, value)?);
                }

                Some(flags)
            }
            Some(v) => {
                return Err(format!(
                    "Invalid [configuration.{name}].compiler_flags: expected a table, found {}.\nFix: define compiler flags under `[configuration.{name}.compiler_flags]`, for example `release_target = 17`, or remove `compiler_flags`.",
                    v.type_str()
                ));
            }
            None => None,
        };

        Ok(Configuration {
            name,
            sources,
            dependencies,
            includes,
            targets,
            javadoc,
            entry,
            java_version,
            tasks,
            compiler_flags,
            environment,
            inherit,
        })
    }

    pub fn sources(&self) -> Option<&Vec<String>> {
        self.sources.as_ref()
    }

    pub fn dependencies(&self) -> Option<&Vec<DependencyReference>> {
        self.dependencies.as_ref()
    }

    pub fn includes(&self) -> Option<&Vec<String>> {
        self.includes.as_ref()
    }

    pub fn targets(&self) -> Option<&Vec<String>> {
        self.targets.as_ref()
    }

    pub fn javadoc(&self) -> Option<&JavadocConfiguration> {
        self.javadoc.as_ref()
    }

    pub fn javadoc_output_dir(&self) -> &str {
        self.javadoc
            .as_ref()
            .and_then(JavadocConfiguration::output_dir)
            .map(String::as_str)
            .unwrap_or(consts::DEFAULT_JAVADOC_DIR)
    }

    pub fn javadoc_target(&self) -> Option<&String> {
        self.javadoc.as_ref().and_then(JavadocConfiguration::target)
    }

    pub fn entry(&self) -> Option<&String> {
        self.entry.as_ref()
    }

    pub fn java_version(&self) -> u8 {
        self.java_version
    }

    pub fn tasks(&self) -> &HashMap<String, Rc<dyn TaskRunner>> {
        &self.tasks
    }

    pub fn inherits(&self) -> Option<&String> {
        self.inherit.as_ref()
    }

    pub fn environment(&self) -> &HashMap<String, String> {
        &self.environment
    }

    pub fn compiler_flags(&self) -> Option<&Vec<CompilerFlags>> {
        self.compiler_flags.as_ref()
    }

    pub fn apply_implicit(&mut self, flags: StartupFlags) {
        if self.sources.is_some() {
            self.tasks.insert(
                String::from("javadocs"),
                Rc::new(ImplicitJavadocTask::new()),
            );

            if self.targets().is_some() {
                self.tasks
                    .insert(String::from("build"), Rc::new(ImplicitBuildTask::new()));
            }

            if self.entry.is_some() {
                self.tasks
                    .insert(String::from("run"), Rc::new(ImplicitRunTask::new(flags)));
            }
        }
    }

    pub fn inherit_from(&mut self, configuration: &Configuration) {
        self.sources = inherit_vec(self.sources.as_mut(), configuration.sources.as_ref());
        self.dependencies = inherit_vec(
            self.dependencies.as_mut(),
            configuration.dependencies.as_ref(),
        );
        self.includes = inherit_vec(self.includes.as_mut(), configuration.includes.as_ref());
        self.targets = inherit_vec(self.targets.as_mut(), configuration.targets.as_ref());
        match (self.javadoc.as_mut(), configuration.javadoc.as_ref()) {
            (Some(javadoc), Some(parent_javadoc)) => javadoc.inherit_from(parent_javadoc),
            (None, Some(parent_javadoc)) => self.javadoc = Some(parent_javadoc.clone()),
            _ => {}
        }
        if self.entry.is_none() && configuration.entry.is_some() {
            self.entry = configuration.entry.clone();
        }
        self.java_version = configuration.java_version;
        for (k, task) in configuration.tasks() {
            if !self.tasks.contains_key(k) {
                self.tasks.insert(k.clone(), task.clone());
            }
        }
        self.compiler_flags = inherit_vec(
            self.compiler_flags.as_mut(),
            configuration.compiler_flags.as_ref(),
        );
        for (k, v) in &configuration.environment {
            if !self.environment.contains_key(k) {
                self.environment.insert(k.clone(), v.clone());
            }
        }
    }

    pub fn print_info(&self) {
        println!("╞ Configuration \"{}\":", self.name);

        if let Some(s) = &self.sources {
            println!(
                "│\tSources          {}",
                toml_utils::string_vec_to_string(s)
            )
        }

        if let Some(d) = &self.dependencies {
            println!("│\tDependencies     {}", dependency_references_to_string(d))
        }

        if let Some(i) = &self.includes {
            println!(
                "│\tIncludes         {}",
                toml_utils::string_vec_to_string(i)
            )
        }

        if let Some(e) = &self.entry {
            println!("│\tMain class       {e}")
        }

        if let Some(javadoc) = &self.javadoc {
            if let Some(output_dir) = javadoc.output_dir() {
                println!("│\tJavadocs         {output_dir}")
            }

            if let Some(target) = javadoc.target() {
                println!("│\tJavadoc jar      {target}")
            }
        }

        println!("│\tJava version     {}", self.java_version);

        let mut environment: String = String::new();
        for (k, v) in &self.environment {
            environment.push_str(k);
            environment.push_str(format!(": \"{v}\", ").as_str());
        }

        environment.pop();
        environment.pop();

        println!("│\tEnvironment      [ {environment} ]");

        if let Some(flags) = &self.compiler_flags {
            let mut string: String = String::new();

            for f in flags {
                let mut flag = String::new();

                for component in f.get_canon_flag() {
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
        for (key, task) in &self.tasks {
            let mut phases: String = String::new();

            for phase in task.phase_order() {
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

fn read_optional_string_for_configuration(
    configuration_name: &str,
    key: &str,
    toml: &Table,
) -> Result<Option<String>, String> {
    toml_utils::read_optional_string(key, toml)
        .map_err(|error| contextual_configuration_error(configuration_name, key, error))
}

fn read_optional_integer_for_configuration(
    configuration_name: &str,
    key: &str,
    toml: &Table,
) -> Result<Option<u8>, String> {
    toml_utils::read_optional_integer(key, toml)
        .map_err(|error| contextual_configuration_error(configuration_name, key, error))
}

fn read_optional_string_array_for_configuration(
    configuration_name: &str,
    key: &str,
    toml: &Table,
) -> Result<Option<Vec<String>>, String> {
    toml_utils::read_optional_string_array(key, toml)
        .map_err(|error| contextual_configuration_error(configuration_name, key, error))
}

fn read_optional_dependency_array_for_configuration(
    configuration_name: &str,
    key: &str,
    toml: &Table,
) -> Result<Option<Vec<DependencyReference>>, String> {
    let mut references = Vec::new();

    let Some(value) = toml.get(key) else {
        return Ok(None);
    };

    let Some(array) = value.as_array() else {
        return Err(format!(
            "Invalid [configuration.{configuration_name}].{key}: expected an array of dependency names or reference tables, found {}.\nFix: use `dependencies = [ \"dependency-name\" ]` or `dependencies = [ {{ name = \"dependency-name\", scope = \"compile\" }} ]`.",
            value.type_str()
        ));
    };

    for (index, value) in array.iter().enumerate() {
        if let Some(name) = value.as_str() {
            references.push(DependencyReference::new(
                String::from(name),
                DependencyScope::Compile,
                None,
            ));
            continue;
        }

        if let Some(table) = value.as_table() {
            references.push(read_table_as_dependency_reference(
                configuration_name,
                index,
                table,
            )?);
            continue;
        }

        return Err(format!(
            "Invalid [configuration.{configuration_name}].{key}[{index}]: expected a dependency name string or an inline table, found {}.\nFix: use `\"dependency-name\"` or `{{ name = \"dependency-name\", scope = \"compile\" }}`.",
            value.type_str()
        ));
    }

    Ok(Some(references))
}

fn read_table_as_dependency_reference(
    configuration_name: &str,
    index: usize,
    toml: &Table,
) -> Result<DependencyReference, String> {
    let name = read_string("name", toml).map_err(|error| {
        contextual_dependency_reference_error(configuration_name, index, "name", error)
    })?;

    let scope: DependencyScope = read_optional_string("scope", toml)
        .map_err(|error| {
            contextual_dependency_reference_error(configuration_name, index, "scope", error)
        })?
        .unwrap_or(String::from("compile"))
        .try_into()
        .map_err(|error| {
            contextual_dependency_reference_error(configuration_name, index, "scope", error)
        })?;

    let packaging: Option<PackagingType> = read_optional_string("package", toml)
        .map_err(|error| {
            contextual_dependency_reference_error(configuration_name, index, "package", error)
        })?
        .map(|p| p.try_into())
        .transpose()
        .map_err(|error| {
            contextual_dependency_reference_error(configuration_name, index, "package", error)
        })?;

    Ok(DependencyReference::new(name, scope, packaging))
}

fn contextual_configuration_error(configuration_name: &str, key: &str, error: String) -> String {
    format!(
        "Invalid [configuration.{configuration_name}].{key}: {}",
        error
    )
}

fn contextual_dependency_reference_error(
    configuration_name: &str,
    index: usize,
    key: &str,
    error: String,
) -> String {
    format!("Invalid [configuration.{configuration_name}].dependencies[{index}].{key}: {error}")
}

fn dependency_references_to_string(references: &[DependencyReference]) -> String {
    references
        .iter()
        .map(|reference| reference.name())
        .collect::<Vec<_>>()
        .join(", ")
}

fn read_optional_javadoc_string(
    configuration_name: &str,
    toml: &Table,
    key: &str,
) -> Result<Option<String>, String> {
    match toml.get(key) {
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(value) => Err(format!(
            "Invalid [configuration.{configuration_name}.javadoc].{key}: expected a string, found {}.\nFix: write javadoc `{key}` as a quoted path, or remove the key.",
            value.type_str()
        )),
        None => Ok(None),
    }
}

fn inherit_vec<T: Clone + Eq>(
    inheritor: Option<&mut Vec<T>>,
    host: Option<&Vec<T>>,
) -> Option<Vec<T>> {
    match inheritor {
        Some(data) => {
            if let Some(host_data) = host {
                for s in host_data {
                    if !data.contains(s) {
                        data.push(s.clone());
                    }
                }
            }

            Some(data.clone())
        }
        None if host.is_some() => Some(host.unwrap().clone()),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::StartupFlags;
    use crate::java::compiler_flags::CompilerFlags;

    fn table(toml: &str) -> Table {
        toml.parse::<Table>().unwrap()
    }

    fn dependency_names(configuration: &Configuration) -> Vec<&str> {
        configuration
            .dependencies()
            .unwrap()
            .iter()
            .map(DependencyReference::name)
            .collect()
    }

    #[test]
    fn configuration_defaults_project_environment_and_java_version() {
        let configuration = Configuration::from(
            String::from("main"),
            &table(""),
            String::from("Demo"),
            String::from("1.0.0"),
        )
        .unwrap();

        assert_eq!(configuration.java_version(), 8);
        assert_eq!(
            configuration
                .environment()
                .get("project_name")
                .map(String::as_str),
            Some("Demo")
        );
        assert_eq!(
            configuration
                .environment()
                .get("configuration")
                .map(String::as_str),
            Some("main")
        );
        assert_eq!(
            configuration
                .environment()
                .get("version")
                .map(String::as_str),
            Some("1.0.0")
        );
    }

    #[test]
    fn configuration_loads_fields_and_compiler_flags() {
        let configuration = Configuration::from(
            String::from("main"),
            &table(
                r#"
                sources = "src/"
                dependencies = [
                    "dep-a",
                    { name = "dep-b", scope = "runtime", package = "shade" },
                ]
                includes = [ "plugin.yml" ]
                targets = [ "target/demo.jar" ]
                entry = "com.example.Main"
                java_version = 21

                [environment]
                channel = "stable"

                [compiler_flags]
                store_parameter_names = true
                source_encoding = "UTF-8"
                "#,
            ),
            String::from("Demo"),
            String::from("1.0.0"),
        )
        .unwrap();

        assert_eq!(
            configuration.sources().unwrap(),
            &vec![String::from("src/")]
        );
        assert_eq!(dependency_names(&configuration), vec!["dep-a", "dep-b"]);
        assert_eq!(
            configuration.dependencies().unwrap()[0].scope(),
            DependencyScope::Compile
        );
        assert_eq!(
            configuration.dependencies().unwrap()[1].scope(),
            DependencyScope::Runtime
        );
        assert!(configuration.dependencies().unwrap()[1].is_shaded());
        assert_eq!(
            configuration.includes().unwrap(),
            &vec![String::from("plugin.yml")]
        );
        assert_eq!(
            configuration.targets().unwrap(),
            &vec![String::from("target/demo.jar")]
        );
        assert_eq!(
            configuration.entry().map(String::as_str),
            Some("com.example.Main")
        );
        assert_eq!(configuration.java_version(), 21);
        assert_eq!(
            configuration
                .environment()
                .get("channel")
                .map(String::as_str),
            Some("stable")
        );
        assert!(
            configuration
                .compiler_flags()
                .unwrap()
                .contains(&CompilerFlags::StoreParameterNames { setting: true })
        );
        assert!(
            configuration
                .compiler_flags()
                .unwrap()
                .contains(&CompilerFlags::Encoding {
                    encoding: String::from("UTF-8")
                })
        );
    }

    #[test]
    fn configuration_loads_javadoc_output_and_target() {
        let configuration = Configuration::from(
            String::from("main"),
            &table(
                r#"
                sources = [ "src/" ]

                [javadoc]
                output-dir = "targets/javadoc/"
                target = "targets/{configuration}/{version}/{project_name}-javadoc.jar"
                "#,
            ),
            String::from("Demo"),
            String::from("1.0.0"),
        )
        .unwrap();

        assert_eq!(configuration.javadoc_output_dir(), "targets/javadoc/");
        assert_eq!(
            configuration.javadoc_target().map(String::as_str),
            Some("targets/{configuration}/{version}/{project_name}-javadoc.jar")
        );
    }

    #[test]
    fn configuration_uses_default_javadoc_output_without_javadoc_table() {
        let configuration = Configuration::from(
            String::from("main"),
            &table(r#"sources = [ "src/" ]"#),
            String::from("Demo"),
            String::from("1.0.0"),
        )
        .unwrap();

        assert_eq!(
            configuration.javadoc_output_dir(),
            consts::DEFAULT_JAVADOC_DIR
        );
        assert_eq!(configuration.javadoc_target(), None);
    }

    #[test]
    fn apply_implicit_adds_build_task_when_sources_and_targets_exist() {
        let mut configuration = Configuration::from(
            String::from("main"),
            &table(
                r#"
                sources = [ "src/" ]
                targets = [ "target/demo.jar" ]
                "#,
            ),
            String::from("Demo"),
            String::from("1.0.0"),
        )
        .unwrap();

        configuration.apply_implicit(StartupFlags::default());

        let build = configuration.tasks().get("build").unwrap();
        assert_eq!(
            build.phase_order(),
            &[
                String::from("collect"),
                String::from("compile"),
                String::from("shade"),
                String::from("package"),
            ]
        );
    }

    #[test]
    fn inherit_from_appends_unique_values_and_inherits_missing_fields() {
        let parent = Configuration::from(
            String::from("base"),
            &table(
                r#"
                sources = [ "src/main/" ]
                dependencies = [ "dep-a" ]
                includes = [ "plugin.yml" ]
                targets = [ "target/base.jar" ]
                entry = "com.example.Main"
                java_version = 17

                [javadoc]
                output-dir = "target/docs/base/"
                target = "target/base-javadocs.jar"

                [environment]
                inherited = "yes"
                "#,
            ),
            String::from("Demo"),
            String::from("1.0.0"),
        )
        .unwrap();
        let mut child = Configuration::from(
            String::from("child"),
            &table(
                r#"
                sources = [ "src/main/", "src/child/" ]
                dependencies = [ "dep-b" ]
                "#,
            ),
            String::from("Demo"),
            String::from("1.0.0"),
        )
        .unwrap();

        child.inherit_from(&parent);

        assert_eq!(
            child.sources().unwrap(),
            &vec![String::from("src/main/"), String::from("src/child/")]
        );
        assert_eq!(dependency_names(&child), vec!["dep-b", "dep-a"]);
        assert_eq!(child.includes().unwrap(), &vec![String::from("plugin.yml")]);
        assert_eq!(
            child.targets().unwrap(),
            &vec![String::from("target/base.jar")]
        );
        assert_eq!(child.javadoc_output_dir(), "target/docs/base/");
        assert_eq!(
            child.javadoc_target().map(String::as_str),
            Some("target/base-javadocs.jar")
        );
        assert_eq!(child.entry().map(String::as_str), Some("com.example.Main"));
        assert_eq!(child.java_version(), 17);
        assert_eq!(
            child.environment().get("inherited").map(String::as_str),
            Some("yes")
        );
    }

    #[test]
    fn rejects_non_string_environment_values() {
        let error = match Configuration::from(
            String::from("main"),
            &table(
                r#"
                [environment]
                port = 25565
                "#,
            ),
            String::from("Demo"),
            String::from("1.0.0"),
        ) {
            Ok(_) => panic!("expected non-string environment value to fail"),
            Err(error) => error,
        };

        assert!(error.contains("Invalid [configuration.main.environment].port"));
        assert!(error.contains("environment values must be quoted strings"));
    }

    #[test]
    fn rejects_malformed_optional_configuration_fields() {
        let error = match Configuration::from(
            String::from("main"),
            &table("sources = 12"),
            String::from("Demo"),
            String::from("1.0.0"),
        ) {
            Ok(_) => panic!("expected malformed sources to fail"),
            Err(error) => error,
        };

        assert!(error.contains("Invalid [configuration.main].sources"));
        assert!(error.contains("sources = [ \"src/\" ]"));
    }

    #[test]
    fn rejects_removed_shaded_configuration() {
        let error = match Configuration::from(
            String::from("main"),
            &table(
                r#"
                dependencies = [ "dep-a" ]
                shaded = [ "dep-a" ]
                "#,
            ),
            String::from("Demo"),
            String::from("1.0.0"),
        ) {
            Ok(_) => panic!("expected removed shaded field to fail"),
            Err(error) => error,
        };

        assert!(error.contains("Invalid [configuration.main].shaded"));
        assert!(error.contains("package = \"shade\""));
    }

    #[test]
    fn rejects_out_of_range_java_version() {
        let error = match Configuration::from(
            String::from("main"),
            &table("java_version = 300"),
            String::from("Demo"),
            String::from("1.0.0"),
        ) {
            Ok(_) => panic!("expected out-of-range java_version to fail"),
            Err(error) => error,
        };

        assert!(error.contains("Invalid [configuration.main].java_version"));
        assert!(error.contains("expected a number from 0 to 255"));
    }
}
