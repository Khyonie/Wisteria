use toml::{map::Map, Value};

pub fn read_string(key: &str, toml: &Map<String, Value>) -> Result<String, (String, u8)>
{
    match toml.get(key)
    {
        Some(v) if v.is_str() => Ok(v.as_str().unwrap().to_string()),
        Some(v) => Err((format!("Mismatched type for \"{key}\", expected a string, found {}", v.type_str()), 11)),
        None => Err((format!("Missing key {key}"), 10))
    }
}

pub fn read_boolean(key: &str, toml: &Map<String, Value>) -> Result<bool, (String, u8)>
{
    match toml.get(key)
    {
        Some(v) if v.is_bool() => Ok(v.as_bool().unwrap()),
        Some(v) => Err((format!("Mismatched type for \"{key}\", expected a boolean, found {}", v.type_str()), 12)),
        None => Err((format!("Missing key {key}"), 10))
    }
}

pub fn read_integer(key: &str, toml: &Map<String, Value>) -> Result<u8, (String, u8)>
{
    match toml.get(key)
    {
        Some(v) if v.is_integer() => Ok(v.as_integer().unwrap() as u8),
        Some(v) => Err((format!("Mismatched type for \"{key}\", expected a number, found {}", v.type_str()), 14)),
        None => Err((format!("Missing key {key}"), 10))
    }
}

pub fn read_string_array(key: &str, toml: &Map<String, Value>) -> Result<Vec<String>, (String, u8)>
{
    match toml.get(key)
    {
        Some(v) if v.is_array() => {
            let mut data: Vec<String> = Vec::new();

            for e in v.as_array().unwrap()
            {
                match e.as_str()
                {
                    Some(s) => data.push(s.to_string()),
                    None => return Err((format!("Mismatched element in string array {key}, expected a string, found {}", e.type_str()), 15))
                }
            }

            Ok(data)
        }
        Some(v) if v.is_str() => Ok(vec!(v.as_str().unwrap().to_string())),
        Some(v) => Err((format!("Mismatched type for \"{key}\", expected a string array, found {}", v.type_str()), 13)),
        None => Err((format!("Missing key {key}"), 10))
    }
}

pub fn string_vec_to_string(data: &Vec<String>) -> String
{
    let mut string: String = String::new();

    for e in data 
    {
        string.push_str(e.as_str());
        string.push_str(", ");
    }

    string.pop();
    string.pop();

    string
}
