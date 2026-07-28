use std::path::PathBuf;

use reqwest::blocking::Client;

use crate::dependency::cache;
use crate::dependency::{UpdateContext, UpdatePolicy};
use crate::maven::repository::{self, ArtifactVersion};
use crate::workspace::{download, paths};

pub fn resolve(
    url: &str,
    group_id: &str,
    artifact_id: &str,
    version: Option<&String>,
    classifier: Option<&String>,
    update_policy: &UpdatePolicy,
    update: &UpdateContext,
) -> Result<Vec<PathBuf>, (String, u8)> {
    let target_version = match version {
        Some(version) => match version.as_str() {
            "latest" => ArtifactVersion::Latest,
            "release" => ArtifactVersion::Release,
            _ => ArtifactVersion::Version {
                version: version.to_string(),
            },
        },
        None => ArtifactVersion::Latest,
    };

    let client: Client = Client::builder()
        .user_agent(download::USER_AGENT)
        .build()
        .unwrap();

    let version = match repository::get_version(
        &client,
        url,
        group_id,
        artifact_id,
        classifier,
        &target_version,
    ) {
        Ok(t) => t,
        Err(e) => {
            return Err((format!("Failed to get Maven repository version information: {e}"), 1))
        }
    };

    let filepath = cache::maven_cache_path(
        group_id,
        artifact_id,
        &version.0,
        version.1.as_deref(),
        classifier,
    );
    let path: PathBuf = PathBuf::from(&filepath);

    if update_policy.should_update(update) {
        paths::ensure_parents(&filepath).map_err(|e| (e, 1))?;

        if path.exists() {
            println!("Nothing to do");
            return Ok(vec![path]);
        }

        let target_url = match repository::get_artifact(
            &client,
            url,
            group_id,
            artifact_id,
            classifier,
            &target_version,
        ) {
            Ok(t) => t,
            Err(e) => {
                return Err((format!("Failed to get Maven repository artifact: {e}"), 1))
            }
        };

        download::download(artifact_id.to_string(), target_url, filepath)?;
    } else {
        println!("Not updating");
    }

    Ok(vec![path])
}
