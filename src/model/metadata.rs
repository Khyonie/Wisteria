use std::{
    fs::{create_dir_all, read_to_string, write},
    io,
};

use toml::Table;

use crate::{
    config::toml_utils::{read_boolean, read_string},
    util::consts::{self, METADATA_FILE},
};

/// Project-local Wisteria state stored in `.wisteria/metadata.toml`.
pub struct Metadata {
    pub dirty: bool,
    pub configuration: String,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            dirty: false,
            configuration: String::from("main"),
        }
    }
}

impl Metadata {
    pub fn load() -> Result<Self, String> {
        let toml_string = read_metadata_file()?;

        Self::from_toml(&toml_string)
    }

    pub fn load_or_initialize() -> Result<Self, String> {
        match read_to_string(consts::METADATA_FILE) {
            Ok(toml_string) => Self::from_toml(&toml_string),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let metadata = Self::default();
                metadata.write_to_workspace()?;
                Ok(metadata)
            }
            Err(error) => Err(read_metadata_error(error)),
        }
    }

    pub fn write_to_workspace(&self) -> Result<(), String> {
        create_dir_all(consts::WISTERIA_DIR)
            .map_err(|e| format!("Could not create Wisteria metadata folder: {e}"))?;
        write(consts::METADATA_FILE, self.to_file())
            .map_err(|e| format!("Could not write {}: {e}", consts::METADATA_FILE))
    }

    pub fn to_file(&self) -> String {
        format!(
            "dirty = {}\ncurrent_configuration = \"{}\"",
            self.dirty, self.configuration
        )
    }

    fn from_toml(toml_string: &str) -> Result<Self, String> {
        let toml: Table = toml_string
            .parse::<Table>()
            .map_err(invalid_metadata_error)?;

        let dirty = read_boolean("dirty", &toml).map_err(invalid_metadata_error)?;
        let configuration =
            read_string("current_configuration", &toml).unwrap_or(String::from("main"));

        Ok(Self {
            dirty,
            configuration,
        })
    }
}

fn read_metadata_file() -> Result<String, String> {
    read_to_string(consts::METADATA_FILE).map_err(read_metadata_error)
}

fn read_metadata_error(error: io::Error) -> String {
    format!(
        "Failed to read metadata file at {}: {error}",
        consts::METADATA_FILE
    )
}

fn invalid_metadata_error(error: impl std::fmt::Display) -> String {
    format!(
        "Invalid or corrupt Wisteria metadata file at {METADATA_FILE}.\nFix: repair the file, or run `wisteria clean metadata` to reset it to the main configuration: {error}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TempDir, with_current_dir};
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
    fn load_or_initialize_creates_default_metadata_when_missing() {
        let temp = TempDir::new("metadata-load-or-initialize");

        with_current_dir(temp.path(), || {
            let metadata = Metadata::load_or_initialize().unwrap();

            assert!(!metadata.dirty);
            assert_eq!(metadata.configuration, "main");
            assert_eq!(
                fs::read_to_string(consts::METADATA_FILE).unwrap(),
                "dirty = false\ncurrent_configuration = \"main\""
            );
        });
    }

    #[test]
    fn load_still_rejects_missing_metadata_when_strict_loading_is_requested() {
        let temp = TempDir::new("metadata-strict-missing");

        with_current_dir(temp.path(), || {
            let error = match Metadata::load() {
                Ok(_) => panic!("expected missing metadata to fail"),
                Err(error) => error,
            };

            assert!(error.contains("Failed to read metadata file"));
            assert!(!temp.path().join(".wisteria/metadata.toml").exists());
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

            assert!(error.contains("Invalid or corrupt Wisteria metadata file"));
        });
    }

    #[test]
    fn load_or_initialize_rejects_corrupt_metadata_without_overwriting_it() {
        let temp = TempDir::new("metadata-initialize-corrupt");
        fs::create_dir_all(temp.path().join(".wisteria")).unwrap();
        fs::write(temp.path().join(".wisteria/metadata.toml"), "dirty =").unwrap();

        with_current_dir(temp.path(), || {
            let error = match Metadata::load_or_initialize() {
                Ok(_) => panic!("expected corrupt metadata to fail"),
                Err(error) => error,
            };

            assert!(error.contains("Invalid or corrupt Wisteria metadata file"));
            assert!(error.contains("wisteria clean metadata"));
            assert_eq!(
                fs::read_to_string(consts::METADATA_FILE).unwrap(),
                "dirty ="
            );
        });
    }
}
