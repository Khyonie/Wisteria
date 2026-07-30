use std::{fs, fs::File, path::PathBuf};

use zip::ZipArchive;

use crate::util::consts;
use crate::workspace::files;

pub fn shade_jars(shaded_jars: &[PathBuf]) -> Result<(), (String, u8)> {
    for shaded in shaded_jars {
        let file: File = match File::open(shaded) {
            Ok(f) => f,
            Err(e) => {
                return Err((
                    format!("Failed to open jar {}: {e}", shaded.to_string_lossy()),
                    1,
                ));
            }
        };

        let mut archive = match ZipArchive::new(file) {
            Ok(a) => a,
            Err(e) => {
                return Err((
                    format!("Failed to open jar {}: {e}", shaded.to_string_lossy()),
                    1,
                ));
            }
        };

        let shaded_jar_path = PathBuf::from(consts::SHADED_OUT_PATH);
        if !shaded_jar_path.exists() {
            if let Err(e) = fs::create_dir_all(&shaded_jar_path) {
                return Err((format!("Could not create shaded work folder: {e}"), 1));
            }
        }

        if let Err(e) = archive.extract(consts::SHADED_OUT_PATH) {
            return Err((
                format!("Could not extract {}: {e}", shaded.to_string_lossy()),
                1,
            ));
        }

        let read = shaded_jar_path.read_dir().unwrap();
        for entry in read.flatten() {
            if entry.path().is_file() {
                fs::remove_file(entry.path()).unwrap();
                continue;
            }

            if entry.file_name() == "META-INF" {
                continue;
            }

            if files::collect_files_with_extension(&entry.path(), "class").is_empty() {
                fs::remove_dir_all(entry.path()).unwrap();
                continue;
            }
        }
    }

    Ok(())
}
