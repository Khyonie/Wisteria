use std::collections::HashMap;

use toml::{map::Map, Value};

use crate::build::task::TaskRunner;
use crate::model::{Configuration, Project, ProjectInfo};

#[derive(Clone)]
pub struct DefinedTask {
    name: String,
    phases: HashMap<String, Vec<String>>,
    phase_order: Vec<String>,
}

impl DefinedTask {
    pub fn new(name: &str, toml: &Map<String, Value>) -> Result<Self, (String, u8)> {
        let phases: HashMap<String, Vec<String>> = match toml.get("phase") {
            Some(t) if t.is_table() => {
                let mut phases: HashMap<String, Vec<String>> = HashMap::new();
                for (key, value) in t.as_table().unwrap() {
                    match value.as_array()
                    {
                        Some(value) => {
                            let mut phase_components: Vec<String> = Vec::new();

                            for v in value
                            {
                                match v.as_str()
                                {
                                    Some(s) => phase_components.push(s.to_string()),
                                    None => return Err((format!("Mismatched type for phase element in phase \"{}\", expected a string, found {}", key, v.type_str()), 15))
                                }
                            }

                            phases.insert(key.clone(), phase_components);
                        },
                        None => return Err((format!("Mismatched type for phase \"{}\", expected an array of strings, found {}", key, value.type_str()), 13))
                    }
                }

                phases
            }
            Some(v) => {
                return Err((
                    format!(
                        "Mismatched type for phase, expected a table, found {}",
                        v.type_str()
                    ),
                    16,
                ))
            }
            None => {
                return Err((
                    String::from("Missing key \"phase\" which should be a table"),
                    10,
                ))
            }
        };

        let phase_order: Vec<String> = match toml.get("phases") {
            Some(a) if a.is_array() => {
                let mut phase_order: Vec<String> = Vec::new();
                for v in a.as_array().unwrap() {
                    match v.as_str()
                    {
                        Some(s) => phase_order.push(s.to_string()),
                        None => {
                            return Err((format!("Mismatched type for phase order element, expected a string, found {}", v.type_str()), 15))
                        }
                    }
                }

                phase_order
            }
            Some(v) => {
                return Err((
                    format!(
                        "Mismatched type for phase order, expected an array of strings, found {}",
                        v.type_str()
                    ),
                    13,
                ))
            }
            None => {
                return Err((
                    String::from("Missing key \"phases\", which should be an array of strings"),
                    10,
                ))
            }
        };

        Ok(DefinedTask {
            name: name.to_string(),
            phases,
            phase_order,
        })
    }
}

impl TaskRunner for DefinedTask {
    fn invoke(
        &self,
        _info: &ProjectInfo,
        _project: &Project,
        _configuration: &Configuration,
    ) -> Result<(), (String, u8)> {
        println!("# Running task {}", self.name);
        for (index, phase) in self.phase_order.iter().enumerate() {
            println!(
                "[Phase {}/{}] Running phase {phase}",
                index + 1,
                self.phase_order.len()
            );
            let phase_actions = match self.phases.get(phase) {
                Some(a) => a,
                None => return Err((format!("No phase \"{phase}\" has been defined"), 1)),
            };

            for _action in phase_actions {
                // TODO Build a command from actions
            }
        }

        Ok(())
    }

    fn phase_order(&self) -> &[String] {
        self.phase_order.as_ref()
    }
}
