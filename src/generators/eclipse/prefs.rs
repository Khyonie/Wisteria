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
