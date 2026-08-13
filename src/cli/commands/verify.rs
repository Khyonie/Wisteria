use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    process::exit,
};

use crate::cli::commands::dependencies::require_lockfile_or_exit;
use crate::cli::commands::project_or_exit;
use crate::dependency::Dependency;
use crate::model::{Lockfile, LockfileArtifact, Project};
use crate::util::consts;
use crate::workspace::files;

pub fn trigger_verify(project: Result<Project, String>, args: &[String]) {
    if args.len() > 2 {
        println!(
            "`wisteria verify` checks the whole project and does not accept dependency names.\nFix: run `wisteria verify` without additional arguments."
        );
        exit(1)
    }

    let project: Project = project_or_exit(project);
    let lockfile = require_lockfile_or_exit();
    let issues = verify_project_lockfile(&project, &lockfile);

    if !issues.is_empty() {
        println!("Verification failed:");
        for issue in &issues {
            println!("- {issue}");
        }
        exit(1)
    }

    println!("Operation complete! project.toml, wisteria.lock, and the dependency cache agree.");
}

fn verify_project_lockfile(project: &Project, lockfile: &Lockfile) -> Vec<String> {
    let mut issues = Vec::new();
    let artifacts_by_name = artifacts_by_name(lockfile);

    verify_lockable_project_dependencies(project, &artifacts_by_name, &mut issues);
    verify_locked_artifacts(project.dependencies(), &artifacts_by_name, &mut issues);

    issues
}

fn verify_lockable_project_dependencies(
    project: &Project,
    artifacts_by_name: &BTreeMap<&str, Vec<&LockfileArtifact>>,
    issues: &mut Vec<String>,
) {
    let mut dependency_names: Vec<&str> =
        project.dependencies().keys().map(String::as_str).collect();
    dependency_names.sort_unstable();

    for name in dependency_names {
        let dependency = project.dependencies().get(name).unwrap();
        if dependency.lockfile_source().is_some() && !artifacts_by_name.contains_key(name) {
            issues.push(format!(
                "Dependency `{name}` is lockable but missing from `{}`.\n  Fix: run `wisteria update {name}` to resolve, download, and lock it.",
                consts::LOCKFILE
            ));
        }
    }
}

fn verify_locked_artifacts(
    dependencies: &HashMap<String, Dependency>,
    artifacts_by_name: &BTreeMap<&str, Vec<&LockfileArtifact>>,
    issues: &mut Vec<String>,
) {
    for (name, artifacts) in artifacts_by_name {
        let Some(dependency) = dependencies.get(*name) else {
            issues.push(format!(
                "`{}` contains dependency `{name}`, but project.toml does not declare it.\n  Fix: run `wisteria sync` to remove stale lockfile entries, or add `{name}` back to project.toml.",
                consts::LOCKFILE
            ));
            continue;
        };

        if dependency.lockfile_source().is_none() {
            issues.push(format!(
                "`{}` contains dependency `{name}`, but that dependency type `{}` is not lockable.\n  Fix: run `wisteria sync` to regenerate `{}` from project.toml.",
                consts::LOCKFILE,
                dependency.type_str(),
                consts::LOCKFILE
            ));
            continue;
        }

        if artifacts.len() > 1 {
            issues.push(format!(
                "`{}` contains {} artifacts named `{name}`.\n  Fix: run `wisteria sync {name}` or `wisteria update {name}` to keep only the current artifact.",
                consts::LOCKFILE,
                artifacts.len()
            ));
        }

        for artifact in artifacts {
            verify_artifact(name, dependency, artifact, issues);
        }
    }
}

fn verify_artifact(
    name: &str,
    dependency: &Dependency,
    artifact: &LockfileArtifact,
    issues: &mut Vec<String>,
) {
    if !dependency.matches_lockfile_artifact(artifact) {
        issues.push(format!(
            "Lockfile artifact `{name}` no longer matches the dependency declared in project.toml.\n  Fix: run `wisteria sync {name}` if the matching artifact is already cached, or `wisteria update {name}` to resolve, download, and lock the current dependency."
        ));
    }

    let cache_path = PathBuf::from(artifact.cache_path());
    if !cache_path.exists() {
        issues.push(format!(
            "Locked dependency `{name}` is missing from the cache at `{}`.\n  Fix: run `wisteria fetch {name}` to download the locked artifact.",
            artifact.cache_path()
        ));
        return;
    }

    match files::generate_sha2_for_file(&cache_path) {
        Ok(hash) if hash == artifact.hash() => {}
        Ok(_) => issues.push(format!(
            "Cached dependency `{name}` at `{}` does not match the hash in `{}`.\n  Fix: run `wisteria fetch {name}` to replace the cache from the lockfile, or `wisteria update {name}` if you intended to move to a newer artifact.",
            artifact.cache_path(),
            consts::LOCKFILE
        )),
        Err(error) => issues.push(format!(
            "Could not hash cached dependency `{name}` at `{}`: {error}\n  Fix: repair the cache file or run `wisteria fetch {name}` to replace it.",
            artifact.cache_path()
        )),
    }
}

fn artifacts_by_name(lockfile: &Lockfile) -> BTreeMap<&str, Vec<&LockfileArtifact>> {
    let mut artifacts = BTreeMap::new();
    for artifact in lockfile.artifacts() {
        artifacts
            .entry(artifact.name())
            .or_insert_with(Vec::new)
            .push(artifact);
    }
    artifacts
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::test_support::{TempDir, with_current_dir};

    const CACHE_PATH: &str = ".wisteria/cache/library/library.jar";
    const FETCH_URL: &str = "https://example.com/library.jar";

    fn project_from_toml(temp: &TempDir, contents: &str) -> Project {
        let project_file = temp.path().join("project.toml");
        fs::write(&project_file, contents).unwrap();

        Project::from(Some(project_file.to_string_lossy().to_string())).unwrap()
    }

    fn url_project(temp: &TempDir) -> Project {
        url_project_with_url(temp, "https://example.com/library.jar")
    }

    fn url_project_with_url(temp: &TempDir, url: &str) -> Project {
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

    fn locked_artifact(name: &str, hash: String) -> LockfileArtifact {
        LockfileArtifact::new(
            String::from(name),
            String::from("url"),
            None,
            String::from(FETCH_URL),
            String::from(CACHE_PATH),
            hash,
        )
    }

    #[test]
    fn verify_accepts_matching_lockfile_and_cache() {
        let temp = TempDir::new("verify-matching");

        with_current_dir(temp.path(), || {
            let project = url_project(&temp);
            fs::create_dir_all(".wisteria/cache/library").unwrap();
            fs::write(CACHE_PATH, "cached").unwrap();
            let hash = files::generate_sha2_for_file(&PathBuf::from(CACHE_PATH)).unwrap();
            let lockfile =
                Lockfile::from_artifacts_for_test(vec![locked_artifact("library", hash)]);

            let issues = verify_project_lockfile(&project, &lockfile);

            assert!(issues.is_empty());
        });
    }

    #[test]
    fn verify_rejects_project_dependency_missing_from_lockfile() {
        let temp = TempDir::new("verify-missing-lock");
        let project = url_project(&temp);
        let lockfile = Lockfile::from_artifacts_for_test(vec![]);

        let issues = verify_project_lockfile(&project, &lockfile);

        assert_eq!(issues.len(), 1);
        assert!(issues[0].contains("missing from `wisteria.lock`"));
        assert!(issues[0].contains("wisteria update library"));
    }

    #[test]
    fn verify_rejects_stale_lockfile_dependency() {
        let temp = TempDir::new("verify-stale-lock");
        let project = url_project(&temp);
        let lockfile =
            Lockfile::from_artifacts_for_test(vec![locked_artifact("stale", String::from("hash"))]);

        let issues = verify_project_lockfile(&project, &lockfile);

        assert_eq!(issues.len(), 2);
        assert!(
            issues
                .iter()
                .any(|issue| issue.contains("project.toml does not declare it"))
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.contains("library") && issue.contains("missing"))
        );
    }

    #[test]
    fn verify_rejects_cache_hash_mismatch() {
        let temp = TempDir::new("verify-hash-mismatch");

        with_current_dir(temp.path(), || {
            let project = url_project(&temp);
            fs::create_dir_all(".wisteria/cache/library").unwrap();
            fs::write(CACHE_PATH, "cached").unwrap();
            let lockfile = Lockfile::from_artifacts_for_test(vec![locked_artifact(
                "library",
                String::from("expected hash"),
            )]);

            let issues = verify_project_lockfile(&project, &lockfile);

            assert_eq!(issues.len(), 1);
            assert!(issues[0].contains("does not match the hash"));
            assert!(issues[0].contains("wisteria fetch library"));
        });
    }

    #[test]
    fn verify_rejects_lockfile_artifact_that_no_longer_matches_project_dependency() {
        let temp = TempDir::new("verify-dependency-drift");

        with_current_dir(temp.path(), || {
            let project = url_project_with_url(&temp, "https://example.com/new-library.jar");
            fs::create_dir_all(".wisteria/cache/library").unwrap();
            fs::write(CACHE_PATH, "cached").unwrap();
            let hash = files::generate_sha2_for_file(&PathBuf::from(CACHE_PATH)).unwrap();
            let lockfile =
                Lockfile::from_artifacts_for_test(vec![locked_artifact("library", hash)]);

            let issues = verify_project_lockfile(&project, &lockfile);

            assert_eq!(issues.len(), 1);
            assert!(issues[0].contains("no longer matches"));
            assert!(issues[0].contains("wisteria sync library"));
        });
    }
}
