use std::process::Command;

use crate::{
    cli::args::StartupFlags,
    model::{Configuration, Project, ProjectInfo},
    project::{ImplicitBuildTask, TaskOutput, TaskRunner},
    util::{consts, exit_code},
};

pub struct ImplicitRunTask {
    order: Vec<String>,
    flags: StartupFlags,
}

impl ImplicitRunTask {
    pub fn new(flags: StartupFlags) -> Self {
        ImplicitRunTask {
            order: vec![
                String::from("resolve"),
                String::from("collect"),
                String::from("compile"),
                String::from("shade"),
                String::from("package"),
                String::from("run"),
            ],
            flags,
        }
    }
}

impl TaskRunner for ImplicitRunTask {
    fn invoke(
        &self,
        info: &ProjectInfo,
        project: &Project,
        configuration: &Configuration,
        output: &mut TaskOutput<'_>,
    ) -> Result<(), String> {
        ImplicitBuildTask::new().invoke(info, project, configuration, output)?;

        output.step_started("Running", "application", 6);
        output.suspend();
        match self.run() {
            Ok(()) => {
                output.step_completed("Running", "application", 6, "Done");
                Ok(())
            }
            Err(error) => {
                output.step_failed("Running", "application", 6, &error);
                Err(error)
            }
        }
    }

    fn phase_order(&self) -> &[String] {
        self.order.as_ref()
    }
}

impl ImplicitRunTask {
    fn run(&self) -> Result<(), String> {
        let mut java_command = Command::new("java");
        java_command.args(["-jar", consts::TARGET_JAR_PATH]);
        java_command.args(&self.flags.passed_args);

        let status = match java_command.status() {
            Ok(s) => s,
            Err(e) => return Err(format!("Failed to start Java process: {e}")),
        };

        if !status.success() {
            exit_code::record_external_process_exit_code(status);
            return Err(format!("Java process exited with status {status}"));
        }

        Ok(())
    }
}
