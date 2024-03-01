use std::{fs, path::PathBuf, process::exit};

use toml::{map::Map, Table, Value};

use crate::{silentln, Flags};

pub struct Metadata
{
    last_compiled_task: Option<String>,
    last_compilation_time: u128,
    compilation_times: Vec<u32>
}

impl Metadata
{
    pub fn from(last_compiled_task: String, last_compilation_time: u64, compilation_times: Vec<u32>) -> Self
    {
        Metadata { last_compiled_task: Some(last_compiled_task), last_compilation_time: last_compilation_time as u128, compilation_times }
    }

    pub fn to_table(&self) -> Map<String, Value>
    {
        let mut data = Table::new();

        data.insert(String::from("last_compiled_task"), Value::String(self.last_compiled_task().unwrap().to_string()));
        data.insert(String::from("last_compilation_time"), Value::Integer(self.last_compilation_time() as i64));
        
        let compilation_times: Vec<Value> = self.compilation_times().iter()
            .map(| v | Value::Integer(*v as i64))
            .collect();

        data.insert(String::from("compilation_times"), Value::Array(compilation_times));

        data
    }

    pub fn last_compiled_task(&self) -> Option<&String> 
    {
        self.last_compiled_task.as_ref()
    }

    pub fn last_compilation_time(&self) -> u128 
    {
        self.last_compilation_time
    }

    pub fn compilation_times(&self) -> &[u32] 
    {
        &self.compilation_times
    }
}

pub fn load_metadata(flags: &Flags) -> Metadata
{
    if !PathBuf::from(".wisteria").exists()
    {
        return Metadata { 
            last_compiled_task: None, 
            last_compilation_time: 0u128,
            compilation_times: Vec::new() 
        };
    }

    let wisteria_data = match fs::read_to_string(".wisteria") {
        Ok(o) => o,
        Err(e) => {
            silentln!(flags, "Could not load wisteria metadata, error: {}. Aborting...", e);
            exit(0b1111_0010);
        }
    };

    let metadata = match wisteria_data.parse::<Table>() {
        Ok(o) => o,
        Err(e) => {
            silentln!(flags, "Invalid or corrupt wisteria metadata file, error: {}. Continuing with an empty metadata file...", e);
            return Metadata {
                last_compiled_task: None, 
                last_compilation_time: 0u128, 
                compilation_times: Vec::new() 
            }
        }
    };

    let last_compiled_task: Option<String> = match metadata.get("last_compiled_task") {
        Some(v) if v.is_str() => Some(v.as_str().unwrap().to_string()),
        _ => None
    };

    let last_compilation_time: u128 = match metadata.get("last_compilation_time") {
        Some(v) if v.is_integer() => v.as_integer().unwrap() as u128,
        Some(_) => 0u128,
        None => 0u128
    };

    let compilation_times: Vec<u32> = match metadata.get("compilation_times") {
        Some(v) if v.is_array() => {
            v.as_array().unwrap().iter()
                .filter_map(| v | {
                    if let Some(i) = v.as_integer()
                    {
                        return Some(i as u32)
                    }

                    None
                })
                .collect()
        },
        _ => Vec::new()
    };

    Metadata { last_compiled_task, last_compilation_time, compilation_times }
}
