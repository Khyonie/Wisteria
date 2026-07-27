use std::{fs, path::PathBuf, process::exit};

use crate::util::consts;

pub fn trigger_clean(args: &[String]) {
    match args[2].to_lowercase().as_str() {
        "classes" => {
            if !PathBuf::from(consts::BINARY_OUT_PATH).exists() {
                println!("Binary folder does not exist, nothing to do.");
                exit(0)
            }
            match fs::remove_dir_all(consts::BINARY_OUT_PATH) {
                Ok(_) => println!("Operation complete."),
                Err(e) => {
                    println!("Could not remove classes folder: {e}");
                    exit(1)
                }
            }
        }
        "dependencies" => {
            if !PathBuf::from(consts::CACHE_PATH).exists() {
                println!("Dependency cache folder does not exist, nothing to do.");
                exit(0)
            }
            match fs::remove_dir_all(consts::CACHE_PATH) {
                Ok(_) => println!("Operation complete."),
                Err(e) => {
                    println!("Could not remove dependency folder: {e}");
                    exit(1)
                }
            }
        }
        _ => println!("Unknown clean target {}", args[2]),
    }
}
