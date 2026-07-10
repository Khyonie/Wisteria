use std::{fs, path::PathBuf, process::exit};

pub fn trigger_clean(args: &[String]) {
    match args[2].to_lowercase().as_str() {
        "classes" => {
            if !PathBuf::from(".wisteria/work/bin/").exists() {
                println!("Binary folder does not exist, nothing to do.");
                exit(0)
            }
            match fs::remove_dir_all(".wisteria/work/bin/") {
                Ok(_) => println!("Operation complete."),
                Err(e) => {
                    println!("Could not remove classes folder: {e}");
                    exit(1)
                }
            }
        }
        "dependencies" => {
            if !PathBuf::from(".wisteria/cache/").exists() {
                println!("Dependency cache folder does not exist, nothing to do.");
                exit(0)
            }
            match fs::remove_dir_all(".wisteria/cache/") {
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
