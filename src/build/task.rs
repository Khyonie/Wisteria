use crate::model::{Configuration, Project, ProjectInfo};

pub use crate::build::defined::DefinedTask;
pub use crate::build::implicit::ImplicitBuildTask;

pub trait TaskRunner {
    fn invoke(
        &self,
        info: &ProjectInfo,
        project: &Project,
        configuration: &Configuration,
    ) -> Result<(), String>;

    fn phase_order(&self) -> &[String];
}
