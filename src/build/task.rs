use crate::{
    model::{Configuration, Project, ProjectInfo},
    output::OutputRenderer,
};

pub use crate::build::defined::DefinedTask;
pub use crate::build::implicit::ImplicitBuildTask;

pub trait TaskRunner {
    fn invoke(
        &self,
        info: &ProjectInfo,
        project: &Project,
        configuration: &Configuration,
        output: &mut TaskOutput<'_>,
    ) -> Result<(), String>;

    fn phase_order(&self) -> &[String];
}

pub struct TaskOutput<'a> {
    renderer: &'a mut dyn OutputRenderer,
    operation: &'a str,
    total_steps: usize,
}

impl<'a> TaskOutput<'a> {
    pub fn new(
        renderer: &'a mut dyn OutputRenderer,
        operation: &'a str,
        total_steps: usize,
    ) -> Self {
        Self {
            renderer,
            operation,
            total_steps,
        }
    }

    pub fn renderer(&mut self) -> &mut dyn OutputRenderer {
        self.renderer
    }

    pub fn operation_started(&mut self) {
        self.renderer
            .operation_started(self.operation, self.total_steps);
    }

    pub fn operation_completed(&mut self, message: &str) {
        self.renderer.operation_completed(self.operation, message);
    }

    pub fn step_started(&mut self, action: &str, item: &str, index: usize) {
        self.renderer
            .step_started(self.operation, action, item, index, self.total_steps);
    }

    pub fn step_completed(&mut self, action: &str, item: &str, index: usize, message: &str) {
        self.renderer.step_completed(
            self.operation,
            action,
            item,
            index,
            self.total_steps,
            message,
        );
    }

    pub fn step_failed(&mut self, action: &str, item: &str, index: usize, message: &str) {
        self.renderer.step_failed(
            self.operation,
            action,
            item,
            index,
            self.total_steps,
            message,
        );
    }

    pub fn log(&mut self, message: &str) {
        self.renderer.log(message);
    }

    pub fn suspend(&mut self) {
        self.renderer.suspend();
    }
}
