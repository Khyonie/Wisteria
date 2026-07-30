use std::collections::HashMap;

pub fn generate_config(config: EclipseConfiguration) -> String {
    let mut data: String = String::new();

    let prefix = config.get_prefix();

    for (k, v) in config.deconstruct() {
        data.push_str(&format!("{prefix}{k}={v}\n"));
    }

    data
}

pub struct EclipseConfiguration {
    data: HashMap<String, String>,
    prefix: String,
}

impl EclipseConfiguration {
    pub fn new() -> Self {
        EclipseConfiguration {
            data: HashMap::new(),
            prefix: String::new(),
        }
    }

    pub fn get_prefix(&self) -> String {
        self.prefix.clone()
    }

    pub fn add_key(mut self, key: &str, value: &str) -> Self {
        self.data.insert(key.to_string(), value.to_string());

        self
    }

    pub fn prefix(mut self, prefix: &str) -> Self {
        self.prefix = prefix.to_string();

        self
    }

    pub fn deconstruct(self) -> HashMap<String, String> {
        self.data
    }
}

impl Default for EclipseConfiguration {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_tracks_prefix_and_keys() {
        let config = EclipseConfiguration::new()
            .prefix("org.example.")
            .add_key("enabled", "true");

        assert_eq!(config.get_prefix(), "org.example.");
        assert_eq!(
            config.deconstruct().get("enabled").map(String::as_str),
            Some("true")
        );
    }

    #[test]
    fn generate_config_applies_prefix_to_each_key() {
        let output = generate_config(
            EclipseConfiguration::new()
                .prefix("org.example.")
                .add_key("enabled", "true")
                .add_key("version", "1"),
        );

        assert!(output.contains("org.example.enabled=true\n"));
        assert!(output.contains("org.example.version=1\n"));
    }
}
