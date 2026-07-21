const PROJECT_TOML_TEMPLATE: &str = r#"[project]
name = "{PROJECT_NAME}"
version = "0.1.0"
description = "A brief summary of this project."
natures = [ "eclipse", "maven" ] # What environments should your project be compatible with?
#authors = "Me"
#homepage = "http://my.website/"
#sourcepage = "https://github.com/Me/Repository

#────────────────────────────────────────────────────────────────────────────────
# Add your project's required dependencies here.
# Dependencies declared here can be referenced later in project configurations.
[dependencies]

#────────────────────────────────────────────────────────────────────────────────
# Add your project's required configurations here.
# A configuration is a collection of the data your project needs to have tasks be performed on it.
[configuration.main]
sources = [ "src/" ] # Define where Wisteria will look for source files
dependencies = [  ] # Add the dependencies you've defined above here to add them to the classpath
targets = [ "targets/{configuration}/{project_name}-{version}.jar" ]
"#;

const PROJECT_TOML_MINIMAL_TEMPLATE: &str = r#"[project]
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

pub fn generate_wisteria_project(name: &str, minimal: bool) -> String {
    if minimal {
        return PROJECT_TOML_MINIMAL_TEMPLATE.replace("{PROJECT_NAME}", name);
    }

    PROJECT_TOML_TEMPLATE.replace("{PROJECT_NAME}", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_default_project_template_with_project_name() {
        let project = generate_wisteria_project("Demo", false);

        assert!(project.contains("name = \"Demo\""));
        assert!(project.contains("natures = [ \"eclipse\", \"maven\" ]"));
        assert!(project.contains("[configuration.main]"));
        assert!(project.contains("targets = [ \"targets/{configuration}/{project_name}-{version}.jar\" ]"));
    }

    #[test]
    fn generates_minimal_project_template_with_empty_config_lists() {
        let project = generate_wisteria_project("Demo", true);

        assert!(project.contains("name = \"Demo\""));
        assert!(project.contains("sources = [ ]"));
        assert!(project.contains("dependencies = [ ]"));
        assert!(project.contains("targets = [ ]"));
        assert!(!project.contains("Add your project's required dependencies here"));
    }
}
