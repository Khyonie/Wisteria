use std::collections::BTreeMap;

use reqwest::blocking::Client;
use xml::{writer::XmlEvent, EmitterConfig, EventWriter};

use crate::dependency::Dependency;
use crate::maven::{repository, repository::ArtifactVersion};
use crate::model::{Configuration, Project};
use crate::workspace::download;

const DEFAULT_MAVEN_CENTRAL: &str = "https://repo1.maven.org/maven2";

#[allow(unused)]
pub fn generate_pom(project: &Project, configuration: &Configuration) -> Result<String, String> {
    let mut bytes: Vec<u8> = Vec::new();

    let config: EmitterConfig = EmitterConfig::new()
        .perform_indent(true)
        .indent_string(String::from("\t"))
        .write_document_declaration(false);

    let mut writer = EventWriter::new_with_config(&mut bytes, config);
    writer
        .write(
            XmlEvent::start_element("project")
                .default_ns("http://maven.apache.org/POM/4.0.0")
                .ns("xsi", "http://www.w3.org/2001/XMLSchema-instance")
                .attr(
                    "xsi:schemaLocation",
                    "http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd",
                ),
        )
        .map_err(|e| e.to_string())?;

    write_text_element(&mut writer, "modelVersion", "4.0.0")?;
    write_text_element(&mut writer, "groupId", "com.example")?;
    write_text_element(&mut writer, "artifactId", project.info().name())?;
    write_text_element(&mut writer, "version", project.info().version())?;

    writer
        .write(XmlEvent::start_element("properties"))
        .map_err(|e| e.to_string())?;
    write_text_element(
        &mut writer,
        "maven.compiler.release",
        &configuration.java_version().to_string(),
    )?;
    writer
        .write(XmlEvent::end_element())
        .map_err(|e| e.to_string())?;

    let repositories = collect_repositories(project, configuration);
    if !repositories.is_empty() {
        write_repositories(&mut writer, &repositories)?;
    }

    writer
        .write(XmlEvent::start_element("dependencies"))
        .map_err(|e| e.to_string())?;

    let client: Client = Client::builder()
        .user_agent(download::USER_AGENT)
        .build()
        .unwrap();

    let Some(configuration_dependencies) = configuration.dependencies() else {
        writer
            .write(XmlEvent::end_element())
            .map_err(|e| e.to_string())?;
        writer
            .write(XmlEvent::end_element())
            .map_err(|e| e.to_string())?;

        return Ok(String::from_utf8(bytes).unwrap());
    };

    for dependency_name in configuration_dependencies {
        let Some(dependency) = project.dependencies().get(dependency_name) else {
            continue;
        };

        match dependency {
            Dependency::FetchFromMaven {
                url,
                group_id,
                artifact_id,
                version,
                classifier,
                ..
            } => {
                writer
                    .write(XmlEvent::start_element("dependency"))
                    .map_err(|e| e.to_string())?;

                write_text_element(&mut writer, "groupId", group_id)?;
                write_text_element(&mut writer, "artifactId", artifact_id)?;

                let target_version = artifact_version(version.as_ref());
                let maven_version = repository::get_version(
                    &client,
                    url,
                    group_id,
                    artifact_id,
                    classifier.as_ref(),
                    &target_version,
                )
                .map_err(|e| e.to_string())?;
                write_text_element(&mut writer, "version", &maven_version.0)?;

                if let Some(classifier) = classifier {
                    write_text_element(&mut writer, "classifier", classifier)?;
                }

                writer
                    .write(XmlEvent::end_element())
                    .map_err(|e| e.to_string())?;
            }
            _ => continue,
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

fn collect_repositories(
    project: &Project,
    configuration: &Configuration,
) -> BTreeMap<String, String> {
    let mut repositories: BTreeMap<String, String> = BTreeMap::new();

    let Some(configuration_dependencies) = configuration.dependencies() else {
        return repositories;
    };

    for dependency_name in configuration_dependencies {
        let Some(Dependency::FetchFromMaven { url, .. }) =
            project.dependencies().get(dependency_name)
        else {
            continue;
        };

        if is_default_maven_central(url) || repositories.values().any(|known_url| known_url == url)
        {
            continue;
        }

        let id = format!("wisteria-repository-{}", repositories.len() + 1);
        repositories.insert(id, url.clone());
    }

    repositories
}

fn write_repositories<W: std::io::Write>(
    writer: &mut EventWriter<W>,
    repositories: &BTreeMap<String, String>,
) -> Result<(), String> {
    writer
        .write(XmlEvent::start_element("repositories"))
        .map_err(|e| e.to_string())?;

    for (id, url) in repositories {
        writer
            .write(XmlEvent::start_element("repository"))
            .map_err(|e| e.to_string())?;
        write_text_element(writer, "id", id)?;
        write_text_element(writer, "url", url)?;

        writer
            .write(XmlEvent::start_element("releases"))
            .map_err(|e| e.to_string())?;
        write_text_element(writer, "enabled", "true")?;
        writer
            .write(XmlEvent::end_element())
            .map_err(|e| e.to_string())?;

        writer
            .write(XmlEvent::start_element("snapshots"))
            .map_err(|e| e.to_string())?;
        write_text_element(writer, "enabled", "true")?;
        writer
            .write(XmlEvent::end_element())
            .map_err(|e| e.to_string())?;

        writer
            .write(XmlEvent::end_element())
            .map_err(|e| e.to_string())?;
    }

    writer
        .write(XmlEvent::end_element())
        .map_err(|e| e.to_string())
}

fn artifact_version(version: Option<&String>) -> ArtifactVersion {
    match version.map(|version| version.as_str()) {
        Some("latest") | None => ArtifactVersion::Latest,
        Some("release") => ArtifactVersion::Release,
        Some(version) => ArtifactVersion::Version {
            version: version.to_string(),
        },
    }
}

fn is_default_maven_central(url: &str) -> bool {
    url.trim_end_matches('/') == DEFAULT_MAVEN_CENTRAL
}

fn write_text_element<W: std::io::Write>(
    writer: &mut EventWriter<W>,
    name: &str,
    text: &str,
) -> Result<(), String> {
    writer
        .write(XmlEvent::start_element(name))
        .map_err(|e| e.to_string())?;
    writer
        .write(XmlEvent::characters(text))
        .map_err(|e| e.to_string())?;
    writer
        .write(XmlEvent::end_element())
        .map_err(|e| e.to_string())
}
