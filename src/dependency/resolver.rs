use std::{collections::HashMap, path::PathBuf};

use regex::Regex;

use crate::dependency::sources;
use crate::dependency::{Dependency, UpdateContext};

impl Dependency {
    pub fn resolve(
        &self,
        name: &str,
        environment: &HashMap<String, String>,
        regexes: &HashMap<&str, Regex>,
        update: UpdateContext,
    ) -> Result<Vec<PathBuf>, (String, u8)> {
        match self {
            Dependency::LocalFile { path, .. } => {
                sources::local::resolve_file(path, environment, regexes)
            }
            Dependency::LocalFolder { path, recursive } => {
                sources::local::resolve_folder(path, *recursive, environment, regexes)
            }
            Dependency::LocalRepository { .. } => todo!(),
            Dependency::FetchFromUrl {
                url, update_policy, ..
            } => sources::url::resolve(name, url, update_policy, &update),
            Dependency::FetchFromMaven {
                url,
                group_id,
                artifact_id,
                version,
                classifier,
                update_policy,
                ..
            } => sources::maven::resolve(
                url,
                group_id,
                artifact_id,
                version.as_ref(),
                classifier.as_ref(),
                update_policy,
                &update,
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
                username,
                repository,
                asset,
                tag.as_ref(),
                release_type,
                update_policy,
                &update,
            ),
            Dependency::BuildFromScript { .. } => todo!(),
        }
    }
}
