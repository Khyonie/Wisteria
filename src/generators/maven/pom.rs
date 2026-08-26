use std::path::Path;

use xml::{EmitterConfig, EventWriter, writer::XmlEvent};

use crate::dependency::Dependency;
use crate::model::lockfile::try_read_lockfile;
use crate::model::{Configuration, Lockfile, LockfileArtifact, Project};
use crate::util::consts;

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

    writer
        .write(XmlEvent::start_element("dependencies"))
        .map_err(|e| e.to_string())?;

    let Some(configuration_dependencies) = configuration.dependencies() else {
        writer
            .write(XmlEvent::end_element())
            .map_err(|e| e.to_string())?;
        writer
            .write(XmlEvent::end_element())
            .map_err(|e| e.to_string())?;

        return Ok(String::from_utf8(bytes).unwrap());
    };

    let mut loaded_lockfile: Option<Option<Lockfile>> = None;

    for dependency_reference in configuration_dependencies {
        let Some(dependency) = project.dependencies().get(dependency_reference.name()) else {
            continue;
        };

        if !matches!(dependency, Dependency::FetchFromMaven { .. }) {
            continue;
        }

        let lockfile = match loaded_lockfile {
            Some(ref lockfile) => lockfile,
            None => loaded_lockfile.insert(try_read_lockfile()?),
        };

        let system_dependency =
            maven_system_dependency(dependency_reference.name(), dependency, lockfile.as_ref())?;
        write_maven_system_dependency(&mut writer, &system_dependency)?;
    }
    writer
        .write(XmlEvent::end_element())
        .map_err(|e| e.to_string())?;

    writer
        .write(XmlEvent::end_element())
        .map_err(|e| e.to_string())?;

    Ok(String::from_utf8(bytes).unwrap())
}

struct MavenSystemDependency<'a> {
    group_id: &'a str,
    artifact_id: &'a str,
    version: &'a str,
    classifier: Option<&'a str>,
    system_path: String,
}

fn maven_system_dependency<'a>(
    name: &str,
    dependency: &'a Dependency,
    lockfile: Option<&'a Lockfile>,
) -> Result<MavenSystemDependency<'a>, String> {
    let Dependency::FetchFromMaven {
        group_id,
        artifact_id,
        classifier,
        ..
    } = dependency
    else {
        unreachable!("maven_system_dependency only accepts Maven dependencies");
    };

    let artifact = matching_lockfile_artifact(name, dependency, lockfile)?;
    let version = artifact.version().ok_or_else(|| {
        format!(
            "Lockfile artifact for Maven dependency \"{name}\" does not include a resolved version.\nFix: run `wisteria update {name}` to resolve the version and regenerate `{}`.",
            consts::LOCKFILE
        )
    })?;

    ensure_system_path_exists(name, artifact.cache_path())?;

    Ok(MavenSystemDependency {
        group_id,
        artifact_id,
        version,
        classifier: classifier.as_deref(),
        system_path: maven_system_path(artifact.cache_path()),
    })
}

fn matching_lockfile_artifact<'a>(
    name: &str,
    dependency: &Dependency,
    lockfile: Option<&'a Lockfile>,
) -> Result<&'a LockfileArtifact, String> {
    let Some(lockfile) = lockfile else {
        return Err(format!(
            "Cannot generate Maven POM system dependency for \"{name}\" because `{}` does not exist.\nFix: run `wisteria sync {name}` if the artifact is already cached, or `wisteria update {name}` to resolve, download, and lock it.",
            consts::LOCKFILE
        ));
    };

    let artifacts: Vec<&LockfileArtifact> = lockfile
        .artifacts()
        .iter()
        .filter(|artifact| artifact.name() == name)
        .collect();
    let matching_artifacts: Vec<&LockfileArtifact> = artifacts
        .iter()
        .copied()
        .filter(|artifact| dependency.matches_lockfile_artifact(artifact))
        .collect();

    match matching_artifacts.as_slice() {
        [artifact] => Ok(*artifact),
        [] if artifacts.is_empty() => Err(format!(
            "Cannot generate Maven POM system dependency for \"{name}\" because `{}` has no matching artifact.\nFix: run `wisteria sync {name}` if the artifact is already cached, or `wisteria update {name}` to resolve, download, and lock it.",
            consts::LOCKFILE
        )),
        [] => Err(format!(
            "Cannot generate Maven POM system dependency for \"{name}\" because the artifact in `{}` does not match project.toml.\nFix: run `wisteria sync {name}` if project.toml is current, or `wisteria update {name}` to resolve and download it again.",
            consts::LOCKFILE
        )),
        _ => Err(format!(
            "Cannot generate Maven POM system dependency for \"{name}\" because `{}` has multiple matching artifacts.\nFix: run `wisteria update {name}` to rewrite a single lock entry.",
            consts::LOCKFILE
        )),
    }
}

fn ensure_system_path_exists(name: &str, cache_path: &str) -> Result<(), String> {
    if Path::new(cache_path).exists() {
        return Ok(());
    }

    Err(format!(
        "Cannot generate Maven POM system dependency for \"{name}\" because the locked artifact is not cached at `{cache_path}`.\nFix: run `wisteria fetch {name}` to download the artifact recorded in `{}`, or `wisteria update {name}` if the lockfile is stale.",
        consts::LOCKFILE
    ))
}

fn maven_system_path(cache_path: &str) -> String {
    let normalized = cache_path.replace('\\', "/");

    if Path::new(cache_path).is_absolute() {
        normalized
    } else {
        format!("${{project.basedir}}/{normalized}")
    }
}

fn write_maven_system_dependency<W: std::io::Write>(
    writer: &mut EventWriter<W>,
    dependency: &MavenSystemDependency<'_>,
) -> Result<(), String> {
    writer
        .write(XmlEvent::start_element("dependency"))
        .map_err(|e| e.to_string())?;

    write_text_element(writer, "groupId", dependency.group_id)?;
    write_text_element(writer, "artifactId", dependency.artifact_id)?;
    write_text_element(writer, "version", dependency.version)?;

    if let Some(classifier) = dependency.classifier {
        write_text_element(writer, "classifier", classifier)?;
    }

    write_text_element(writer, "scope", "system")?;
    write_text_element(writer, "systemPath", &dependency.system_path)?;

    writer
        .write(XmlEvent::end_element())
        .map_err(|e| e.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::{LockfileArtifact, lockfile::lockfile_artifacts_to_toml},
        test_support::{TempDir, with_current_dir},
        workspace::files,
    };
    use std::{fs, path::Path};

    fn project_from_toml(temp: &TempDir, contents: &str) -> Project {
        let project_file = temp.path().join("project.toml");
        fs::write(&project_file, contents).unwrap();

        Project::from(Some(project_file.to_string_lossy().to_string())).unwrap()
    }

    #[test]
    fn generate_pom_writes_basic_project_coordinates_and_java_release() {
        let temp = TempDir::new("pom-basic");
        let project = project_from_toml(
            &temp,
            r#"
            [project]
            name = "Demo"
            version = "1.2.3"
            description = "Demo"

            [configuration.main]
            java_version = 21
            "#,
        );
        let configuration = project.info().configurations().get("main").unwrap();

        let pom = with_current_dir(temp.path(), || {
            generate_pom(&project, configuration).unwrap()
        });

        assert!(pom.contains("<groupId>com.example</groupId>"));
        assert!(pom.contains("<artifactId>Demo</artifactId>"));
        assert!(pom.contains("<version>1.2.3</version>"));
        assert!(pom.contains("<maven.compiler.release>21</maven.compiler.release>"));
        assert!(pom.contains("<dependencies />") || pom.contains("<dependencies>"));
    }

    #[test]
    fn generate_pom_writes_maven_dependencies_as_system_paths_from_lockfile() {
        let temp = TempDir::new("pom-system-dependencies");
        let cache_path = ".wisteria/cache/com.example/library/1.2.3/library.jar";
        let fetch_url = "https://repo.example/com/example/library/1.2.3/library-1.2.3.jar";
        write_cached_lockfile_artifact(&temp, "library", "1.2.3", fetch_url, cache_path);

        let project = project_from_toml(
            &temp,
            r#"
            [project]
            name = "Demo"
            version = "1.0.0"
            description = "Demo"

            [dependencies.maven]
            library = { url = "https://repo.example", group_id = "com.example", artifact_id = "library", version = "latest" }

            [configuration.main]
            dependencies = [ "library" ]
            "#,
        );
        let configuration = project.info().configurations().get("main").unwrap();

        let pom = with_current_dir(temp.path(), || {
            generate_pom(&project, configuration).unwrap()
        });

        assert!(pom.contains("<groupId>com.example</groupId>"));
        assert!(pom.contains("<artifactId>library</artifactId>"));
        assert!(pom.contains("<version>1.2.3</version>"));
        assert!(pom.contains("<scope>system</scope>"));
        assert!(pom.contains(
            "<systemPath>${project.basedir}/.wisteria/cache/com.example/library/1.2.3/library.jar</systemPath>"
        ));
        assert!(!pom.contains("<repositories>"));
    }

    #[test]
    fn generate_pom_does_not_read_lockfile_without_maven_dependencies() {
        let temp = TempDir::new("pom-non-maven-dependencies");
        fs::write(
            temp.path().join(Path::new(consts::LOCKFILE)),
            "schema = \"bad\"",
        )
        .unwrap();
        let project = project_from_toml(
            &temp,
            r#"
            [project]
            name = "Demo"
            version = "1.0.0"
            description = "Demo"

            [dependencies.archive]
            library = { path = "lib/library.jar" }

            [configuration.main]
            dependencies = [ "library" ]
            "#,
        );
        let configuration = project.info().configurations().get("main").unwrap();

        let pom = with_current_dir(temp.path(), || {
            generate_pom(&project, configuration).unwrap()
        });

        assert!(pom.contains("<artifactId>Demo</artifactId>"));
        assert!(pom.contains("<dependencies />") || pom.contains("<dependencies>"));
    }

    #[test]
    fn generate_pom_requires_lockfile_for_maven_dependencies() {
        let temp = TempDir::new("pom-missing-lockfile");
        let project = project_from_toml(
            &temp,
            r#"
            [project]
            name = "Demo"
            version = "1.0.0"
            description = "Demo"

            [dependencies.maven]
            library = { url = "https://repo.example", group_id = "com.example", artifact_id = "library", version = "1.2.3" }

            [configuration.main]
            dependencies = [ "library" ]
            "#,
        );
        let configuration = project.info().configurations().get("main").unwrap();

        let error = with_current_dir(temp.path(), || {
            generate_pom(&project, configuration).unwrap_err()
        });

        assert!(error.contains("wisteria.lock"));
        assert!(error.contains("wisteria sync library"));
        assert!(error.contains("wisteria update library"));
    }

    #[test]
    fn generate_pom_rejects_missing_cached_system_path() {
        let temp = TempDir::new("pom-missing-system-path");
        let cache_path = ".wisteria/cache/com.example/library/1.2.3/library.jar";
        let fetch_url = "https://repo.example/com/example/library/1.2.3/library-1.2.3.jar";
        write_lockfile_artifact(
            &temp,
            LockfileArtifact::new(
                String::from("library"),
                String::from("maven"),
                Some(String::from("1.2.3")),
                String::from(fetch_url),
                String::from(cache_path),
                String::from("hash"),
            ),
        );

        let project = project_from_toml(
            &temp,
            r#"
            [project]
            name = "Demo"
            version = "1.0.0"
            description = "Demo"

            [dependencies.maven]
            library = { url = "https://repo.example", group_id = "com.example", artifact_id = "library", version = "1.2.3" }

            [configuration.main]
            dependencies = [ "library" ]
            "#,
        );
        let configuration = project.info().configurations().get("main").unwrap();

        let error = with_current_dir(temp.path(), || {
            generate_pom(&project, configuration).unwrap_err()
        });

        assert!(error.contains(cache_path));
        assert!(error.contains("wisteria fetch library"));
    }

    fn write_cached_lockfile_artifact(
        temp: &TempDir,
        name: &str,
        version: &str,
        fetch_url: &str,
        cache_path: &str,
    ) {
        let full_cache_path = temp.path().join(cache_path);
        fs::create_dir_all(full_cache_path.parent().unwrap()).unwrap();
        fs::write(&full_cache_path, "jar").unwrap();
        let hash = files::generate_sha2_for_file(&full_cache_path).unwrap();

        write_lockfile_artifact(
            temp,
            LockfileArtifact::new(
                String::from(name),
                String::from("maven"),
                Some(String::from(version)),
                String::from(fetch_url),
                String::from(cache_path),
                hash,
            ),
        );
    }

    fn write_lockfile_artifact(temp: &TempDir, artifact: LockfileArtifact) {
        fs::write(
            temp.path().join(Path::new(consts::LOCKFILE)),
            lockfile_artifacts_to_toml(vec![artifact]).unwrap(),
        )
        .unwrap();
    }
}
