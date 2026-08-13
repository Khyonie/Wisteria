use std::{fs, io::ErrorKind};

use serde::{Deserialize, Serialize};

use crate::{dependency::resolver::ResolvedDependency, util::consts};

pub const LOCKFILE_SCHEMA_VERSION: u16 = 1;

#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub struct Lockfile {
    schema: u16,
    artifact: Vec<LockfileArtifact>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
pub struct LockfileArtifact {
    name: String,
    source: String,
    version: String,
    fetch_url: String,
    cache_path: String,
    hash: String,
}

pub fn read_lockfile() -> Result<Option<Lockfile>, String> {
    let lockfile_str = match fs::read_to_string(consts::LOCKFILE) {
        Ok(s) => s,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("Failed to read lockfile: {e}")),
    };

    let lockfile =
        toml::from_str(&lockfile_str).map_err(|e| format!("Failed to parse lockfile: {e}"))?;

    Ok(Some(lockfile))
}

pub fn lockable_artifacts_to_toml(dependencies: &[ResolvedDependency]) -> Result<String, String> {
    let locked_artifacts: Vec<LockfileArtifact> = dependencies
        .iter()
        .flat_map(|d| {
            d.artifacts
                .iter()
                .filter_map(|path| path.lock.as_ref())
                .cloned()
        })
        .collect();

    let lockfile = Lockfile {
        schema: LOCKFILE_SCHEMA_VERSION,
        artifact: locked_artifacts,
    };

    let toml = toml::to_string_pretty(&lockfile)
        .map_err(|e| format!("Failed to serialize lockfile TOML: {e}"))?;

    verify_lockfile(&toml, &lockfile)?;

    Ok(toml)
}

pub fn write_lockfile(toml: &str) -> Result<(), String> {
    // The lockfile will basically track what's in the cache, and right now we don't have a good
    // way to rebuild it. So we want to be careful not to overwrite the lockfile until we know
    // we can write a good one.
    fs::write(consts::LOCKFILE_TEMP, toml)
        .map_err(|e| format!("Failed to write temporary lockfile, the current wisteria.lock has not been modified: {e}"))?;

    match fs::rename(consts::LOCKFILE_TEMP, consts::LOCKFILE) {
        Ok(()) => Ok(()),
        Err(rename_error) => {
            let cleanup_note = match fs::remove_file(consts::LOCKFILE_TEMP) {
                Ok(()) => String::new(),
                Err(cleanup_error) => format!(
                    "\nAlso failed to remove temporary lockfile `{}`, please remove it manually: {cleanup_error}",
                    consts::LOCKFILE_TEMP
                ),
            };

            Err(format!(
                "Failed to replace `{}` with `{}`, the current wisteria.lock may not have been modified: {rename_error}{cleanup_note}",
                consts::LOCKFILE,
                consts::LOCKFILE_TEMP
            ))
        }
    }
}

fn verify_lockfile(toml: &str, lockfile: &Lockfile) -> Result<(), String> {
    let deserialized_lockfile: Lockfile = toml::from_str(toml)
        .map_err(|e| format!("Generated lockfile failed serialization, the current wisteria.lock has not been modified: {e}"))?;

    if deserialized_lockfile != *lockfile {
        return Err(String::from(
            "Generated lockfile does not match expected output, the current wisteria.lock has not been modified",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::model::{Lockfile, LockfileArtifact, lockfile::LOCKFILE_SCHEMA_VERSION};

    #[test]
    fn lockfile_serialization_makes_round_trip() {
        let lockable_artifacts = vec![
            LockfileArtifact {
                name: String::from("gson"),
                source: String::from("maven"),
                version: String::from("2.14.0"),
                fetch_url: String::from("https://example/gson.jar"),
                cache_path: String::from(".wisteria/cache/com.google.code.gson/gson.jar"),
                hash: String::from("foo"),
            },
            LockfileArtifact {
                name: String::from("anenome"),
                source: String::from("github"),
                version: String::from("2.0.0"),
                fetch_url: String::from("https://github.com/Khyonie/Anenome.jar"),
                cache_path: String::from(".wisteria/cache/Khyonie/Anenome/Anenome.jar"),
                hash: String::from("bar"),
            },
        ];

        let lockfile = Lockfile {
            schema: LOCKFILE_SCHEMA_VERSION,
            artifact: lockable_artifacts.clone(),
        };
        let toml = toml::to_string_pretty(&lockfile).unwrap();

        let parsed: Lockfile = toml::from_str(&toml).unwrap();

        assert_eq!(parsed.artifact, lockable_artifacts)
    }
}
