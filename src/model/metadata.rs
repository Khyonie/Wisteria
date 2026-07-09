use std::fs::read_to_string;

use toml::Table;

use crate::config::toml_utils::{read_boolean, read_string};

/// Project-local Wisteria state stored in `.wisteria/metadata.toml`.
pub struct Metadata {
    pub dirty: bool,
    pub configuration: String,
}

impl Metadata {
    pub fn load() -> Result<Self, (String, u8)> {
        let toml_string =
            read_to_string(".wisteria/metadata.toml").map_err(|e| (format!("{e}"), 1))?;

        let toml: Table = toml_string.parse::<Table>()
            .map_err(| e | (format!("Invalid or corrupt Wisteria metadata file. Fix \".wisteria/metadata.toml\" in your favorite text editor, or run \"wisteria clean metadata\": {e}"), 1))?;

        let dirty = read_boolean("dirty", &toml)?;
        let configuration =
            read_string("current_configuration", &toml).unwrap_or(String::from("main"));

        Ok(Self {
            dirty,
            configuration,
        })
    }
}
