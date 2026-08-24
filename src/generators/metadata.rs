use crate::model::Metadata;

pub const WISTERIA_METADATA_TEMPLATE: &str = r#"dirty = true
current_configuration = "main""#;

pub fn generate_metadata(metadata: &Metadata) -> String {
    metadata.to_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_editable_metadata_file() {
        let metadata = Metadata {
            dirty: true,
            configuration: String::from("testing"),
        };

        assert_eq!(
            generate_metadata(&metadata),
            "dirty = true\ncurrent_configuration = \"testing\""
        );
    }

    #[test]
    fn default_metadata_template_points_at_main_configuration() {
        assert!(WISTERIA_METADATA_TEMPLATE.contains("dirty = true"));
        assert!(WISTERIA_METADATA_TEMPLATE.contains("current_configuration = \"main\""));
    }
}
