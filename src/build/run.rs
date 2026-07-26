use std::process::{Command, Stdio};

use crate::{cli::args::StartupFlags, project::{ImplicitBuildTask, TaskRunner}};

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
        info: &crate::model::ProjectInfo,
        project: &crate::model::Project,
        configuration: &crate::model::Configuration,
    ) -> Result<(), (String, u8)> {
        let _ = ImplicitBuildTask::new().invoke(&info, project, configuration);

        let mut java_command = Command::new("java");
        java_command.args(["-jar", "./wisteria/work/target.jar"]);
        java_command.args(&self.flags.passed_args);
        java_command.stdin(Stdio::piped());
        java_command.stdout(Stdio::piped());

        let mut child = match java_command.spawn() {
            Ok(c) => c,
            Err(e) => {
                return Err((format!("Failed to start Java process: {e}"), 1))
            }
        };

        let status = match child.wait() {
            Ok(s) => s,
            Err(e) => {
                return Err((format!("Failed to get child process's exit status: {e}"), 1))
            },
        };

        match status.code()
        {
            Some(c) => println!("Process exited with status {c}"),
            None => println!("Process exited with signal"),
        }

        Ok(())
    }

    fn phase_order(&self) -> &[String] {
        self.order.as_ref()
    }
}
