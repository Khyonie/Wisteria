use std::collections::HashMap;

use toml::{Value, map::Map};

use crate::build::task::{TaskOutput, TaskRunner};
use crate::model::{Configuration, Project, ProjectInfo};

#[derive(Clone)]
pub struct DefinedTask {
    phases: HashMap<String, Vec<String>>,
    phase_order: Vec<String>,
}

impl DefinedTask {
    pub fn new(_name: &str, toml: &Map<String, Value>) -> Result<Self, String> {
        let phases: HashMap<String, Vec<String>> = match toml.get("phase") {
            Some(t) if t.is_table() => {
                let mut phases: HashMap<String, Vec<String>> = HashMap::new();
                for (key, value) in t.as_table().unwrap() {
                    match value.as_array() {
                        Some(value) => {
                            let mut phase_components: Vec<String> = Vec::new();

                            for v in value {
                                match v.as_str() {
                                    Some(s) => phase_components.push(s.to_string()),
                                    None => {
                                        return Err(format!(
                                            "Mismatched type for phase element in phase \"{}\", expected a string, found {}",
                                            key,
                                            v.type_str()
                                        ));
                                    }
                                }
                            }

                            phases.insert(key.clone(), phase_components);
                        }
                        None => {
                            return Err(format!(
                                "Mismatched type for phase \"{}\", expected an array of strings, found {}",
                                key,
                                value.type_str()
                            ));
                        }
                    }
                }

                phases
            }
            Some(v) => {
                return Err(format!(
                    "Mismatched type for phase, expected a table, found {}",
                    v.type_str()
                ));
            }
            None => {
                return Err(String::from(
                    "Missing key \"phase\" which should be a table",
                ));
            }
        };

        let phase_order: Vec<String> = match toml.get("phases") {
            Some(a) if a.is_array() => {
                let mut phase_order: Vec<String> = Vec::new();
                for v in a.as_array().unwrap() {
                    match v.as_str() {
                        Some(s) => phase_order.push(s.to_string()),
                        None => {
                            return Err(format!(
                                "Mismatched type for phase order element, expected a string, found {}",
                                v.type_str()
                            ));
                        }
                    }
                }

                phase_order
            }
            Some(v) => {
                return Err(format!(
                    "Mismatched type for phase order, expected an array of strings, found {}",
                    v.type_str()
                ));
            }
            None => {
                return Err(String::from(
                    "Missing key \"phases\", which should be an array of strings",
                ));
            }
        };

        Ok(DefinedTask {
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
        output: &mut TaskOutput<'_>,
    ) -> Result<(), String> {
        for (index, phase) in self.phase_order.iter().enumerate() {
            let step = index + 1;
            output.step_started("Running", phase, step);
            let phase_actions = match self.phases.get(phase) {
                Some(a) => a,
                None => {
                    let error = format!("No phase \"{phase}\" has been defined");
                    output.step_failed("Running", phase, step, &error);
                    return Err(error);
                }
            };

            for _action in phase_actions {
                // TODO Build a command from actions
            }
            output.step_completed("Running", phase, step, "Done");
        }

        Ok(())
    }

    fn phase_order(&self) -> &[String] {
        self.phase_order.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml::Table;

    fn task_table(toml: &str) -> Table {
        toml.parse::<Table>().unwrap()
    }

    #[test]
    fn parses_defined_task_phases_and_order() {
        let toml = task_table(
            r#"
            phases = [ "prepare", "run" ]

            [phase]
            prepare = [ "echo preparing" ]
            run = [ "echo running" ]
            "#,
        );

        let task = DefinedTask::new("custom", &toml).unwrap();

        assert_eq!(
            task.phase_order(),
            &[String::from("prepare"), String::from("run")]
        );
        assert_eq!(
            task.phases.get("prepare").unwrap(),
            &vec![String::from("echo preparing")]
        );
    }

    #[test]
    fn rejects_missing_phase_table() {
        let toml = task_table(r#"phases = [ "run" ]"#);

        let error = match DefinedTask::new("custom", &toml) {
            Ok(_) => panic!("expected missing phase table to fail"),
            Err(error) => error,
        };

        assert!(error.contains("Missing key \"phase\""));
    }

    #[test]
    fn rejects_non_string_phase_action() {
        let toml = task_table(
            r#"
            phases = [ "run" ]

            [phase]
            run = [ 1 ]
            "#,
        );

        let error = match DefinedTask::new("custom", &toml) {
            Ok(_) => panic!("expected non-string phase action to fail"),
            Err(error) => error,
        };

        assert!(error.contains("Mismatched type for phase element"));
    }

    #[test]
    fn rejects_non_string_phase_order_entry() {
        let toml = task_table(
            r#"
            phases = [ "run", 1 ]

            [phase]
            run = [ "echo running" ]
            "#,
        );

        let error = match DefinedTask::new("custom", &toml) {
            Ok(_) => panic!("expected non-string phase order entry to fail"),
            Err(error) => error,
        };

        assert!(error.contains("Mismatched type for phase order element"));
    }
}
