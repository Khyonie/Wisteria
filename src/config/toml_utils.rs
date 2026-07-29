use toml::{map::Map, Value};

pub fn read_string(key: &str, toml: &Map<String, Value>) -> Result<String, (String, u8)> {
    match read_optional_string(key, toml)? {
        Some(value) => Ok(value),
        None => Err((missing_key_message(key, value_hint(key, "a string")), 10)),
    }
}

pub fn read_optional_string(
    key: &str,
    toml: &Map<String, Value>,
) -> Result<Option<String>, (String, u8)> {
    match toml.get(key) {
        Some(v) if v.is_str() => Ok(Some(v.as_str().unwrap().to_string())),
        Some(v) => Err((
            format!(
                "Mismatched type for \"{key}\", expected a string, found {}. {}",
                v.type_str(),
                value_hint(key, "a string")
            ),
            11,
        )),
        None => Ok(None),
    }
}

pub fn read_boolean(key: &str, toml: &Map<String, Value>) -> Result<bool, (String, u8)> {
    match toml.get(key) {
        Some(v) if v.is_bool() => Ok(v.as_bool().unwrap()),
        Some(v) => Err((
            format!(
                "Mismatched type for \"{key}\", expected a boolean, found {}. {}",
                v.type_str(),
                value_hint(key, "a boolean")
            ),
            12,
        )),
        None => Err((missing_key_message(key, value_hint(key, "a boolean")), 10)),
    }
}

pub fn read_integer(key: &str, toml: &Map<String, Value>) -> Result<u8, (String, u8)> {
    match read_optional_integer(key, toml)? {
        Some(value) => Ok(value),
        None => Err((missing_key_message(key, value_hint(key, "a number")), 10)),
    }
}

pub fn read_optional_integer(
    key: &str,
    toml: &Map<String, Value>,
) -> Result<Option<u8>, (String, u8)> {
    match toml.get(key) {
        Some(v) if v.is_integer() => {
            let value = v.as_integer().unwrap();
            if !(0..=u8::MAX as i64).contains(&value) {
                return Err((
                    format!(
                        "Invalid value for \"{key}\": expected a number from 0 to {}, found {value}. {}",
                        u8::MAX,
                        value_hint(key, "a number")
                    ),
                    14,
                ));
            }

            Ok(Some(value as u8))
        }
        Some(v) => Err((
            format!(
                "Mismatched type for \"{key}\", expected a number, found {}. {}",
                v.type_str(),
                value_hint(key, "a number")
            ),
            14,
        )),
        None => Ok(None),
    }
}

pub fn read_string_array(
    key: &str,
    toml: &Map<String, Value>,
) -> Result<Vec<String>, (String, u8)> {
    match read_optional_string_array(key, toml)? {
        Some(value) => Ok(value),
        None => Err((
            missing_key_message(key, value_hint(key, "a string array")),
            10,
        )),
    }
}

pub fn read_optional_string_array(
    key: &str,
    toml: &Map<String, Value>,
) -> Result<Option<Vec<String>>, (String, u8)> {
    match toml.get(key) {
        Some(v) if v.is_array() => {
            let mut data: Vec<String> = Vec::new();

            for (index, e) in v.as_array().unwrap().iter().enumerate() {
                match e.as_str() {
                    Some(s) => data.push(s.to_string()),
                    None => {
                        return Err((
                            format!(
                                "Mismatched element at index {index} in string array \"{key}\", expected a string, found {}. {}",
                                e.type_str(),
                                value_hint(key, "a string array")
                            ),
                            15,
                        ))
                    }
                }
            }

            Ok(Some(data))
        }
        Some(v) if v.is_str() => Ok(Some(vec![v.as_str().unwrap().to_string()])),
        Some(v) => Err((
            format!(
                "Mismatched type for \"{key}\", expected a string array, found {}. {}",
                v.type_str(),
                value_hint(key, "a string array")
            ),
            13,
        )),
        None => Ok(None),
    }
}

pub fn string_vec_to_string(data: &Vec<String>) -> String {
    let mut string: String = String::new();

    for e in data {
        string.push_str(e.as_str());
        string.push_str(", ");
    }

    string.pop();
    string.pop();

    string
}

fn missing_key_message(key: &str, hint: String) -> String {
    format!("Missing key {key}. {hint}")
}

fn value_hint(key: &str, expected: &str) -> String {
    match key {
        "name" => String::from("Fix: add `name = \"YourProjectName\"`."),
        "version" => String::from("Fix: add `version = \"0.1.0\"`."),
        "description" => String::from("Fix: add `description = \"Short project description\"`."),
        "authors" => String::from("Fix: use `authors = [ \"Your Name\" ]` or remove the key."),
        "license" => String::from("Fix: use `license = [ \"MIT\" ]` or remove the key."),
        "natures" => String::from("Fix: use `natures = [ \"eclipse\", \"maven\" ]` or remove the key."),
        "sources" => String::from("Fix: use `sources = [ \"src/\" ]` or `sources = \"src/\"`."),
        "dependencies" => String::from("Fix: use `dependencies = [ \"dependency-name\" ]` or remove the key."),
        "shaded" => String::from("Fix: use `shaded = [ \"dependency-name\" ]` or remove the key."),
        "includes" => String::from("Fix: use `includes = [ \"plugin.yml\" ]` or remove the key."),
        "targets" => String::from("Fix: use `targets = [ \"target/{project_name}.jar\" ]` or `targets = \"target/{project_name}.jar\"`."),
        "entry" => String::from("Fix: use `entry = \"com.example.Main\"` or remove the key."),
        "java_version" => String::from("Fix: use a numeric Java release, for example `java_version = 17`."),
        "inherit" => String::from("Fix: use `inherit = \"base-configuration\"` or remove the key."),
        "update_policy" => String::from("Fix: use a supported update policy such as `SwitchOrUpdate`, `TaskOrUpdate`, or `Never`."),
        "group_id" => String::from("Fix: use Maven coordinates, for example `group_id = \"com.example\"`."),
        "artifact_id" => String::from("Fix: use Maven coordinates, for example `artifact_id = \"library\"`."),
        "repository" => String::from("Fix: use a repository name, or for GitHub use `repository = \"Owner/Repository\"`."),
        "path" => String::from("Fix: use a quoted path, for example `path = \"lib/library.jar\"`."),
        "url" => String::from("Fix: use a quoted URL, for example `url = \"https://example.com/library.jar\"`."),
        "javadoc" => String::from("Fix: use a quoted Javadoc URL, for example `javadoc = \"https://example.com/docs/\"`."),
        _ => format!("Fix: set `{key}` to {expected}, or remove it if it is optional."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml::Table;

    fn table(toml: &str) -> Table {
        toml.parse::<Table>().unwrap()
    }

    #[test]
    fn reads_expected_scalar_types() {
        let toml = table(
            r#"
            name = "Demo"
            enabled = true
            version = 17
            "#,
        );

        assert_eq!(read_string("name", &toml).as_deref(), Ok("Demo"));
        assert_eq!(read_boolean("enabled", &toml), Ok(true));
        assert_eq!(read_integer("version", &toml), Ok(17));
    }

    #[test]
    fn reports_type_mismatches_with_specific_codes() {
        let toml = table(
            r#"
            name = 12
            enabled = "yes"
            version = "17"
            "#,
        );

        assert_eq!(read_string("name", &toml).unwrap_err().1, 11);
        assert_eq!(read_boolean("enabled", &toml).unwrap_err().1, 12);
        assert_eq!(read_integer("version", &toml).unwrap_err().1, 14);
    }

    #[test]
    fn reads_string_arrays_and_string_shorthand() {
        let toml = table(
            r#"
            source = "src/"
            dependencies = [ "a", "b" ]
            "#,
        );

        assert_eq!(read_string_array("source", &toml).unwrap(), vec!["src/"]);
        assert_eq!(
            read_string_array("dependencies", &toml).unwrap(),
            vec!["a", "b"]
        );
    }

    #[test]
    fn rejects_non_string_array_elements() {
        let toml = table(r#"dependencies = [ "a", 1 ]"#);

        let error = read_string_array("dependencies", &toml).unwrap_err();

        assert!(error.0.contains("Mismatched element"));
        assert_eq!(error.1, 15);
    }

    #[test]
    fn joins_string_vectors_with_commas() {
        let values = vec![
            String::from("alpha"),
            String::from("beta"),
            String::from("gamma"),
        ];

        assert_eq!(string_vec_to_string(&values), "alpha, beta, gamma");
    }
}
