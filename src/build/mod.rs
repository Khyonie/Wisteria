pub mod compile;
pub mod defined;
pub mod implicit;
pub mod package;
pub mod resolve;
pub mod run;
pub mod shade;
pub mod sources;
pub mod task;

use crate::{
    model::{Configuration, Project},
    project::{ImplicitBuildTask, TaskRunner},
};

pub fn build_project(project: &Project, configuration: &Configuration) -> Result<(), String> {
    ImplicitBuildTask::new().invoke(project.info(), project, configuration)
}
