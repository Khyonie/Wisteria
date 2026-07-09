use crate::model::Metadata;

pub const WISTERIA_METADATA_TEMPLATE: &str = r#"dirty = true
current_configuration = "main""#;

const EDITABLE_METADATA_TEMPLATE: &str = r#"dirty = {dirty}
current_configuration = "{configuration}""#;

pub fn generate_metadata(metadata: &Metadata) -> String {
    EDITABLE_METADATA_TEMPLATE
        .replace("{dirty}", &metadata.dirty.to_string())
        .replace("{configuration}", &metadata.configuration)
}
