pub mod classpath;
pub mod prefs;
pub mod project;

pub use classpath::generate_classpath;
pub use prefs::{generate_eclipse_config, generate_maven_config};
pub use project::generate_project;
