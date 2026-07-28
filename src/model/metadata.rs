use std::fs::read_to_string;

use toml::Table;

use crate::{config::toml_utils::{read_boolean, read_string}, util::consts::{self, METADATA_FILE}};

/// Project-local Wisteria state stored in `.wisteria/metadata.toml`.
pub struct Metadata {
    pub dirty: bool,
    pub configuration: String,
}

impl Default for Metadata
{
    fn default() -> Self {
        Self { dirty: false, configuration: String::from("main") }
    }
}

impl Metadata {
    pub fn load() -> Result<Self, (String, u8)> {
        let toml_string =
            read_to_string(consts::METADATA_FILE).map_err(|e| (format!("Failed to read metadata file at {}: {e}", consts::METADATA_FILE), 1))?;

        let toml: Table = toml_string.parse::<Table>()
            .map_err(| e | (format!("Invalid or corrupt Wisteria metadata file. Fix \"{}\" in your favorite text editor, or run \"wisteria clean metadata\": {e}", METADATA_FILE), 1))?;

        let dirty = read_boolean("dirty", &toml)?;
        let configuration =
            read_string("current_configuration", &toml).unwrap_or(String::from("main"));

        Ok(Self {
            dirty,
            configuration,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{with_current_dir, TempDir};
    use std::fs;

    #[test]
    fn default_metadata_uses_main_configuration_and_clean_state() {
        let metadata = Metadata::default();

        assert!(!metadata.dirty);
        assert_eq!(metadata.configuration, "main");
    }

    #[test]
    fn loads_metadata_from_workspace_file() {
        let temp = TempDir::new("metadata-load");
        fs::create_dir_all(temp.path().join(".wisteria")).unwrap();
        fs::write(
            temp.path().join(".wisteria/metadata.toml"),
            r#"
            dirty = true
            current_configuration = "testing"
            "#,
        )
        .unwrap();

        with_current_dir(temp.path(), || {
            let metadata = Metadata::load().unwrap();

            assert!(metadata.dirty);
            assert_eq!(metadata.configuration, "testing");
        });
    }

    #[test]
    fn defaults_missing_current_configuration_to_main() {
        let temp = TempDir::new("metadata-default-config");
        fs::create_dir_all(temp.path().join(".wisteria")).unwrap();
        fs::write(temp.path().join(".wisteria/metadata.toml"), "dirty = false").unwrap();

        with_current_dir(temp.path(), || {
            let metadata = Metadata::load().unwrap();

            assert!(!metadata.dirty);
            assert_eq!(metadata.configuration, "main");
        });
    }

    #[test]
    fn rejects_corrupt_metadata_toml() {
        let temp = TempDir::new("metadata-corrupt");
        fs::create_dir_all(temp.path().join(".wisteria")).unwrap();
        fs::write(temp.path().join(".wisteria/metadata.toml"), "dirty =").unwrap();

        with_current_dir(temp.path(), || {
            let error = match Metadata::load() {
                Ok(_) => panic!("expected corrupt metadata to fail"),
                Err(error) => error,
            };

            assert!(error.0.contains("Invalid or corrupt Wisteria metadata file"));
            assert_eq!(error.1, 1);
        });
    }
}
