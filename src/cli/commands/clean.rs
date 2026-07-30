use std::{fs, path::PathBuf, process::exit};

use crate::generators::generate_metadata;
use crate::model::Metadata;
use crate::util::consts;
use crate::workspace::nature::Nature;

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
        "metadata" => {
            let _ = fs::write(
                consts::METADATA_FILE,
                generate_metadata(&Metadata::default()),
            );
            println!("Operation complete.");
            exit(0)
        }
        "natures" => {
            for (index, nature) in Nature::values().iter().enumerate() {
                print!(
                    "{}/{} Removing nature {}... ",
                    index + 1,
                    Nature::values().len(),
                    nature.type_str()
                );
                match nature.remove_nature() {
                    Ok(_) => println!("Done!"),
                    Err(e) => {
                        println!("Failed to remove nature: {e}");
                        exit(1)
                    }
                }
            }

            println!("Operation complete.");
            exit(0)
        }
        _ => {
            println!("Unknown clean target {}", args[2]);
            println!("Valid clean targets: one of [ classes, dependencies, metadata, natures ]");
            exit(1)
        }
    }
}
