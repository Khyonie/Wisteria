use std::collections::HashMap;

use regex::Regex;

use crate::{
    model::{Configuration, Project},
    output::OutputRenderer,
    workspace::nature::Nature,
};

pub(crate) fn refresh(
    project: &Project,
    configuration: &Configuration,
    regexes: &HashMap<&str, Regex>,
    output: &mut dyn OutputRenderer,
) -> Result<(), (Nature, String)> {
    let removable_natures = Nature::values();
    let total = removable_natures.len() + project.info().natures().len();
    let mut step = 1;

    output.operation_started("refresh", total);
    for nature in removable_natures {
        let item = format!("{} nature", nature.type_str());
        output.step_started("refresh", "Removing", &item, step, total);

        if let Err(error) = nature.remove_nature() {
            output.step_failed("refresh", "Removing", &item, step, total, &error);
            output.operation_completed("refresh", "Refresh finished with errors.");
            return Err((nature, error));
        }

        output.step_completed("refresh", "Removing", &item, step, total, "Done");
        step += 1;
    }

    for nature in project.info().natures() {
        let item = format!("{} nature", nature.type_str());
        output.step_started("refresh", "Applying", &item, step, total);

        if let Err(e) = nature.setup_nature(project, configuration, regexes) {
            output.step_failed("refresh", "Applying", &item, step, total, &e);
            output.log(&format!(
                "Removing incomplete {} nature for project cleanliness.",
                nature.type_str()
            ));

            match nature.remove_nature() {
                Ok(_) => output.log("Removed incomplete nature."),
                Err(error) => output.log(&format!(
                    "Could not remove incomplete nature: {error}. You may have to clean the project manually."
                )),
            }
            output.operation_completed("refresh", "Refresh finished with errors.");
            return Err((nature.clone(), e));
        }

        output.step_completed("refresh", "Applying", &item, step, total, "Done");
        step += 1;
    }

    output.operation_completed(
        "refresh",
        &format!(
            "Refreshed {} configured {}",
            project.info().natures().len(),
            nature_label(project.info().natures().len())
        ),
    );
    Ok(())
}

fn nature_label(count: usize) -> &'static str {
    match count {
        1 => "nature",
        _ => "natures",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        output::{self, OutputMode},
        test_support::{TempDir, with_current_dir},
    };
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
    fn refresh_removes_stale_natures_and_applies_configured_natures() {
        let temp = TempDir::new("refresh-maven-only");
        fs::create_dir_all(temp.path().join(".settings")).unwrap();
        fs::write(temp.path().join(".project"), "stale").unwrap();
        fs::write(temp.path().join(".classpath"), "stale").unwrap();
        let project = project_from_toml(
            &temp,
            r#"
            [project]
            name = "Demo"
            version = "1.0.0"
            description = "Demo"
            natures = [ "maven" ]

            [configuration.main]
            dependencies = [ ]
            "#,
        );
        let configuration = project.info().configurations().get("main").unwrap();

        with_current_dir(temp.path(), || {
            let mut output = output::renderer(OutputMode::Plain);
            if let Err((nature, error)) =
                refresh(&project, configuration, &regexes(), output.as_mut())
            {
                panic!(
                    "expected refresh to succeed for {}: {error}",
                    nature.type_str()
                );
            }
        });

        assert!(!temp.path().join(".project").exists());
        assert!(!temp.path().join(".classpath").exists());
        assert!(temp.path().join("pom.xml").exists());
        assert!(
            temp.path()
                .join(".settings/org.eclipse.m2e.core.prefs")
                .exists()
        );
    }
}
