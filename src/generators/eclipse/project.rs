use xml::{common::XmlVersion, writer::XmlEvent, EmitterConfig, EventWriter};

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
