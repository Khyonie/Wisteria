use crate::util::consts;

pub fn url_cache_path(name: &str) -> String {
    format!("{}/{name}/{name}.jar", consts::CACHE_PATH)
}

pub fn maven_cache_path(
    group_id: &str,
    artifact_id: &str,
    version: &str,
    value_postfix: Option<&str>,
    classifier: Option<&String>,
) -> String {
    let value_postfix = value_postfix
        .map(|value| format!("-{value}"))
        .unwrap_or_default();
    let classifier_string = classifier
        .map(|classifier| format!("-{classifier}"))
        .unwrap_or_default();

    format!(
        "{}/{group_id}/{artifact_id}/{version}/{artifact_id}{value_postfix}{classifier_string}.jar",
        consts::CACHE_PATH
    )
}

pub fn github_cache_path(username: &str, repository: &str, tag: &str) -> String {
    format!("{}/{username}/{repository}/{tag}/{repository}.jar", consts::CACHE_PATH)
}
