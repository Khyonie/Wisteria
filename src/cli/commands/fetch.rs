use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::exit,
};

use crate::cli::args::StartupFlags;
use crate::cli::commands::dependencies::{duplicate_dependency_name, require_lockfile_or_exit};
use crate::cli::commands::project_or_exit;
use crate::model::{Lockfile, LockfileArtifact, Project};
use crate::output;
use crate::util::consts;
use crate::workspace::{download, files, paths};

pub fn trigger_fetch(project: Result<Project, String>, args: &[String], flags: &StartupFlags) {
    let _project: Project = project_or_exit(project);
    let lockfile = require_lockfile_or_exit();
    let artifacts = lockfile_artifacts_or_exit(&lockfile, args);

    let mut failed_fetches = Vec::new();
    let size = artifacts.len();
    let mut output = output::renderer(flags.output_mode);
    output.operation_started("fetch", size);

    for (index, artifact) in artifacts.iter().enumerate() {
        let step = index + 1;
        output.step_started("fetch", "Fetching", artifact.name(), step, size);

        match fetch_lockfile_artifact(artifact) {
            Ok(status) => {
                output.step_completed("fetch", "Fetching", artifact.name(), step, size, &status)
            }
            Err(error) => {
                output.step_failed("fetch", "Fetching", artifact.name(), step, size, &error);
                failed_fetches.push((artifact.name().to_string(), error));
            }
        }
    }

    if failed_fetches.is_empty() {
        output.operation_completed("fetch", &fetch_summary(artifacts.len()));
    } else {
        output.operation_completed("fetch", "Fetch finished with errors.");
    }

    if !failed_fetches.is_empty() {
        output.log("Failed to fetch one or more dependencies:");
        for (name, reason) in &failed_fetches {
            output.log(&format!("\t{name}: {reason}"));
        }
        exit(1)
    }
}

fn fetch_summary(count: usize) -> String {
    format!("Fetched {count} {}", dependency_label(count))
}

fn dependency_label(count: usize) -> &'static str {
    match count {
        1 => "dependency",
        _ => "dependencies",
    }
}

fn lockfile_artifacts_or_exit<'a>(
    lockfile: &'a Lockfile,
    args: &[String],
) -> Vec<&'a LockfileArtifact> {
    match select_lockfile_artifacts(lockfile, args) {
        Ok(artifacts) => artifacts,
        Err(error) => {
            println!("{error}");
            exit(1)
        }
    }
}

fn select_lockfile_artifacts<'a>(
    lockfile: &'a Lockfile,
    args: &[String],
) -> Result<Vec<&'a LockfileArtifact>, String> {
    if args.len() == 2 {
        return Ok(lockfile.artifacts().iter().collect());
    }

    if args[2] == "all" {
        if args.len() > 3 {
            return Err(format!(
                "Invalid arguments: `all` cannot be combined with explicit dependency names.\nFix: run either `wisteria fetch all` or `wisteria fetch {}`.",
                args[3..].join(" ")
            ));
        }

        return Ok(lockfile.artifacts().iter().collect());
    }

    if let Some(duplicate) = duplicate_dependency_name(&args[2..]) {
        return Err(format!(
            "Dependency `{duplicate}` was listed more than once.\nFix: list each dependency at most once, or run `wisteria fetch all`."
        ));
    }

    let mut artifacts = Vec::new();
    for target in &args[2..] {
        let matching_artifacts: Vec<&LockfileArtifact> = lockfile
            .artifacts()
            .iter()
            .filter(|artifact| artifact.name() == target)
            .collect();

        if matching_artifacts.is_empty() {
            return Err(format!(
                "No dependency named \"{target}\" exists in wisteria.lock.\n{}\nFix: run `wisteria sync` to regenerate the lockfile, or use one of the locked dependency names above.",
                valid_lockfile_dependencies_message(lockfile)
            ));
        }

        artifacts.extend(matching_artifacts);
    }

    Ok(artifacts)
}

fn valid_lockfile_dependencies_message(lockfile: &Lockfile) -> String {
    let names: BTreeSet<&str> = lockfile
        .artifacts()
        .iter()
        .map(LockfileArtifact::name)
        .collect();

    if names.is_empty() {
        return String::from("The lockfile does not contain any dependency artifacts.");
    }

    let mut message = String::from("Locked dependencies:");
    for name in names {
        message.push_str(&format!("\n- {name}"));
    }
    message
}

fn fetch_lockfile_artifact(artifact: &LockfileArtifact) -> Result<String, String> {
    let cache_path = PathBuf::from(artifact.cache_path());
    let replacing_existing = cache_path.exists();
    if replacing_existing {
        let hash = files::generate_sha2_for_file(&cache_path)?;
        if hash == artifact.hash() {
            return Ok(String::from("Already cached"));
        }
    }

    let temp_path = temporary_artifact_path(artifact.cache_path());
    paths::ensure_parents(artifact.cache_path())?;
    paths::ensure_parents(path_to_str(&temp_path)?)?;

    let size = match download::download_silent(
        artifact.name().to_string(),
        artifact.fetch_url().to_string(),
        temp_path.to_string_lossy().to_string(),
    ) {
        Ok(size) => size,
        Err(error) => {
            let cleanup_note = cleanup_temp_artifact(&temp_path);
            return Err(format!("{error}{cleanup_note}"));
        }
    };

    let hash = match files::generate_sha2_for_file(&temp_path) {
        Ok(hash) => hash,
        Err(error) => {
            let cleanup_note = cleanup_temp_artifact(&temp_path);
            return Err(format!("{error}{cleanup_note}"));
        }
    };

    if hash != artifact.hash() {
        let cleanup_note = cleanup_temp_artifact(&temp_path);
        return Err(format!(
            "Downloaded artifact from `{}` does not match the hash in `{}`; the existing cached artifact was not replaced.{cleanup_note}",
            artifact.fetch_url(),
            consts::LOCKFILE
        ));
    }

    fs::rename(&temp_path, &cache_path).map_err(|e| {
        let cleanup_note = cleanup_temp_artifact(&temp_path);
        format!(
            "Downloaded artifact matched the lockfile, but failed to replace `{}`: {e}{cleanup_note}",
            artifact.cache_path()
        )
    })?;

    if replacing_existing {
        Ok(format!("Re-fetched {:.3} MB", size))
    } else {
        Ok(format!("Fetched {:.3} MB", size))
    }
}

fn temporary_artifact_path(cache_path: &str) -> PathBuf {
    PathBuf::from(format!("{cache_path}.{}", consts::TEMP_FILE_EXTENSION))
}

fn path_to_str(path: &Path) -> Result<&str, String> {
    path.to_str()
        .ok_or_else(|| format!("Path `{}` is not valid UTF-8", path.display()))
}

fn cleanup_temp_artifact(temp_path: &Path) -> String {
    match fs::remove_file(temp_path) {
        Ok(()) => String::new(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => format!(
            "\nAlso failed to remove temporary download `{}`; please remove it manually: {e}",
            temp_path.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TempDir, with_current_dir};

    fn locked_artifact() -> LockfileArtifact {
        LockfileArtifact::new(
            String::from("library"),
            String::from("url"),
            None,
            String::from("https://example.com/library.jar"),
            String::from(".wisteria/cache/library/library.jar"),
            String::from("3673014e72b67383be302485694555a57ad393afdebaed6ded110a775bd0556d"),
        )
    }

    #[test]
    fn fetch_lockfile_artifact_skips_matching_cached_artifact() {
        let temp = TempDir::new("fetch-cached");

        with_current_dir(temp.path(), || {
            let artifact = locked_artifact();
            fs::create_dir_all(".wisteria/cache/library").unwrap();
            fs::write(artifact.cache_path(), "cached").unwrap();

            fetch_lockfile_artifact(&artifact).unwrap();

            assert_eq!(fs::read_to_string(artifact.cache_path()).unwrap(), "cached");
            assert!(!temporary_artifact_path(artifact.cache_path()).exists());
        });
    }

    #[test]
    fn select_lockfile_artifacts_selects_all_by_default() {
        let artifact = locked_artifact();
        let lockfile = Lockfile::from_artifacts_for_test(vec![artifact]);
        let args = vec![String::from("wisteria"), String::from("fetch")];

        let selected = select_lockfile_artifacts(&lockfile, &args).unwrap();

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].name(), "library");
    }

    #[test]
    fn select_lockfile_artifacts_rejects_unknown_dependency() {
        let lockfile = Lockfile::from_artifacts_for_test(vec![locked_artifact()]);
        let args = vec![
            String::from("wisteria"),
            String::from("fetch"),
            String::from("missing"),
        ];

        let error = select_lockfile_artifacts(&lockfile, &args).unwrap_err();

        assert!(error.contains("No dependency named \"missing\""));
        assert!(error.contains("Locked dependencies"));
    }

    #[test]
    fn select_lockfile_artifacts_rejects_duplicate_dependency() {
        let lockfile = Lockfile::from_artifacts_for_test(vec![locked_artifact()]);
        let args = vec![
            String::from("wisteria"),
            String::from("fetch"),
            String::from("library"),
            String::from("library"),
        ];

        let error = select_lockfile_artifacts(&lockfile, &args).unwrap_err();

        assert!(error.contains("listed more than once"));
        assert!(error.contains("wisteria fetch all"));
    }
}
