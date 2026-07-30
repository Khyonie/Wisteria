use serde::Deserialize;

//
// XML decoding stuff
//
#[derive(Deserialize)]
pub struct MavenMetadata {
    versioning: MavenVersionsContainer,
}

#[derive(Deserialize)]
#[allow(unused)]
struct MavenVersionsContainer {
    latest: Option<String>,
    release: Option<String>,
    versions: MavenVersionsList,
}

#[derive(Deserialize)]
struct MavenVersionsList {
    version: Vec<String>,
}

#[derive(Deserialize)]
#[allow(unused)]
pub struct VersionSnapshot {
    classifier: Option<String>,
    extension: String,
    value: String,
}

#[derive(Deserialize)]
pub struct SnapshotMetadata {
    versioning: SnapshotVersionContainer,
}

#[derive(Deserialize)]
struct SnapshotVersionContainer {
    snapshot: Option<SnapshotIdentifier>,
    #[serde(rename = "snapshotVersions")]
    snapshot_versions: Option<SnapshotVersionList>,
}

#[derive(Deserialize)]
struct SnapshotIdentifier {
    timestamp: String,
    #[serde(rename = "buildNumber")]
    build_number: String,
}

#[derive(Deserialize)]
struct SnapshotVersionList {
    #[serde(rename = "snapshotVersion")]
    snapshot_version: Vec<SnapshotVersion>,
}

#[derive(Deserialize)]
pub struct SnapshotVersion {
    classifier: Option<String>,
    extension: String,
    value: String,
}

#[allow(unused)]
impl MavenMetadata {
    pub fn latest(&self) -> Option<&String> {
        self.versioning.latest.as_ref()
    }

    pub fn release(&self) -> Option<&String> {
        self.versioning.release.as_ref()
    }

    pub fn versions(&self) -> &[String] {
        self.versioning.versions.version.as_ref()
    }
}

impl SnapshotMetadata {
    pub fn take_classifier(
        &self,
        classifier: Option<&String>,
        target_version: &str,
    ) -> Option<String> {
        if let Some(snapshot_versions) = &self.versioning.snapshot_versions {
            if classifier.is_none() {
                // Locate the plain jar
                for snapshot in &snapshot_versions.snapshot_version {
                    if snapshot.classifier.is_none() && snapshot.extension == "jar" {
                        return Some(snapshot.value.clone());
                    }
                }

                return None;
            }

            if let Some(classifier) = classifier {
                for snapshot in &snapshot_versions.snapshot_version {
                    if snapshot.extension != "jar" {
                        continue;
                    }

                    if let Some(artifact_classifier) = &snapshot.classifier
                        && classifier == artifact_classifier
                    {
                        return Some(snapshot.value.clone());
                    }
                }
            }

            return None;
        }

        self.timestamped_snapshot_value(target_version)
    }

    fn timestamped_snapshot_value(&self, target_version: &str) -> Option<String> {
        let snapshot = self.versioning.snapshot.as_ref()?;
        let base_version = target_version
            .strip_suffix("-SNAPSHOT")
            .unwrap_or(target_version);

        Some(format!(
            "{base_version}-{}-{}",
            snapshot.timestamp, snapshot.build_number
        ))
    }
}

#[cfg(test)]
mod tests {
    use serde_xml_rs::from_str;

    use super::SnapshotMetadata;

    #[test]
    fn reads_snapshot_version_entries() {
        let metadata: SnapshotMetadata = from_str(
            r#"
            <metadata>
              <versioning>
                <snapshotVersions>
                  <snapshotVersion>
                    <extension>jar</extension>
                    <value>1.0-20260709.120000-1</value>
                  </snapshotVersion>
                  <snapshotVersion>
                    <classifier>shaded</classifier>
                    <extension>jar</extension>
                    <value>1.0-20260709.120000-1</value>
                  </snapshotVersion>
                </snapshotVersions>
              </versioning>
            </metadata>
            "#,
        )
        .unwrap();

        assert_eq!(
            metadata
                .take_classifier(Some(&String::from("shaded")), "1.0-SNAPSHOT")
                .as_deref(),
            Some("1.0-20260709.120000-1")
        );
    }

    #[test]
    fn reads_plain_jar_snapshot_version_when_classifier_is_absent() {
        let metadata: SnapshotMetadata = from_str(
            r#"
            <metadata>
              <versioning>
                <snapshotVersions>
                  <snapshotVersion>
                    <classifier>sources</classifier>
                    <extension>jar</extension>
                    <value>1.0-20260709.120000-1</value>
                  </snapshotVersion>
                  <snapshotVersion>
                    <extension>jar</extension>
                    <value>1.0-20260709.120000-2</value>
                  </snapshotVersion>
                </snapshotVersions>
              </versioning>
            </metadata>
            "#,
        )
        .unwrap();

        assert_eq!(
            metadata.take_classifier(None, "1.0-SNAPSHOT").as_deref(),
            Some("1.0-20260709.120000-2")
        );
    }

    #[test]
    fn reads_maven_metadata_versions() {
        let metadata: super::MavenMetadata = from_str(
            r#"
            <metadata>
              <versioning>
                <latest>2.0.0</latest>
                <release>1.9.0</release>
                <versions>
                  <version>1.0.0</version>
                  <version>1.9.0</version>
                  <version>2.0.0</version>
                </versions>
              </versioning>
            </metadata>
            "#,
        )
        .unwrap();

        assert_eq!(metadata.latest().map(String::as_str), Some("2.0.0"));
        assert_eq!(metadata.release().map(String::as_str), Some("1.9.0"));
        assert_eq!(
            metadata.versions(),
            &[
                String::from("1.0.0"),
                String::from("1.9.0"),
                String::from("2.0.0"),
            ]
        );
    }

    #[test]
    fn derives_snapshot_value_without_snapshot_versions() {
        let metadata: SnapshotMetadata = from_str(
            r#"
            <metadata>
              <versioning>
                <snapshot>
                  <timestamp>20260709.120000</timestamp>
                  <buildNumber>1</buildNumber>
                </snapshot>
              </versioning>
            </metadata>
            "#,
        )
        .unwrap();

        assert_eq!(
            metadata
                .take_classifier(Some(&String::from("shaded")), "1.0-SNAPSHOT")
                .as_deref(),
            Some("1.0-20260709.120000-1")
        );
    }
}
