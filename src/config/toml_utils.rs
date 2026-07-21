use toml::{map::Map, Value};

pub fn read_string(key: &str, toml: &Map<String, Value>) -> Result<String, (String, u8)> {
    match toml.get(key) {
        Some(v) if v.is_str() => Ok(v.as_str().unwrap().to_string()),
        Some(v) => Err((
            format!(
                "Mismatched type for \"{key}\", expected a string, found {}",
                v.type_str()
            ),
            11,
        )),
        None => Err((format!("Missing key {key}"), 10)),
    }
}

pub fn read_boolean(key: &str, toml: &Map<String, Value>) -> Result<bool, (String, u8)> {
    match toml.get(key) {
        Some(v) if v.is_bool() => Ok(v.as_bool().unwrap()),
        Some(v) => Err((
            format!(
                "Mismatched type for \"{key}\", expected a boolean, found {}",
                v.type_str()
            ),
            12,
        )),
        None => Err((format!("Missing key {key}"), 10)),
    }
}

pub fn read_integer(key: &str, toml: &Map<String, Value>) -> Result<u8, (String, u8)> {
    match toml.get(key) {
        Some(v) if v.is_integer() => Ok(v.as_integer().unwrap() as u8),
        Some(v) => Err((
            format!(
                "Mismatched type for \"{key}\", expected a number, found {}",
                v.type_str()
            ),
            14,
        )),
        None => Err((format!("Missing key {key}"), 10)),
    }
}

pub fn read_string_array(
    key: &str,
    toml: &Map<String, Value>,
) -> Result<Vec<String>, (String, u8)> {
    match toml.get(key) {
        Some(v) if v.is_array() => {
            let mut data: Vec<String> = Vec::new();

            for e in v.as_array().unwrap() {
                match e.as_str() {
                    Some(s) => data.push(s.to_string()),
                    None => {
                        return Err((
                            format!(
                            "Mismatched element in string array {key}, expected a string, found {}",
                            e.type_str()
                        ),
                            15,
                        ))
                    }
                }
            }

            Ok(data)
        }
        Some(v) if v.is_str() => Ok(vec![v.as_str().unwrap().to_string()]),
        Some(v) => Err((
            format!(
                "Mismatched type for \"{key}\", expected a string array, found {}",
                v.type_str()
            ),
            13,
        )),
        None => Err((format!("Missing key {key}"), 10)),
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
