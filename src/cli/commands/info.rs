use crate::cli::commands::project_or_exit;
use crate::model::Project;

pub fn trigger_info(project: Result<Project, (String, u8)>) {
    let project: Project = project_or_exit(project);
    project.print_info();
}
