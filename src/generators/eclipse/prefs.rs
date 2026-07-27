use crate::eclipse::eq_sep_config::EclipseConfiguration;
use crate::model::Configuration;

pub fn generate_eclipse_config(configuration: &Configuration) -> EclipseConfiguration {
    EclipseConfiguration::new()
        .add_key("eclipse.preferences.version", "1")
        .prefix("org.eclipse.jdt.core.compiler.")
        .add_key(
            "codegen.targetPlatform",
            &configuration.java_version().to_string(),
        )
        .add_key("source", &configuration.java_version().to_string())
}

pub fn generate_maven_config() -> EclipseConfiguration {
    EclipseConfiguration::new().add_key("eclipse.preferences.version", "1")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eclipse::eq_sep_config::generate_config;
    use toml::Table;

    fn configuration(toml: &str) -> Configuration {
        Configuration::from(
            String::from("main"),
            &toml.parse::<Table>().unwrap(),
            String::from("Demo"),
            String::from("1.0.0"),
        )
        .unwrap()
    }

    #[test]
    fn eclipse_java_config_uses_configuration_java_version() {
        let output = generate_config(generate_eclipse_config(&configuration("java_version = 21")));

        assert!(output.contains("org.eclipse.jdt.core.compiler.codegen.targetPlatform=21\n"));
        assert!(output.contains("org.eclipse.jdt.core.compiler.source=21\n"));
        assert!(output.contains("org.eclipse.jdt.core.compiler.eclipse.preferences.version=1\n"));
    }

    #[test]
    fn maven_config_contains_preferences_version() {
        assert_eq!(
            generate_config(generate_maven_config()),
            "eclipse.preferences.version=1\n"
        );
    }
}
