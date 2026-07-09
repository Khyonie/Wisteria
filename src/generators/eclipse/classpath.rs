use std::collections::HashMap;

use regex::Regex;
use xml::{common::XmlVersion, writer::XmlEvent, EmitterConfig, EventWriter};

use crate::dependency::{Dependency, UpdateContext};
use crate::model::{Configuration, Project};
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
                .attr("output", "target/classes")
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
                .attr("path", "target/classes/"),
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
