use crate::{model::{Configuration, Project}, project::{ImplicitBuildTask, TaskRunner}};

pub fn build_project(project: &Project, configuration: &Configuration) -> Result<(), (String, u8)>
{
    let _ = ImplicitBuildTask::new().invoke(project.info(), project, configuration);

    Ok(())
}
