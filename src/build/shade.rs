use std::{fs, fs::File, path::PathBuf};

use zip::ZipArchive;

use crate::util::consts;
use crate::workspace::files;

pub fn shade_jars(shaded_jars: &[PathBuf]) -> Result<(), String> {
    for shaded in shaded_jars {
        let file: File = match File::open(shaded) {
            Ok(f) => f,
            Err(e) => {
                return Err(format!(
                    "Failed to open jar {}: {e}",
                    shaded.to_string_lossy()
                ));
            }
        };

        let mut archive = match ZipArchive::new(file) {
            Ok(a) => a,
            Err(e) => {
                return Err(format!(
                    "Failed to open jar {}: {e}",
                    shaded.to_string_lossy()
                ));
            }
        };

        let shaded_jar_path = PathBuf::from(consts::SHADED_OUT_PATH);
        if let Err(e) = fs::create_dir_all(&shaded_jar_path) {
            return Err(format!("Could not create shaded work folder: {e}"));
        }

        if let Err(e) = archive.extract(consts::SHADED_OUT_PATH) {
            return Err(format!(
                "Could not extract {}: {e}",
                shaded.to_string_lossy()
            ));
        }

        let read = shaded_jar_path.read_dir().map_err(|e| {
            format!(
                "Could not read shaded work folder \"{}\" after extracting \"{}\": {e}",
                shaded_jar_path.display(),
                shaded.display()
            )
        })?;
        for entry in read {
            let entry = entry.map_err(|e| {
                format!(
                    "Could not read an entry in shaded work folder \"{}\" after extracting \"{}\": {e}",
                    shaded_jar_path.display(),
                    shaded.display()
                )
            })?;
            let entry_path = entry.path();

            if entry_path.is_file() {
                fs::remove_file(&entry_path).map_err(|e| {
                    format!(
                        "Could not remove extracted shaded file \"{}\" from jar \"{}\": {e}",
                        entry_path.display(),
                        shaded.display()
                    )
                })?;
                continue;
            }

            if entry.file_name() == "META-INF" {
                continue;
            }

            if files::collect_files_with_extension(&entry_path, "class")
                .map_err(|e| {
                    format!(
                        "Could not inspect extracted shaded path \"{}\" from jar \"{}\": {e}",
                        entry_path.display(),
                        shaded.display()
                    )
                })?
                .is_empty()
            {
                fs::remove_dir_all(&entry_path).map_err(|e| {
                    format!(
                        "Could not remove extracted shaded directory \"{}\" from jar \"{}\": {e}",
                        entry_path.display(),
                        shaded.display()
                    )
                })?;
                continue;
            }
        }
    }

    Ok(())
}
