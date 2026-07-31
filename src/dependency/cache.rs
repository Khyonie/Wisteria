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
    format!(
        "{}/{username}/{repository}/{tag}/{repository}.jar",
        consts::CACHE_PATH
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_url_cache_path_from_dependency_name() {
        assert_eq!(
            url_cache_path("library"),
            ".wisteria/cache/library/library.jar"
        );
    }

    #[test]
    fn builds_maven_cache_path_without_optional_suffixes() {
        assert_eq!(
            maven_cache_path("com.example", "library", "1.0.0", None, None),
            ".wisteria/cache/com.example/library/1.0.0/library.jar"
        );
    }

    #[test]
    fn builds_maven_cache_path_with_snapshot_value_and_classifier() {
        assert_eq!(
            maven_cache_path(
                "com.example",
                "library",
                "1.0-SNAPSHOT",
                Some("1.0-20260721.100000-1"),
                Some(&String::from("shaded")),
            ),
            ".wisteria/cache/com.example/library/1.0-SNAPSHOT/library-1.0-20260721.100000-1-shaded.jar"
        );
    }

    #[test]
    fn builds_github_cache_path() {
        assert_eq!(
            github_cache_path("Owner", "Repository", "v1.2.3"),
            ".wisteria/cache/Owner/Repository/v1.2.3/Repository.jar"
        );
    }
}
