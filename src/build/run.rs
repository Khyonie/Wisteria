use std::process::Command;

use crate::{
    cli::args::StartupFlags,
    model::{Configuration, Project, ProjectInfo},
    project::{ImplicitBuildTask, TaskRunner},
    util::consts,
};

pub struct ImplicitRunTask {
    order: Vec<String>,
    flags: StartupFlags
}

impl ImplicitRunTask {
    pub fn new(flags: StartupFlags) -> Self {
        ImplicitRunTask {
            order: vec![
                String::from("collect"),
                String::from("compile"),
                String::from("shade"),
                String::from("package"),
                String::from("run"),
            ],
            flags
        }
    }
}

impl TaskRunner for ImplicitRunTask
{
    fn invoke(
        &self,
        info: &ProjectInfo,
        project: &Project,
        configuration: &Configuration,
    ) -> Result<(), (String, u8)> {
        ImplicitBuildTask::new().invoke(info, project, configuration)?;

        self.run()
    }

    fn phase_order(&self) -> &[String] {
        self.order.as_ref()
    }
}

impl ImplicitRunTask
{
    fn run(&self) -> Result<(), (String, u8)>
    {
        let mut java_command = Command::new("java");
        java_command.args(["-jar", consts::TARGET_JAR_PATH]);
        java_command.args(&self.flags.passed_args);

        let status = match java_command.status() {
            Ok(s) => s,
            Err(e) => {
                return Err((format!("Failed to start Java process: {e}"), 1))
            }
        };

        if !status.success() {
            let code = status.code().unwrap_or(1);
            return Err((
                format!("Java process exited with status {status}"),
                u8::try_from(code).unwrap_or(1),
            ));
        }

        Ok(())
    }
}
