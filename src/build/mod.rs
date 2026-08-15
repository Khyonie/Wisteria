pub mod compile;
pub mod defined;
pub mod implicit;
pub mod javadoc;
pub mod package;
pub mod resolve;
pub mod run;
pub mod shade;
pub mod sources;
pub mod task;

use crate::{
    model::{Configuration, Project},
    output::{self, OutputMode},
    project::{ImplicitBuildTask, TaskOutput, TaskRunner},
};

pub fn build_project(project: &Project, configuration: &Configuration) -> Result<(), String> {
    let task = ImplicitBuildTask::new();
    let mut renderer = output::renderer(OutputMode::Plain);
    let mut output = TaskOutput::new(renderer.as_mut(), "build", task.phase_order().len());

    output.operation_started();
    let result = task.invoke(project.info(), project, configuration, &mut output);
    match &result {
        Ok(()) => output.operation_completed("Built project"),
        Err(error) => {
            output.log(error);
            output.operation_completed("Task finished with errors.");
        }
    }

    result
}
