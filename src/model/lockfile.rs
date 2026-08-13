use std::{fs, io::ErrorKind};

use serde::{Deserialize, Serialize};
use toml::Value;

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

/// Attempts to read wisteria.lock in the current directory. If the file does not exist, Ok(None)
/// will be returned.
pub fn try_read_lockfile() -> Result<Option<Lockfile>, String> {
    let lockfile_str = match fs::read_to_string(consts::LOCKFILE) {
        Ok(s) => s,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("Failed to read lockfile: {e}")),
    };

    let lockfile_value: Value =
        toml::from_str(&lockfile_str).map_err(|e| format!("Failed to parse lockfile: {e}"))?;

    let schema = read_schema(&lockfile_value)?;
    validate_schema(schema)?;

    let lockfile: Lockfile = lockfile_value
        .try_into()
        .map_err(|e| format!("Failed to parse lockfile: {e}"))?;

    Ok(Some(lockfile))
}

/// Takes resolved dependencies and generates a TOML-serialized lockfile for it.
/// Only dependencies which can be put into a lockfile will be stored.
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

/// Attempts to write a lockfile TOML string to wisteria.lock.
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

/// Ensures a serialized lockfile is valid TOML before writing it.
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

fn read_schema(lockfile: &Value) -> Result<i64, String> {
    let Some(schema) = lockfile.get("schema") else {
        return Err(format!(
            "Missing wisteria.lock schema.\nFix: delete `{}` and run `wisteria update` again, or restore it from source control.",
            consts::LOCKFILE
        ));
    };

    schema.as_integer().ok_or_else(|| {
        format!(
            "Invalid wisteria.lock schema; expected an integer, found {}.\nFix: delete `{}` and run `wisteria update` again, or restore it from source control.",
            schema.type_str(),
            consts::LOCKFILE
        )
    })
}

/// Validates that a read lockfile uses a supported schema.
fn validate_schema(schema: i64) -> Result<(), String> {
    let current_schema = i64::from(LOCKFILE_SCHEMA_VERSION);

    match schema {
        ..=0 => Err(format!(
            "Invalid wisteria.lock schema {schema}; schema versions start at 1.\nFix: delete `{}` and run `wisteria update` again, or restore it from source control.",
            consts::LOCKFILE
        )),
        schema if schema == current_schema => Ok(()),
        schema if schema < current_schema => Err(format!(
            "Unsupported wisteria.lock schema {schema}; this Wisteria version does not know how to migrate it to schema {LOCKFILE_SCHEMA_VERSION}.\nFix: delete `{}` and run `wisteria update` again, or restore it from source control.",
            consts::LOCKFILE
        )),
        schema => Err(format!(
            "Unsupported wisteria.lock schema {schema}; this Wisteria version only supports schema {LOCKFILE_SCHEMA_VERSION}.\nFix: update Wisteria, or restore a lockfile generated by a compatible version.",
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use crate::{
        dependency::resolver::{ResolvedArtifact, ResolvedDependency},
        test_support::{TempDir, with_current_dir},
        util::consts,
    };

    use super::*;

    fn gson_artifact() -> LockfileArtifact {
        LockfileArtifact {
            name: String::from("gson"),
            source: String::from("maven"),
            version: String::from("2.14.0"),
            fetch_url: String::from("https://example/gson.jar"),
            cache_path: String::from(".wisteria/cache/com.google.code.gson/gson.jar"),
            hash: String::from("foo"),
        }
    }

    fn anenome_artifact() -> LockfileArtifact {
        LockfileArtifact {
            name: String::from("anenome"),
            source: String::from("github"),
            version: String::from("2.0.0"),
            fetch_url: String::from("https://github.com/Khyonie/Anenome.jar"),
            cache_path: String::from(".wisteria/cache/Khyonie/Anenome/Anenome.jar"),
            hash: String::from("bar"),
        }
    }

    #[test]
    fn read_lockfile_returns_none_when_missing() {
        let temp = TempDir::new("lockfile-missing");

        with_current_dir(temp.path(), || {
            let lockfile = try_read_lockfile().unwrap();

            assert_eq!(lockfile, None);
            assert!(!PathBuf::from(consts::LOCKFILE).exists());
            assert!(!PathBuf::from(consts::LOCKFILE_TEMP).exists());
        });
    }

    #[test]
    fn read_lockfile_parses_existing_lockfile() {
        let temp = TempDir::new("lockfile-read");

        with_current_dir(temp.path(), || {
            fs::write(
                consts::LOCKFILE,
                r#"
schema = 1

[[artifact]]
name = "gson"
source = "maven"
version = "2.14.0"
fetch_url = "https://example/gson.jar"
cache_path = ".wisteria/cache/com.google.code.gson/gson.jar"
hash = "foo"
"#,
            )
            .unwrap();

            let lockfile = try_read_lockfile().unwrap().unwrap();

            assert_eq!(lockfile.schema, LOCKFILE_SCHEMA_VERSION);
            assert_eq!(lockfile.artifact, vec![gson_artifact()]);
        });
    }

    #[test]
    fn read_lockfile_rejects_invalid_toml() {
        let temp = TempDir::new("lockfile-invalid");

        with_current_dir(temp.path(), || {
            fs::write(consts::LOCKFILE, "schema = [").unwrap();

            let error = try_read_lockfile().unwrap_err();

            assert!(error.contains("Failed to parse lockfile"));
        });
    }

    #[test]
    fn read_lockfile_rejects_missing_schema() {
        let temp = TempDir::new("lockfile-missing-schema");

        with_current_dir(temp.path(), || {
            fs::write(
                consts::LOCKFILE,
                r#"
[[artifact]]
name = "gson"
source = "maven"
version = "2.14.0"
fetch_url = "https://example/gson.jar"
cache_path = ".wisteria/cache/com.google.code.gson/gson.jar"
hash = "foo"
"#,
            )
            .unwrap();

            let error = try_read_lockfile().unwrap_err();

            assert!(error.contains("Missing wisteria.lock schema"));
            assert!(error.contains("wisteria update"));
        });
    }

    #[test]
    fn read_lockfile_rejects_non_integer_schema() {
        let temp = TempDir::new("lockfile-non-integer-schema");

        with_current_dir(temp.path(), || {
            fs::write(consts::LOCKFILE, r#"schema = "1""#).unwrap();

            let error = try_read_lockfile().unwrap_err();

            assert!(error.contains("expected an integer"));
            assert!(error.contains("found string"));
        });
    }

    #[test]
    fn read_lockfile_rejects_schema_zero() {
        let temp = TempDir::new("lockfile-zero-schema");

        with_current_dir(temp.path(), || {
            fs::write(consts::LOCKFILE, "schema = 0").unwrap();

            let error = try_read_lockfile().unwrap_err();

            assert!(error.contains("Invalid wisteria.lock schema 0"));
            assert!(error.contains("schema versions start at 1"));
        });
    }

    #[test]
    fn read_lockfile_rejects_newer_schema() {
        let temp = TempDir::new("lockfile-newer-schema");

        with_current_dir(temp.path(), || {
            fs::write(consts::LOCKFILE, "schema = 2").unwrap();

            let error = try_read_lockfile().unwrap_err();

            assert!(error.contains("Unsupported wisteria.lock schema 2"));
            assert!(error.contains("update Wisteria"));
        });
    }

    #[test]
    fn lockable_artifacts_to_toml_serializes_schema_and_lockable_artifacts() {
        let lockable_artifacts = vec![gson_artifact(), anenome_artifact()];
        let dependencies = vec![
            ResolvedDependency {
                name: String::from("lockable"),
                artifacts: vec![
                    ResolvedArtifact {
                        path: PathBuf::from(".wisteria/cache/gson.jar"),
                        lock: Some(lockable_artifacts[0].clone()),
                    },
                    ResolvedArtifact {
                        path: PathBuf::from("lib/local.jar"),
                        lock: None,
                    },
                ],
            },
            ResolvedDependency {
                name: String::from("also-lockable"),
                artifacts: vec![ResolvedArtifact {
                    path: PathBuf::from(".wisteria/cache/anenome.jar"),
                    lock: Some(lockable_artifacts[1].clone()),
                }],
            },
        ];

        let toml = lockable_artifacts_to_toml(&dependencies).unwrap();
        let parsed: Lockfile = toml::from_str(&toml).unwrap();

        assert_eq!(parsed.schema, LOCKFILE_SCHEMA_VERSION);
        assert_eq!(parsed.artifact, lockable_artifacts);
        assert!(toml.contains("schema = 1"));
        assert!(toml.contains("[[artifact]]"));
    }

    #[test]
    fn lockfile_serialization_makes_round_trip() {
        let lockable_artifacts = vec![gson_artifact(), anenome_artifact()];

        let lockfile = Lockfile {
            schema: LOCKFILE_SCHEMA_VERSION,
            artifact: lockable_artifacts.clone(),
        };
        let toml = toml::to_string_pretty(&lockfile).unwrap();

        let parsed: Lockfile = toml::from_str(&toml).unwrap();

        assert_eq!(parsed.artifact, lockable_artifacts)
    }

    #[test]
    fn write_lockfile_replaces_existing_lockfile_and_removes_temp_file() {
        let temp = TempDir::new("lockfile-write");

        with_current_dir(temp.path(), || {
            fs::write(consts::LOCKFILE, "old lockfile").unwrap();

            write_lockfile("schema = 1\n").unwrap();

            assert_eq!(
                fs::read_to_string(consts::LOCKFILE).unwrap(),
                "schema = 1\n"
            );
            assert!(!PathBuf::from(consts::LOCKFILE_TEMP).exists());
        });
    }
}
