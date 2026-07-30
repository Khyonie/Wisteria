use std::collections::HashMap;

use regex::Regex;
use xml::{EmitterConfig, EventWriter, common::XmlVersion, writer::XmlEvent};

use crate::dependency::{Dependency, UpdateContext};
use crate::model::{Configuration, Project};
use crate::util::consts;
use crate::workspace::nature::Nature;

pub fn generate_classpath(
    project: &Project,
    configuration: &Configuration,
    regexes: &HashMap<&str, Regex>,
) -> Result<String, String> {
    let mut bytes: Vec<u8> = Vec::new();

    let config: EmitterConfig = EmitterConfig::new()
        .perform_indent(true)
        .indent_string(String::from("\t"));

    let mut writer = EventWriter::new_with_config(&mut bytes, config);
    writer
        .write(XmlEvent::StartDocument {
            version: XmlVersion::Version10,
            encoding: Some("UTF-8"),
            standalone: None,
        })
        .map_err(|e| e.to_string())?;
    writer
        .write(XmlEvent::start_element("classpath"))
        .map_err(|e| e.to_string())?;

    if let Some(sources) = configuration.sources() {
        for s in sources {
            let source = XmlEvent::start_element("classpathentry")
                .attr("kind", "src")
                .attr("output", consts::ECLIPSE_TARGET_CLASSES_PATH)
                .attr("path", s);

            writer.write(source).map_err(|e| e.to_string())?;
            writer
                .write(XmlEvent::end_element())
                .map_err(|e| e.to_string())?;
        }
    }

    let use_maven_container = has_maven_nature(project);

    if let Some(dependencies) = configuration.dependencies() {
        let mut width: usize = usize::MIN;
        for name in dependencies.iter() {
            width = usize::max(name.len(), width);
        }

        width += 5;
        let size = dependencies.len();

        println!("Dependencies: [{:?}]", &dependencies);
        for (index, d) in dependencies.iter().enumerate() {
            print!(
                "({}/{size}) Resolving {:width$}",
                index + 1,
                format!("{d} ... ")
            );
            let dependencies_opt = match project.dependencies().get(d) {
                Some(dep) => dep,
                None => {
                    println!("Unknown dependency \"{d}\"!");
                    continue;
                }
            };
            if use_maven_container && matches!(dependencies_opt, Dependency::FetchFromMaven { .. })
            {
                println!("Resolved by Maven");
                continue;
            }

            match dependencies_opt.resolve(
                d,
                configuration.environment(),
                regexes,
                UpdateContext::ResolveOnly,
            ) {
                Ok(paths) => {
                    for path in paths {
                        let path: &str = path.to_str().unwrap();
                        let dep = XmlEvent::start_element("classpathentry")
                            .attr("kind", "lib")
                            .attr("path", path);

                        writer.write(dep).map_err(|e| e.to_string())?;
                        writer
                            .write(XmlEvent::end_element())
                            .map_err(|e| e.to_string())?;
                    }
                }
                Err((error, _)) => return Err(error),
            }
        }
    }

    for nature in project.info().natures() {
        match *nature {
            Nature::Eclipse => {
                let container = XmlEvent::start_element("classpathentry")
                    .attr("kind", "con")
                    .attr("path", "org.eclipse.jdt.launching.JRE_CONTAINER");

                writer.write(container).map_err(|e| e.to_string())?;
                writer
                    .write(XmlEvent::end_element())
                    .map_err(|e| e.to_string())?;
            }
            Nature::Maven => {
                let container = XmlEvent::start_element("classpathentry")
                    .attr("kind", "con")
                    .attr("path", "org.eclipse.m2e.MAVEN2_CLASSPATH_CONTAINER");

                writer.write(container).map_err(|e| e.to_string())?;
                writer
                    .write(XmlEvent::end_element())
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    writer
        .write(
            XmlEvent::start_element("classpathentry")
                .attr("kind", "output")
                .attr("path", consts::ECLIPSE_TARGET_CLASSES_DIR),
        )
        .map_err(|e| e.to_string())?;
    writer
        .write(XmlEvent::end_element())
        .map_err(|e| e.to_string())?;
    writer
        .write(XmlEvent::end_element())
        .map_err(|e| e.to_string())?;

    Ok(String::from_utf8(bytes).unwrap())
}

fn has_maven_nature(project: &Project) -> bool {
    project
        .info()
        .natures()
        .iter()
        .any(|nature| matches!(nature, Nature::Maven))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;
    use std::fs;

    fn regexes() -> HashMap<&'static str, Regex> {
        let mut regexes = HashMap::new();
        regexes.insert("envvars", Regex::new(r#"\{(.+?)}"#).unwrap());
        regexes
    }

    fn project_from_toml(temp: &TempDir, contents: &str) -> Project {
        let project_file = temp.path().join("project.toml");
        fs::write(&project_file, contents).unwrap();

        Project::from(Some(project_file.to_string_lossy().to_string())).unwrap()
    }

    #[test]
    fn generated_classpath_contains_sources_local_libraries_and_containers() {
        let temp = TempDir::new("classpath-local");
        let library = temp.path().join("lib/library.jar");
        fs::create_dir_all(library.parent().unwrap()).unwrap();
        fs::write(&library, "").unwrap();

        let project = project_from_toml(
            &temp,
            &format!(
                r#"
                [project]
                name = "Demo"
                version = "1.0.0"
                description = "Demo"
                natures = [ "eclipse" ]

                [dependencies.archive]
                library = {{ path = "{}" }}

                [configuration.main]
                sources = [ "src/" ]
                dependencies = [ "library" ]
                targets = [ "target/demo.jar" ]
                "#,
                library.to_string_lossy()
            ),
        );
        let configuration = project.info().configurations().get("main").unwrap();

        let xml = generate_classpath(&project, configuration, &regexes()).unwrap();

        assert!(xml.contains(r#"kind="src""#));
        assert!(xml.contains(r#"path="src/""#));
        assert!(xml.contains(&format!(
            r#"path="{}""#,
            library.canonicalize().unwrap().to_string_lossy()
        )));
        assert!(xml.contains("org.eclipse.jdt.launching.JRE_CONTAINER"));
        assert!(xml.contains(r#"path="target/classes/""#));
    }

    #[test]
    fn generated_classpath_uses_maven_container_for_maven_dependencies() {
        let temp = TempDir::new("classpath-maven");
        let project = project_from_toml(
            &temp,
            r#"
            [project]
            name = "Demo"
            version = "1.0.0"
            description = "Demo"
            natures = [ "eclipse", "maven" ]

            [dependencies.maven]
            library = { group_id = "com.example", artifact_id = "library" }

            [configuration.main]
            sources = [ "src/" ]
            dependencies = [ "library" ]
            targets = [ "target/demo.jar" ]
            "#,
        );
        let configuration = project.info().configurations().get("main").unwrap();

        let xml = generate_classpath(&project, configuration, &regexes()).unwrap();

        assert!(xml.contains("org.eclipse.m2e.MAVEN2_CLASSPATH_CONTAINER"));
        assert!(!xml.contains(".wisteria/cache/com.example/library"));
    }
}
