use reqwest::blocking::Client;
use serde_xml_rs::from_str;

use crate::maven::metadata::{MavenMetadata, SnapshotMetadata};

const MAVEN_METADATA_FILE: &str = "maven-metadata.xml";

pub enum ArtifactVersion {
    Latest,
    Release,
    Version { version: String },
}

pub fn get_artifact(
    client: &Client,
    url: &str,
    group_id: &str,
    artifact_id: &str,
    classifier: Option<&String>,
    version: &ArtifactVersion,
) -> Result<String, String> {
    let mut request_url = String::from(url);
    if !request_url.ends_with("/") {
        request_url.push('/')
    }

    request_url = format!(
        "{request_url}{}/{}/",
        group_id.replace(".", "/"),
        artifact_id.replace(".", "/")
    );
    request_url.push_str(MAVEN_METADATA_FILE);

    let version_text: String = get_text_at_url(&request_url, client)?;
    let metadata: MavenMetadata =
        from_str(&version_text).map_err(|e| format!("Could not decode maven metadata: {e}"))?;

    let target_version = match version {
        ArtifactVersion::Latest => {
            match metadata.latest() {
                Some(v) => v.clone(),
                None => return Err(format!("Artifact {artifact_id} does not specify a latest version, must explicitly specify a version"))
            }
        },
        ArtifactVersion::Release => {
            match metadata.latest() {
                Some(v) => v.clone(),
                None => return Err(format!("Artifact {artifact_id} does not specify a release version, must explicitly specify a version"))
            }
        },
        ArtifactVersion::Version { version } => {
            if !metadata.versions().contains(version)
            {
                return Err(format!("Artifact {artifact_id} does not have a version {version}"))
            }

            version.clone()
        },
    };

    // Check if there we're dealing with a snapshot-based repository
    request_url = String::from(url);
    if !request_url.ends_with("/") {
        request_url.push('/')
    }
    request_url = format!(
        "{request_url}{}/{}/",
        group_id.replace(".", "/"),
        artifact_id.replace(".", "/")
    );
    request_url.push_str(&target_version);
    request_url.push_str("/maven-metadata.xml");

    let response = get_text_at_url(&request_url, client);
    let snapshot_text = match response {
        Ok(s) => s,
        Err(_) => {
            let classifier_postfix = match classifier {
                Some(c) => &format!("-{c}"),
                None => "",
            };

            let mut url_postfix = "";
            if !url.ends_with("/") {
                url_postfix = "/";
            }

            return Ok(format!("{url}{url_postfix}{}/{}/{target_version}/{artifact_id}-{target_version}{classifier_postfix}.jar", group_id.replace(".", "/"), artifact_id.replace(".", "/")));
        }
    };

    let snapshot_metadata: SnapshotMetadata =
        from_str(&snapshot_text).map_err(|e| format!("Could not decode snapshot metadata: {e}"))?;

    let classifier_postfix = match classifier {
        Some(c) => &format!("-{c}"),
        None => "",
    };

    let mut url_postfix = "";
    if !url.ends_with("/") {
        url_postfix = "/";
    }

    let target_value = match snapshot_metadata.take_classifier(classifier, &target_version) {
        Some(v) => v,
        None => {
            return Err(format!(
                "The given classifier is not valid for artifact {artifact_id}-{target_version}"
            ))
        }
    };

    Ok(format!("{url}{url_postfix}{}/{}/{target_version}/{artifact_id}-{target_value}{classifier_postfix}.jar", group_id.replace(".", "/"), artifact_id.replace(".", "/"), ))
}

pub fn get_version(
    client: &Client,
    url: &str,
    group_id: &str,
    artifact_id: &str,
    classifier: Option<&String>,
    version: &ArtifactVersion,
) -> Result<(String, Option<String>), String> {
    let mut request_url = String::from(url);
    if !request_url.ends_with("/") {
        request_url.push('/')
    }

    request_url = format!(
        "{request_url}{}/{}/",
        group_id.replace(".", "/"),
        artifact_id.replace(".", "/")
    );

    request_url.push_str(MAVEN_METADATA_FILE);

    let version_text: String = get_text_at_url(&request_url, client)?;
    let metadata: MavenMetadata =
        from_str(&version_text).map_err(|e| format!("Could not decode maven metadata: {e}"))?;

    let target_version = match version {
        ArtifactVersion::Latest => {
            match metadata.latest() {
                Some(v) => v.clone(),
                None => return Err(format!("Artifact {artifact_id} does not specify a latest version, must explicitly specify a version"))
            }
        },
        ArtifactVersion::Release => {
            match metadata.latest() {
                Some(v) => v.clone(),
                None => return Err(format!("Artifact {artifact_id} does not specify a release version, must explicitly specify a version"))
            }
        },
        ArtifactVersion::Version { version } => {
            if !metadata.versions().contains(version)
            {
                return Err(format!("Artifact {artifact_id} does not have a version {version}"))
            }

            version.clone()
        },
    };

    // Check if there we're dealing with a snapshot-based repository
    request_url = String::from(url);
    if !request_url.ends_with("/") {
        request_url.push('/')
    }
    request_url = format!(
        "{request_url}{}/{}/",
        group_id.replace(".", "/"),
        artifact_id.replace(".", "/")
    );
    request_url.push_str(&target_version);
    request_url.push_str("/maven-metadata.xml");

    let response = get_text_at_url(&request_url, client);
    let snapshot_text = match response {
        Ok(s) => s,
        Err(_) => return Ok((target_version, None)),
    };

    let snapshot_metadata: SnapshotMetadata =
        from_str(&snapshot_text).map_err(|e| format!("Could not decode snapshot metadata: {e}"))?;

    let snapshot_value = snapshot_metadata.take_classifier(classifier, &target_version);
    Ok((target_version, snapshot_value))
}

fn get_text_at_url(url: &str, client: &Client) -> Result<String, String> {
    let response = match client.get(url).send() {
        Ok(r) => r,
        Err(e) => return Err(format!("Could not reach server at URL {url} ({e})")),
    };

    if !response.status().is_success() {
        return Err(format!(
            "Server responded with status {}",
            response.status().as_str()
        ));
    }

    match response.text() {
        Ok(t) => Ok(t),
        Err(e) => Err(format!("Failed to read server response as text: {e}")),
    }
}
