use std::{
    fs::{self, File},
    path::PathBuf,
};

use crate::model::Configuration;
use crate::util::consts;
use crate::workspace::files;

pub fn collect_sources(configuration: &Configuration) -> Result<Vec<String>, (String, u8)> {
    match configuration.sources() {
        Some(sources) => {
            if sources.is_empty() {
                return Err((
                    String::from("No source folders given, nothing to compile"),
                    1,
                ));
            }

            let mut copied_files: Vec<String> = Vec::new();

            let _ = fs::remove_dir_all(consts::SOURCE_OUT_PATH);

            for source in sources {
                let files = files::collect_files_with_extension(&PathBuf::from(source), "java");
                if files.is_empty() {
                    continue;
                }

                for f in &files {
                    let relative_path = f.to_string_lossy().replacen(source, "", 1);
                    let relative_path = relative_path.trim_start_matches(['/', '\\']);
                    let copy_path = format!("{}/{}", consts::SOURCE_OUT_PATH, relative_path);
                    let mut path = PathBuf::from(&copy_path);
                    path.pop();
                    fs::create_dir_all(path).unwrap();
                    File::create(&copy_path).unwrap();

                    match fs::copy(f, &copy_path) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("{e}");
                            continue;
                        }
                    }

                    copied_files.push(copy_path);
                }
            }

            Ok(copied_files)
        }
        None => Err((
            String::from("No source folders given, nothing to compile"),
            1,
        )),
    }
}
