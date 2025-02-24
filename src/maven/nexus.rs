use serde::Deserialize;

//
// XML decoding stuff
//
#[derive(Deserialize)]
pub struct MavenMetadata
{
    versioning: MavenVersionsContainer
}

#[derive(Deserialize)]
struct MavenVersionsContainer
{
    latest: Option<String>,
    release: Option<String>,
    versions: MavenVersionsList
}

#[derive(Deserialize)]
struct MavenVersionsList
{
    version: Vec<String>
}

#[derive(Deserialize)]
pub struct VersionSnapshot
{
    classifier: Option<String>,
    extension: String,
    value: String
}

#[derive(Deserialize)]
pub struct SnapshotMetadata
{
    versioning: SnapshotVersionContainer
}

#[derive(Deserialize)]
struct SnapshotVersionContainer
{
    snapshotVersions: SnapshotVersionList,
}

#[derive(Deserialize)]
struct SnapshotVersionList
{
    snapshotVersion: Vec<SnapshotVersion>
}

#[derive(Deserialize)]
pub struct SnapshotVersion 
{
    classifier: Option<String>,
    extension: String,
    value: String
}

impl MavenMetadata
{
    pub fn latest(&self) -> Option<&String>
    {
        self.versioning.latest.as_ref()
    }

    pub fn release(&self) -> Option<&String>
    {
        self.versioning.release.as_ref()
    }

    pub fn versions(&self) -> &[String]
    {
        self.versioning.versions.version.as_ref()
    }
}

impl SnapshotMetadata
{
    pub fn from_classifier(&self, classifier: Option<&String>) -> Option<String>
    {
        if classifier.is_none()
        {
            // Locate the plain jar
            for snapshot in &self.versioning.snapshotVersions.snapshotVersion
            {
                if snapshot.classifier.is_none() && snapshot.extension == "jar"
                {
                    return Some(snapshot.value.clone())
                }
            }

            return None
        }

        if let Some(classifier) = classifier
        {
            for snapshot in &self.versioning.snapshotVersions.snapshotVersion
            {
                if snapshot.extension != "jar"
                {
                    continue
                }

                if let Some(artifact_classifier) = &snapshot.classifier
                {
                    if classifier == artifact_classifier
                    {
                        return Some(snapshot.value.clone())
                    }
                }
            }
        }
        None
    }
}
