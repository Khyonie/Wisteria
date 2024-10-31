use std::collections::HashMap;

/// Function that reads an equal-separated config file, like what is used by Eclipse.
pub fn read_config(config: &str) -> Result<HashMap<&str, String>, String>
{
    let mut data: HashMap<&str, String> = HashMap::new();
    for line in config.split('\n')
    {
        if line.is_empty()
        {
            continue
        }

        match line.split_once('=')
        {
            Some((k, v)) => data.insert(k, v.to_string()),
            None => {
                return Err(format!("Invalid key/value pair: \"{line}\""))
            }
        };
    }

    Ok(data)
}

pub fn generate_config(config: EclipseConfiguration) -> String
{
    let mut data: String = String::new();

    let prefix = config.get_prefix();

    for (k, v) in config.deconstruct()
    {
        data.push_str(&format!("{prefix}{k}={v}\n"));
    }

    data
}

pub struct EclipseConfiguration
{
    data: HashMap<String, String>,
    prefix: String
}

impl EclipseConfiguration
{
    pub fn new() -> Self 
    {
        EclipseConfiguration { data: HashMap::new(), prefix: String::new() }
    }

    pub fn get_prefix(&self) -> String 
    {
        self.prefix.clone()
    }

    pub fn add_key(mut self, key: &str, value: &str) -> Self
    {
        self.data.insert(key.to_string(), value.to_string());

        self
    }

    pub fn prefix(mut self, prefix: &str) -> Self
    {
        self.prefix = prefix.to_string();

        self
    }

    pub fn deconstruct(self) -> HashMap<String, String>
    {
        self.data
    }
}
