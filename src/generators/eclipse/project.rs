use xml::{EmitterConfig, EventWriter, common::XmlVersion, writer::XmlEvent};

use crate::model::Project;
use crate::workspace::nature::Nature;

pub fn generate_project(project: &Project) -> Result<String, String> {
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
        .write(XmlEvent::start_element("projectDescription"))
        .map_err(|e| e.to_string())?;

    writer
        .write(XmlEvent::start_element("name"))
        .map_err(|e| e.to_string())?;
    writer
        .write(XmlEvent::characters(project.info().name()))
        .map_err(|e| e.to_string())?;
    writer
        .write(XmlEvent::end_element())
        .map_err(|e| e.to_string())?;

    writer
        .write(XmlEvent::start_element("comment"))
        .map_err(|e| e.to_string())?;
    writer
        .write(XmlEvent::end_element())
        .map_err(|e| e.to_string())?;

    writer
        .write(XmlEvent::start_element("projects"))
        .map_err(|e| e.to_string())?;
    writer
        .write(XmlEvent::end_element())
        .map_err(|e| e.to_string())?;

    writer
        .write(XmlEvent::start_element("buildSpec"))
        .map_err(|e| e.to_string())?;
    writer
        .write(XmlEvent::start_element("buildCommand"))
        .map_err(|e| e.to_string())?;
    writer
        .write(XmlEvent::start_element("name"))
        .map_err(|e| e.to_string())?;
    writer
        .write(XmlEvent::characters("org.eclipse.jdt.core.javabuilder"))
        .map_err(|e| e.to_string())?;
    writer
        .write(XmlEvent::end_element())
        .map_err(|e| e.to_string())?;
    writer
        .write(XmlEvent::start_element("arguments"))
        .map_err(|e| e.to_string())?;
    writer
        .write(XmlEvent::end_element())
        .map_err(|e| e.to_string())?;
    writer
        .write(XmlEvent::end_element())
        .map_err(|e| e.to_string())?;
    writer
        .write(XmlEvent::end_element())
        .map_err(|e| e.to_string())?;

    writer
        .write(XmlEvent::start_element("natures"))
        .map_err(|e| e.to_string())?;

    writer
        .write(XmlEvent::start_element("nature"))
        .map_err(|e| e.to_string())?;
    writer
        .write(XmlEvent::characters("org.eclipse.jdt.core.javanature"))
        .map_err(|e| e.to_string())?;
    writer
        .write(XmlEvent::end_element())
        .map_err(|e| e.to_string())?;

    for nature in project.info().natures() {
        match *nature {
            Nature::Eclipse => {}
            Nature::Maven => {
                writer
                    .write(XmlEvent::start_element("nature"))
                    .map_err(|e| e.to_string())?;
                writer
                    .write(XmlEvent::characters("org.eclipse.m2e.core.maven2Nature"))
                    .map_err(|e| e.to_string())?;
                writer
                    .write(XmlEvent::end_element())
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    writer
        .write(XmlEvent::end_element())
        .map_err(|e| e.to_string())?;

    writer
        .write(XmlEvent::end_element())
        .map_err(|e| e.to_string())?;

    Ok(String::from_utf8(bytes).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;
    use std::fs;

    fn project_from_toml(contents: &str) -> Project {
        let temp = TempDir::new("eclipse-project-generator");
        let project_file = temp.path().join("project.toml");
        fs::write(&project_file, contents).unwrap();

        Project::from(Some(project_file.to_string_lossy().to_string())).unwrap()
    }

    #[test]
    fn generated_project_xml_contains_project_name_and_java_nature() {
        let project = project_from_toml(
            r#"
            [project]
            name = "Demo"
            version = "1.0.0"
            description = "Demo"
            natures = [ "eclipse" ]
            "#,
        );

        let xml = generate_project(&project).unwrap();

        assert!(xml.contains("<name>Demo</name>"));
        assert!(xml.contains("org.eclipse.jdt.core.javabuilder"));
        assert!(xml.contains("org.eclipse.jdt.core.javanature"));
        assert!(!xml.contains("org.eclipse.m2e.core.maven2Nature"));
    }

    #[test]
    fn generated_project_xml_includes_maven_nature_when_configured() {
        let project = project_from_toml(
            r#"
            [project]
            name = "Demo"
            version = "1.0.0"
            description = "Demo"
            natures = [ "eclipse", "maven" ]
            "#,
        );

        let xml = generate_project(&project).unwrap();

        assert!(xml.contains("org.eclipse.m2e.core.maven2Nature"));
    }
}
