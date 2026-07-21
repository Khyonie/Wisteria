#![allow(dead_code)]

use std::{collections::HashMap, rc::Rc};

use toml::Table;

use crate::{
    build::task::{DefinedTask, ImplicitBuildTask, TaskRunner},
    config::toml_utils,
    java::compiler_flags::CompilerFlags,
};

#[derive(Clone)]
pub struct Configuration {
    name: String,
    sources: Option<Vec<String>>,
    dependencies: Option<Vec<String>>,
    shaded: Option<Vec<String>>,
    includes: Option<Vec<String>>,
    targets: Option<Vec<String>>,

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
    ) -> Result<Self, (String, u8)> {
        let sources = toml_utils::read_string_array("sources", toml).ok();
        let dependencies = toml_utils::read_string_array("dependencies", toml).ok();
        let shaded = toml_utils::read_string_array("shaded", toml).ok();
        let includes = toml_utils::read_string_array("includes", toml).ok();
        let targets = toml_utils::read_string_array("targets", toml).ok();

        let entry = toml_utils::read_string("entry", toml).ok();
        let java_version = toml_utils::read_integer("java_version", toml).unwrap_or(8);
        let inherit: Option<String> = toml_utils::read_string("inherit", toml).ok();

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
                            return Err((
                                format!(
                                "Mismatched type for task \"{key}\", expected a table, found {}",
                                t.type_str()
                            ),
                                16,
                            ))
                        }
                        None => panic!(),
                    };
                }
            }
            Some(v) => {
                return Err((
                    format!(
                        "Mismatched type for \"task\", expected a table, found {}",
                        v.type_str()
                    ),
                    16,
                ))
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
                    match value.as_str()
                    {
                        Some(s) => environment.insert(key.clone(), s.to_string()),
                        None => return Err((format!("Mismatched type for environment variable \"{key}\", expected a string, found {}", value.type_str()), 15))
                    };
                }
            }
            Some(v) => {
                return Err((
                    format!(
                        "Mismatched type for \"environment\", expected a table, found {}",
                        v.type_str()
                    ),
                    15,
                ))
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
                return Err((
                    format!(
                        "Mismatched type for \"{name}.compiler_flags\", expected a table, found {}",
                        v.type_str()
                    ),
                    16,
                ))
            }
            None => None,
        };

        Ok(Configuration {
            name,
            sources,
            dependencies,
            shaded,
            includes,
            targets,
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

    pub fn dependencies(&self) -> Option<&Vec<String>> {
        self.dependencies.as_ref()
    }

    pub fn shaded(&self) -> Option<&Vec<String>> {
        self.shaded.as_ref()
    }

    pub fn includes(&self) -> Option<&Vec<String>> {
        self.includes.as_ref()
    }

    pub fn targets(&self) -> Option<&Vec<String>> {
        self.targets.as_ref()
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

    pub fn apply_implicit(&mut self) {
        if self.targets.is_some() && self.sources.is_some() {
            self.tasks
                .insert(String::from("build"), Rc::new(ImplicitBuildTask::new()));
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
            println!(
                "│\tDependencies     {}",
                toml_utils::string_vec_to_string(d)
            )
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
    use crate::java::compiler_flags::CompilerFlags;

    fn table(toml: &str) -> Table {
        toml.parse::<Table>().unwrap()
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
            configuration.environment().get("project_name").map(String::as_str),
            Some("Demo")
        );
        assert_eq!(
            configuration.environment().get("configuration").map(String::as_str),
            Some("main")
        );
        assert_eq!(
            configuration.environment().get("version").map(String::as_str),
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
                dependencies = [ "dep-a", "dep-b" ]
                shaded = [ "dep-b" ]
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

        assert_eq!(configuration.sources().unwrap(), &vec![String::from("src/")]);
        assert_eq!(
            configuration.dependencies().unwrap(),
            &vec![String::from("dep-a"), String::from("dep-b")]
        );
        assert_eq!(configuration.shaded().unwrap(), &vec![String::from("dep-b")]);
        assert_eq!(
            configuration.includes().unwrap(),
            &vec![String::from("plugin.yml")]
        );
        assert_eq!(
            configuration.targets().unwrap(),
            &vec![String::from("target/demo.jar")]
        );
        assert_eq!(configuration.entry().map(String::as_str), Some("com.example.Main"));
        assert_eq!(configuration.java_version(), 21);
        assert_eq!(
            configuration.environment().get("channel").map(String::as_str),
            Some("stable")
        );
        assert!(configuration
            .compiler_flags()
            .unwrap()
            .contains(&CompilerFlags::StoreParameterNames { setting: true }));
        assert!(configuration
            .compiler_flags()
            .unwrap()
            .contains(&CompilerFlags::Encoding {
                encoding: String::from("UTF-8")
            }));
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

        configuration.apply_implicit();

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
        assert_eq!(
            child.dependencies().unwrap(),
            &vec![String::from("dep-b"), String::from("dep-a")]
        );
        assert_eq!(
            child.includes().unwrap(),
            &vec![String::from("plugin.yml")]
        );
        assert_eq!(
            child.targets().unwrap(),
            &vec![String::from("target/base.jar")]
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

        assert!(error.0.contains("Mismatched type for environment variable"));
        assert_eq!(error.1, 15);
    }
}
