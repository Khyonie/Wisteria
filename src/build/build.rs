use crate::{
    model::{Configuration, Project},
    project::{ImplicitBuildTask, TaskRunner},
};

pub fn build_project(project: &Project, configuration: &Configuration) -> Result<(), (String, u8)> {
    ImplicitBuildTask::new().invoke(project.info(), project, configuration)
}
