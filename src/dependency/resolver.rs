use std::{collections::HashMap, path::PathBuf};

use regex::Regex;

use crate::dependency::sources;
use crate::dependency::{Dependency, UpdateContext, UpdatePolicy};
use crate::model::{Lockfile, LockfileArtifact};

impl Dependency {
    pub fn resolve(
        &self,
        name: &str,
        environment: &HashMap<String, String>,
        regexes: &HashMap<&str, Regex>,
        context: ResolveContext<'_>,
    ) -> Result<ResolvedDependency, String> {
        match self {
            Dependency::LocalFile { path, .. } => {
                sources::local::resolve_file(name, path, environment, regexes)
            }
            Dependency::LocalFolder { path, recursive } => {
                sources::local::resolve_folder(name, path, *recursive, environment, regexes)
            }
            Dependency::LocalRepository { .. } => todo!(),
            Dependency::FetchFromUrl {
                url, update_policy, ..
            } => sources::url::resolve(name, url, update_policy, &context),
            Dependency::FetchFromMaven {
                url,
                group_id,
                artifact_id,
                version,
                classifier,
                update_policy,
                ..
            } => sources::maven::resolve(
                sources::maven::MavenResolveRequest {
                    name,
                    url,
                    group_id,
                    artifact_id,
                    version: version.as_ref(),
                    classifier: classifier.as_ref(),
                    update_policy,
                },
                &context,
            ),
            Dependency::FetchFromGithub {
                username,
                repository,
                asset,
                tag,
                release_type,
                update_policy,
                ..
            } => sources::github::resolve(
                sources::github::GithubResolveRequest {
                    name,
                    username,
                    repository,
                    asset,
                    tag: tag.as_ref(),
                    release_type,
                    update_policy,
                },
                &context,
            ),
            Dependency::BuildFromScript { .. } => todo!(),
        }
    }
}

pub struct ResolveContext<'a> {
    update: UpdateContext,
    locked_artifact: Option<&'a LockfileArtifact>,
}

impl<'a> ResolveContext<'a> {
    pub fn new(update: UpdateContext) -> Self {
        Self {
            update,
            locked_artifact: None,
        }
    }

    pub fn with_locked_artifact(
        update: UpdateContext,
        locked_artifact: &'a LockfileArtifact,
    ) -> Self {
        Self {
            update,
            locked_artifact: Some(locked_artifact),
        }
    }

    pub fn for_dependency(
        update: UpdateContext,
        lockfile: Option<&'a Lockfile>,
        dependency_name: &str,
    ) -> Self {
        match lockfile.and_then(|lockfile| lockfile.artifact_for_dependency(dependency_name)) {
            Some(locked_artifact) => Self::with_locked_artifact(update, locked_artifact),
            None => Self::new(update),
        }
    }

    pub fn should_update(&self, update_policy: &UpdatePolicy) -> bool {
        update_policy.should_update(&self.update)
    }

    pub fn locked_artifact(&self) -> Option<&'a LockfileArtifact> {
        self.locked_artifact
    }
}

/// A dependency which has one or more files that exists on disk.
#[derive(Clone, Debug)]
pub struct ResolvedDependency {
    pub name: String,
    pub artifacts: Vec<ResolvedArtifact>,
}

impl ResolvedDependency {
    pub fn new(name: String, artifacts: Vec<ResolvedArtifact>) -> Self {
        Self { name, artifacts }
    }

    pub fn from_paths(name: String, paths: Vec<PathBuf>) -> Self {
        Self {
            name,
            artifacts: paths
                .into_iter()
                .map(|path| ResolvedArtifact::new(path, None))
                .collect(),
        }
    }

    pub fn paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.artifacts.iter().map(|artifact| &artifact.path)
    }
}

/// A single dependency file which exists on disk, and may or may not be present
/// in a lockfile.
#[derive(Clone, Debug)]
pub struct ResolvedArtifact {
    pub path: PathBuf,
    pub lock: Option<LockfileArtifact>,
}

impl ResolvedArtifact {
    pub fn new(path: PathBuf, lock: Option<LockfileArtifact>) -> Self {
        Self { path, lock }
    }
}
