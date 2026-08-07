use std::fmt::{self, Display};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencyReference {
    name: String,
    scope: DependencyScope,
    packaging: Option<PackagingType>,
}

impl DependencyReference {
    pub fn new(name: String, scope: DependencyScope, packaging: Option<PackagingType>) -> Self {
        Self {
            name,
            scope,
            packaging,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn scope(&self) -> DependencyScope {
        self.scope
    }

    pub fn packaging(&self) -> Option<PackagingType> {
        self.packaging
    }

    pub fn is_shaded(&self) -> bool {
        self.packaging == Some(PackagingType::Shade)
    }
}

impl fmt::Display for DependencyReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackagingType {
    #[default]
    Shade,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum DependencyScope {
    #[default]
    Compile,
    Runtime,
    Provided,
    Test,
}

impl DependencyScope {
    pub fn is_on_compile_classpath(self) -> bool {
        matches!(self, Self::Compile | Self::Provided)
    }

    pub fn is_on_runtime_classpath(self) -> bool {
        matches!(self, Self::Compile | Self::Runtime)
    }

    pub fn is_test_only(self) -> bool {
        matches!(self, Self::Test)
    }

    pub fn maven_scope(self) -> Option<&'static str> {
        match self {
            Self::Compile => None,
            Self::Runtime => Some("runtime"),
            Self::Provided => Some("provided"),
            Self::Test => Some("test"),
        }
    }
}

impl Display for DependencyScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DependencyScope::Compile => write!(f, "compile"),
            DependencyScope::Runtime => write!(f, "runtime"),
            DependencyScope::Provided => write!(f, "provided"),
            DependencyScope::Test => write!(f, "test"),
        }
    }
}

impl Display for PackagingType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackagingType::Shade => write!(f, "shade"),
        }
    }
}

impl TryFrom<String> for PackagingType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "shade" => Ok(PackagingType::Shade),
            _ => Err(format!("No such packaging type \"{value}\"")),
        }
    }
}

impl TryFrom<String> for DependencyScope {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "compile" => Ok(DependencyScope::Compile),
            "runtime" => Ok(DependencyScope::Runtime),
            "test" => Ok(DependencyScope::Test),
            "provided" => Ok(DependencyScope::Provided),
            _ => Err(format!("No such dependency scope \"{value}\"")),
        }
    }
}
