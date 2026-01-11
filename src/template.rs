use std::collections::HashMap;

use regex::Regex;
use reqwest::blocking::Client;
use xml::{common::XmlVersion, writer::XmlEvent, EmitterConfig, EventWriter};

use crate::{configuration::Configuration, dependency::UpdateContext, dependency::Dependency, eclipse::eq_sep_config::EclipseConfiguration, nature::Nature, project::Project, Metadata, util::files, maven::{repository, repository::ArtifactVersion}};

const PROJECT_TOML_TEMPLATE: &str = 
r#"[project] # Fill out your basic project information here
name = "{PROJECT_NAME}"
version = "0.1.0"
description = "A brief summary of this project."
natures = [ "eclipse", "maven" ] # What environments should your project be configured for?
#authors = "Me" # Optional, either a string or string array
#homepage = "http://my.website/"
#sourcepage = "https://github.com/Me/Repository/"

#-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=- 
# Add your project's required dependencies here.
# Dependencies declared here can be referenced later in project configurations.
[dependencies]

#-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=- 
# Add your project's required configurations here.
# A configuration is a collection of the data your project needs to have tasks be performed on it.
[configuration.main]
sources = [ "src/" ] # Define where Wisteria will look for source files
dependencies = [  ] # Add the dependencies you've defined above here to add them to the classpath
targets = [ "targets/{configuration}/{name}-{version}.jar" ]

#-=- End configuration! ♥ -=-
"#;

pub const WISTERIA_METADATA_TEMPLATE: &str = 
r#"dirty = true
current_configuration = "main""#;

const EDITABLE_METADATA_TEMPLATE: &str = 
r#"dirty = {dirty}
current_configuration = "{configuration}""#;

pub fn generate_metadata(metadata: &Metadata) -> String 
{
    EDITABLE_METADATA_TEMPLATE.replace("{dirty}", &metadata.dirty.to_string())
        .replace("{configuration}", &metadata.configuration)
}

const PROJECT_TOML_MINIMAL_TEMPLATE: &str = 
r#"[project]
name = "{PROJECT_NAME}"
description = "A brief summary of this project."
version = "0.1.0"
natures = [ "eclipse", "maven" ]

[dependencies]

[configuration.main]
sources = [ ]
dependencies = [ ]
targets = [ ]
"#;

pub fn generate_wisteria_project(name: &str, minimal: bool) -> String
{
    if minimal
    {
        return PROJECT_TOML_MINIMAL_TEMPLATE.replace("{PROJECT_NAME}", name);
    }

    PROJECT_TOML_TEMPLATE.replace("{PROJECT_NAME}", name)
}

pub fn generate_classpath(project: &Project, configuration: &Configuration, regexes: &HashMap<&str, Regex>) -> Result<String, String>
{
    let mut bytes: Vec<u8> = Vec::new();

    let config: EmitterConfig = EmitterConfig::new()
        .perform_indent(true)
        .indent_string(String::from("\t"));

    let mut writer = EventWriter::new_with_config(&mut bytes, config);
    writer.write(XmlEvent::StartDocument { version: XmlVersion::Version10, encoding: Some("UTF-8"), standalone: None }).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::start_element("classpath")).map_err(| e | format!("{e}"))?;

    // Add source files
    if let Some(sources) = configuration.sources()
    {
        for s in sources 
        {
            let source = XmlEvent::start_element("classpathentry")
                .attr("kind", "src")
                .attr("output", "target/classes")
                .attr("path", s);

            writer.write(source).map_err(| e | format!("{e}"))?;
            writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;
        }
    }

    // Add libraries
    if let Some(dependencies) = configuration.dependencies()
    {
        let mut width: usize = usize::MIN;
        for name in dependencies.iter()
        {
            width = usize::max(name.len(), width);
        }

        width += 5;
        let size = dependencies.len();

        println!("Dependencies: [{:?}]", &dependencies);
        for (index, d) in dependencies.iter().enumerate()
        {
            print!("({}/{size}) Resolving {:width$}", index + 1, format!("{d} ... "));
            let dependencies_opt = match project.dependencies().get(d) {
                Some(dep) => dep,
                None => {
                    println!("Unknown dependency \"{d}\"!");
                    continue
                },
            };
            match dependencies_opt.resolve(d, configuration.environment(), regexes, UpdateContext::ResolveOnly)
            {
                Ok(paths) => {
                    for path in paths 
                    {
                        let path: &str = path.to_str().unwrap();
                        let dep = XmlEvent::start_element("classpathentry") 
                            .attr("kind", "lib")
                            .attr("path", path);

                        writer.write(dep).map_err(| e | format!("{e}"))?;
                        writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;
                    }
                }
                Err((error, _)) => {
                    return Err(error)
                }
            }
        }
    }

    // Natures
    for nature in project.info().natures()
    {
        match *nature
        {
            Nature::Eclipse => {
                let container = XmlEvent::start_element("classpathentry")
                    .attr("kind", "con")
                    .attr("path", "org.eclipse.jdt.launching.JRE_CONTAINER");

                writer.write(container).map_err(| e | format!("{e}"))?;
                writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;
            }
            Nature::Maven => {
                let container = XmlEvent::start_element("classpathentry")
                    .attr("kind", "con")
                    .attr("path", "org.eclipse.m2e.MAVEN2_CLASSPATH_CONTAINER");

                writer.write(container).map_err(| e | format!("{e}"))?;
                writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;
            }
        }
    }

    writer.write(XmlEvent::start_element("classpathentry").attr("kind", "output").attr("path", "target/classes/")).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;

    Ok(String::from_utf8(bytes).unwrap())
}

pub fn generate_project(project: &Project) -> Result<String, String>
{
    let mut bytes: Vec<u8> = Vec::new();

    let config: EmitterConfig = EmitterConfig::new()
        .perform_indent(true)
        .indent_string(String::from("\t"));

    let mut writer = EventWriter::new_with_config(&mut bytes, config);
    writer.write(XmlEvent::StartDocument { version: XmlVersion::Version10, encoding: Some("UTF-8"), standalone: None }).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::start_element("projectDescription")).map_err(| e | format!("{e}"))?;

    writer.write(XmlEvent::start_element("name")).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::characters(project.info().name())).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;

    writer.write(XmlEvent::start_element("comment")).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;

    writer.write(XmlEvent::start_element("projects")).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;

    writer.write(XmlEvent::start_element("buildSpec")).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::start_element("buildCommand")).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::start_element("name")).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::characters("org.eclipse.jdt.core.javabuilder")).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::start_element("arguments")).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;

    writer.write(XmlEvent::start_element("natures")).map_err(| e | format!("{e}"))?;

    // Java nature
    writer.write(XmlEvent::start_element("nature")).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::characters("org.eclipse.jdt.core.javanature")).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;

    for nature in project.info().natures()
    {
        match *nature 
        {
            Nature::Eclipse => {}
            Nature::Maven => {
                writer.write(XmlEvent::start_element("nature")).map_err(| e | format!("{e}"))?;
                writer.write(XmlEvent::characters("org.eclipse.m2e.core.maven2Nature")).map_err(| e | format!("{e}"))?;
                writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;
            }
        }
    }
    writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;

    writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;

    Ok(String::from_utf8(bytes).unwrap())
}

pub fn generate_pom(project: &Project, configuration: &Configuration) -> Result<String, String>
{
    let mut bytes: Vec<u8> = Vec::new();

    let config: EmitterConfig = EmitterConfig::new()
        .perform_indent(true)
        .indent_string(String::from("\t"))
        .write_document_declaration(false);

    let mut writer = EventWriter::new_with_config(&mut bytes, config);
    writer.write(
        XmlEvent::start_element("project")
            .default_ns("http://maven.apache.org/POM/4.0.0")
            .ns("xsi", "http://www.w3.org/2001/XMLSchema-instance")
            .attr("xsi:schemaLocation", "http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd")
    )
        .map_err(| e | format!("{e}"))?;

    // Model version
    writer.write(XmlEvent::start_element("modelVersion")).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::characters("4.0.0")).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;

    // Project info
    writer.write(XmlEvent::start_element("groupId")).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::characters("com.example")).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;

    writer.write(XmlEvent::start_element("artifactId")).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::characters(project.info().name())).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;

    writer.write(XmlEvent::start_element("version")).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::characters(project.info().version())).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;

    writer.write(XmlEvent::start_element("properties")).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::start_element("maven.compiler.release")).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::characters(configuration.java_version().to_string().as_str())).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;
    writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;

    // Dependencies
    writer.write(XmlEvent::start_element("dependencies")).map_err(| e | format!("{e}"))?;

    let client: Client = Client::builder()
        .user_agent(files::USER_AGENT)
        .build()
        .unwrap();

    for (_, dependency) in project.dependencies()
    {
        match dependency
        {
            Dependency::FetchFromMaven { url, group_id, artifact_id, version, classifier, update_policy, javadoc } => {
                writer.write(XmlEvent::start_element("dependency")).map_err(| e | format!("{e}"))?;

                writer.write(XmlEvent::start_element("groupId")).map_err(| e | format!("{e}"))?;
                writer.write(XmlEvent::characters("{group_id}")).map_err(| e | format!("{e}"))?;
                writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;

                writer.write(XmlEvent::start_element("artifactId")).map_err(| e | format!("{e}"))?;
                writer.write(XmlEvent::characters("{artifact_id}")).map_err(| e | format!("{e}"))?;
                writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;

                writer.write(XmlEvent::start_element("version")).map_err(| e | format!("{e}"))?;
                match version 
                {
                    Some(v) => {
                        let maven_version = repository::get_version(&client, url, group_id, artifact_id, classifier.as_ref(), &ArtifactVersion::Version { version: v.clone() }).map_err(| e | format!("{e}"))?;
                        writer.write(XmlEvent::characters("{artifact_id}-{maven_version}")).map_err(| e | format!("{e}"))?;
                    },
                    None => {
                        let maven_version = repository::get_version(&client, url, group_id, artifact_id, classifier.as_ref(), &ArtifactVersion::Latest).map_err(| e | format!("{e}"))?;
                        writer.write(XmlEvent::characters("{artifact_id}-{maven_version}")).map_err(| e | format!("{e}"))?;
                    }
                }
                writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;

                writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;
            }
            _ => continue
        }
    }
    writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;

    // End
    writer.write(XmlEvent::end_element()).map_err(| e | format!("{e}"))?;

    Ok(String::from_utf8(bytes).unwrap())
}

pub fn generate_eclipse_config(configuration: &Configuration) -> EclipseConfiguration
{
    EclipseConfiguration::new()
        .add_key("eclipse.preferences.version", "1")
        .prefix("org.eclipse.jdt.core.compiler.")
        .add_key("codegen.targetPlatform", &configuration.java_version().to_string())
        .add_key("source", &configuration.java_version().to_string())

    // TODO Handle compiler flag options, such as release preview
}

pub fn generate_maven_config() -> EclipseConfiguration
{
    EclipseConfiguration::new()
        .add_key("eclipse.preferences.version", "1")
}
