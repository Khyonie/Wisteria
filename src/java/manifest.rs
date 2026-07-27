#[allow(clippy::new_without_default)]
pub struct Manifest {
    entries: Vec<ManifestEntry>,
}

pub enum ManifestEntry {
    Version { version: String },
    CreatedBy { signature: String },
    MainClass { class: String },
    ClassPath { path: Vec<String> },
}

impl Manifest {
    pub fn new() -> Self {
        let entries: Vec<ManifestEntry> = vec![ManifestEntry::Version {
            version: String::from("1.0"),
        }];

        Self { entries }
    }

    pub fn add_entry(&mut self, entry: ManifestEntry) {
        self.entries.push(entry);
    }

    pub fn to_file(&self) -> String {
        let mut manifest: String = String::new();

        for entry in self.entries.iter() {
            manifest.push_str(&entry.to_header());
        }

        manifest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_manifest_contains_version_header() {
        assert_eq!(Manifest::new().to_file(), "Manifest-Version: 1.0\n");
    }

    #[test]
    fn manifest_writes_entries_in_insertion_order() {
        let mut manifest = Manifest::new();
        manifest.add_entry(ManifestEntry::CreatedBy {
            signature: String::from("Wisteria"),
        });
        manifest.add_entry(ManifestEntry::MainClass {
            class: String::from("com.example.Main"),
        });

        assert_eq!(
            manifest.to_file(),
            "Manifest-Version: 1.0\nCreated-By: Wisteria\nMain-Class: com.example.Main\n"
        );
    }

    #[test]
    fn classpath_entry_joins_paths() {
        assert_eq!(
            ManifestEntry::ClassPath {
                path: vec![String::from("lib/a.jar"), String::from("lib/b.jar")]
            }
            .to_header(),
            "Class-Path: lib/a.jar lib/b.jar\n"
        );
    }

    #[test]
    fn classpath_entry_wraps_long_lines_with_continuation_space() {
        let long_path = format!("lib/{}.jar", "a".repeat(80));
        let header = ManifestEntry::ClassPath {
            path: vec![long_path],
        }
        .to_header();

        let lines: Vec<&str> = header.lines().collect();

        assert!(lines.len() > 1);
        assert_eq!(lines[0].len(), 71);
        assert!(lines[1].starts_with(' '));
    }
}

impl ManifestEntry {
    pub fn to_header(&self) -> String {
        match self {
            ManifestEntry::Version { version } => format!("Manifest-Version: {version}\n"),
            ManifestEntry::CreatedBy { signature } => format!("Created-By: {signature}\n"),
            ManifestEntry::MainClass { class } => format!("Main-Class: {class}\n"),
            ManifestEntry::ClassPath { path } => {
                let mut attribute_raw: String = String::from("Class-Path: ");

                for s in path {
                    attribute_raw.push_str(s);
                    attribute_raw.push(' ');
                }
                attribute_raw.pop();

                let mut attribute: String = String::new();
                let mut upper = 71;
                // Chop up string
                while !attribute_raw.is_empty() {
                    let range = 0..usize::min(upper, attribute_raw.len());
                    upper = 70;

                    attribute.push_str(&attribute_raw[range.clone()]);
                    attribute.push_str("\n ");
                    attribute_raw.replace_range(range, "");
                }

                attribute.pop();
                attribute
            }
        }
    }
}
