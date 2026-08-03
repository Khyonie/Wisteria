pub struct DependencyReference
{
    name: String,
    scope: DependencyScope,
    packaging: PackagingType
}

#[derive(Default)]
pub enum PackagingType
{
    #[default]
    Shade,
}

#[derive(Default)]
pub enum DependencyScope
{
    #[default]
    Compile,
    Runtime,
    Provided,
    Test,
}
