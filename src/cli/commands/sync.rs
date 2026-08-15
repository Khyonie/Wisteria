use std::process::exit;

use crate::cli::args::StartupFlags;
use crate::cli::commands::dependencies::{
    dependency_selection_or_exit, read_lockfile_or_exit, write_full_lockfile_artifacts_or_exit,
    write_partial_lockfile_artifacts_or_exit,
};
use crate::cli::commands::{configuration_or_exit, envvar_regexes, project_or_exit};
use crate::dependency::resolver::{ResolveContext, ResolvedDependency};
use crate::dependency::{Dependency, UpdateContext};
use crate::model::{Lockfile, LockfileArtifact, Metadata, Project};
use crate::output::{self, OutputRenderer};
use regex::Regex;
use std::collections::HashMap;

pub fn trigger_sync(project: Result<Project, String>, args: &[String], flags: &StartupFlags) {
    let project: Project = project_or_exit(project);

    let metadata = match Metadata::load() {
        Ok(m) => m,
        Err(e) => {
            println!("{e}");
            exit(1)
        }
    };

    let configuration = configuration_or_exit(&project, &metadata.configuration);
    let regexes = envvar_regexes();
    let lockfile = read_lockfile_or_exit();
    let selection = dependency_selection_or_exit(&project, args, "sync", true);
    let mut output = output::renderer(flags.output_mode);

    let result = sync_dependencies(
        output.as_mut(),
        selection.names(),
        project.dependencies(),
        configuration.environment(),
        &regexes,
        lockfile.as_ref(),
    );

    if !result.failed.is_empty() {
        output.log("Failed to sync one or more dependencies:");
        for (name, reason) in &result.failed {
            output.log(&format!("\t{name}: {reason}"));
        }
        output.log(
            "Fix: run `wisteria fetch` if a lockfile already exists but cached artifacts are missing, or run `wisteria update all` to resolve and download dependencies."
        );
        exit(1)
    }

    if selection.all_dependencies() {
        write_full_lockfile_artifacts_or_exit(result.artifacts);
    } else {
        write_partial_lockfile_artifacts_or_exit(
            lockfile.as_ref(),
            result.artifacts,
            selection.names(),
        );
    }
}

struct SyncResult {
    artifacts: Vec<LockfileArtifact>,
    failed: Vec<(String, String)>,
}

fn sync_dependencies(
    output: &mut dyn OutputRenderer,
    targets: &[String],
    dependencies: &HashMap<String, Dependency>,
    environment: &HashMap<String, String>,
    regexes: &HashMap<&str, Regex>,
    lockfile: Option<&Lockfile>,
) -> SyncResult {
    let mut artifacts = Vec::new();
    let mut failed = Vec::new();
    let size = targets.len();
    output.operation_started("sync", size);

    for (index, target) in targets.iter().enumerate() {
        let step = index + 1;
        let Some((name, dependency)) = dependencies.get_key_value(target) else {
            output.log(&format!("Usage of undeclared dependency \"{target}\""));
            continue;
        };

        output.step_started("sync", "Resolving", name, step, size);

        match sync_dependency_artifacts(name, dependency, environment, regexes, lockfile) {
            Ok(mut dependency_artifacts) => {
                output.step_completed("sync", "Resolving", name, step, size, "Done");
                artifacts.append(&mut dependency_artifacts);
            }
            Err(error) => {
                output.step_failed("sync", "Resolving", name, step, size, &error);
                failed.push((name.clone(), error));
            }
        }
    }

    if failed.is_empty() {
        output.operation_completed("sync", &sync_summary(targets.len()));
    } else {
        output.operation_completed("sync", "Sync finished with errors.");
    }

    SyncResult { artifacts, failed }
}

fn sync_summary(count: usize) -> String {
    format!("Synced {count} {}", dependency_label(count))
}

fn dependency_label(count: usize) -> &'static str {
    match count {
        1 => "dependency",
        _ => "dependencies",
    }
}

fn sync_dependency_artifacts(
    name: &str,
    dependency: &Dependency,
    environment: &HashMap<String, String>,
    regexes: &HashMap<&str, Regex>,
    lockfile: Option<&Lockfile>,
) -> Result<Vec<LockfileArtifact>, String> {
    if dependency.lockfile_source().is_none() {
        return Ok(Vec::new());
    }

    let matching_artifacts: Vec<&LockfileArtifact> = lockfile
        .map(|lockfile| {
            lockfile
                .artifacts()
                .iter()
                .filter(|artifact| artifact.name() == name)
                .collect()
        })
        .unwrap_or_default();
    let valid_artifacts: Vec<&LockfileArtifact> = matching_artifacts
        .iter()
        .copied()
        .filter(|artifact| dependency.matches_lockfile_artifact(artifact))
        .collect();

    if valid_artifacts.len() == 1 {
        return Ok(vec![valid_artifacts[0].clone()]);
    }

    if valid_artifacts.len() > 1 {
        return Err(format!(
            "`{}` has multiple matching lockfile artifacts for dependency `{name}`.\nFix: run `wisteria update {name}` to resolve the dependency and rewrite a single lock entry.",
            crate::util::consts::LOCKFILE
        ));
    }

    let resolved = dependency.resolve(
        name,
        environment,
        regexes,
        ResolveContext::new(UpdateContext::ResolveOnly),
    )?;

    let artifacts = lockfile_artifacts_from_resolved(resolved);
    if artifacts.is_empty() {
        return Err(format!(
            "Dependency `{name}` is lockable, but resolving it did not produce a lockfile artifact.\nFix: run `wisteria update {name}` to resolve, download, and lock it."
        ));
    }

    Ok(artifacts)
}

fn lockfile_artifacts_from_resolved(resolved: ResolvedDependency) -> Vec<LockfileArtifact> {
    resolved
        .artifacts
        .into_iter()
        .filter_map(|artifact| artifact.lock)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::test_support::{TempDir, with_current_dir};

    const CACHE_PATH: &str = ".wisteria/cache/library/library.jar";
    const FETCH_URL: &str = "https://example.com/library.jar";

    fn regexes() -> HashMap<&'static str, Regex> {
        let mut regexes = HashMap::new();
        regexes.insert("envvars", Regex::new(r#"\{(.+?)}"#).unwrap());
        regexes
    }

    fn environment() -> HashMap<String, String> {
        HashMap::new()
    }

    fn project_from_toml(temp: &TempDir, contents: &str) -> Project {
        let project_file = temp.path().join("project.toml");
        fs::write(&project_file, contents).unwrap();

        Project::from(Some(project_file.to_string_lossy().to_string())).unwrap()
    }

    fn url_project(temp: &TempDir, url: &str) -> Project {
        project_from_toml(
            temp,
            &format!(
                r#"
                [project]
                name = "Demo"
                version = "1.0.0"
                description = "Demo project"

                [dependencies.url]
                library = {{ url = "{url}" }}
                "#
            ),
        )
    }

    fn locked_artifact(name: &str, fetch_url: &str) -> LockfileArtifact {
        LockfileArtifact::new(
            String::from(name),
            String::from("url"),
            None,
            String::from(fetch_url),
            String::from(CACHE_PATH),
            String::from("hash"),
        )
    }

    #[test]
    fn sync_preserves_matching_lockfile_artifact_without_cache() {
        let temp = TempDir::new("sync-preserve-lock");
        let project = url_project(&temp, FETCH_URL);
        let lockfile =
            Lockfile::from_artifacts_for_test(vec![locked_artifact("library", FETCH_URL)]);
        let dependency = project.dependencies().get("library").unwrap();

        let artifacts = sync_dependency_artifacts(
            "library",
            dependency,
            &environment(),
            &regexes(),
            Some(&lockfile),
        )
        .unwrap();

        assert_eq!(artifacts, vec![locked_artifact("library", FETCH_URL)]);
    }

    #[test]
    fn sync_rejects_changed_url_without_cached_artifact() {
        let temp = TempDir::new("sync-changed-url");
        let project = url_project(&temp, "https://example.com/new.jar");
        let lockfile =
            Lockfile::from_artifacts_for_test(vec![locked_artifact("library", FETCH_URL)]);
        let dependency = project.dependencies().get("library").unwrap();

        with_current_dir(temp.path(), || {
            let error = sync_dependency_artifacts(
                "library",
                dependency,
                &environment(),
                &regexes(),
                Some(&lockfile),
            )
            .unwrap_err();

            assert!(error.contains("is not cached"));
            assert!(error.contains("wisteria update library"));
        });
    }

    #[test]
    fn sync_all_drops_stale_lockfile_entries() {
        let temp = TempDir::new("sync-drop-stale");
        let project = url_project(&temp, FETCH_URL);
        let lockfile = Lockfile::from_artifacts_for_test(vec![
            locked_artifact("library", FETCH_URL),
            locked_artifact("stale", FETCH_URL),
        ]);
        let targets: Vec<String> = project.dependencies().keys().cloned().collect();
        let mut output = crate::output::renderer(crate::output::OutputMode::Plain);

        let result = sync_dependencies(
            output.as_mut(),
            &targets,
            project.dependencies(),
            &environment(),
            &regexes(),
            Some(&lockfile),
        );

        assert!(result.failed.is_empty());
        assert_eq!(
            result.artifacts,
            vec![locked_artifact("library", FETCH_URL)]
        );
    }
}
