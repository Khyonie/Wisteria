pub struct Manifest
{
    entries: Vec<ManifestEntry>
}

pub enum ManifestEntry
{
    Version{ version: String },
    CreatedBy{ signature: String },
    MainClass{ class: String },
    ClassPath{ path: Vec<String> }
}

impl Manifest 
{
    pub fn new() -> Self 
    {
        let mut entries: Vec<ManifestEntry> = Vec::new();
        entries.push(ManifestEntry::Version{ version: String::from("1.0") });

        Self { entries }
    }

    pub fn add_entry(&mut self, entry: ManifestEntry)
    {
        self.entries.push(entry);
    }

    pub fn to_file(&self) -> String
    {
        let mut manifest: String = String::new();

        for entry in self.entries.iter()
        {
            manifest.push_str(&entry.to_header());
        }

        manifest
    }
}

impl ManifestEntry
{
    pub fn to_header(&self) -> String 
    {
        match self
        {
            ManifestEntry::Version { version } => format!("Manifest-Version: {version}\n"),
            ManifestEntry::CreatedBy { signature } => format!("Created-By: {signature}\n"),
            ManifestEntry::MainClass { class } => format!("Main-Class: {class}\n"),
            ManifestEntry::ClassPath { path } => {
                let mut attribute_raw: String = String::from("Class-Path: ");

                for s in path
                {
                    attribute_raw.push_str(s);
                    attribute_raw.push(' ');
                }
                attribute_raw.pop();

                let mut attribute: String = String::new();
                let mut upper = 71;
                // Chop up string
                while !attribute_raw.is_empty()
                {
                    let range = 0..usize::min(upper, attribute_raw.len());
                    upper = 70;

                    attribute.push_str(&attribute_raw[range.clone()]);
                    attribute.push_str("\n ");
                    attribute_raw.replace_range(range, "");
                }

                attribute.pop();
                attribute
            },
        }
    }
}
