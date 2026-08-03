#[derive(Clone)]
pub struct DependencyReference
{
    name: String,
    scope: DependencyScope,
    packaging: Option<PackagingType>
}

impl DependencyReference
{
    pub fn new(name: String, scope: DependencyScope, packaging: Option<PackagingType>) -> Self
    {
        Self { name, scope, packaging }
    }
}

#[derive(Default, Clone)]
pub enum PackagingType
{
    #[default]
    Shade,
}

#[derive(Default, Clone)]
pub enum DependencyScope
{
    #[default]
    Compile,
    Runtime,
    Provided,
    Test,
}

impl TryFrom<String> for PackagingType
{
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str()
        {
            "shade" => Ok(PackagingType::Shade),
            _ => Err(format!("No such packaging type \"{value}\""))
        }
    }
}

impl TryFrom<String> for DependencyScope
{
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str()
        {
            "compile" => Ok(DependencyScope::Compile),
            "runtime" => Ok(DependencyScope::Runtime),
            "test" => Ok(DependencyScope::Test),
            "provided" => Ok(DependencyScope::Provided),
            _ => Err(format!("No such dependency scope \"{value}\""))
        }
    }
}
