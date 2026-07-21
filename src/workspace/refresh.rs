use std::collections::HashMap;

use regex::Regex;

use crate::{model::{Configuration, Project}, util::consts::print_action_header, workspace::nature::Nature};

pub(crate) fn refresh(project: &Project, configuration: &Configuration, regexes: &HashMap<&str, Regex>) -> Result<(), (Nature, String)>
{
    print_action_header("Removing natures", 1, 2);
    for nature in Nature::values() {
        print!("> Removing project nature \"{}\" ... ", nature.type_str());
        nature.remove_nature()
            .map_err(| e | (nature, e))?;

        println!("Done!");
    }
    println!("Done!");

    print_action_header("Applying natures", 2, 2);
    for nature in project.info().natures() {
        print!("> Applying project nature \"{}\"... ", nature.type_str());
        if let Err(e) = nature.setup_nature(&project, configuration, &regexes)
        {
            println!("Failed: {e}");
            print!("Deleting project nature \"{}\" for project cleanliness ... ", nature.type_str());
            
            match nature.remove_nature() {
                Ok(_) => println!("Done."),
                Err(e) => {
                    println!("Failed: {e}, you may have to clean the project manually.")
                },
            }
            return Err((nature.clone(), e))
        }
        println!("Done!");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{with_current_dir, TempDir};
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
            if let Err((nature, error)) = refresh(&project, configuration, &regexes()) {
                panic!("expected refresh to succeed for {}: {error}", nature.type_str());
            }
        });

        assert!(!temp.path().join(".project").exists());
        assert!(!temp.path().join(".classpath").exists());
        assert!(temp.path().join("pom.xml").exists());
        assert!(temp.path().join(".settings/org.eclipse.m2e.core.prefs").exists());
    }
}
