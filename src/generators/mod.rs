pub mod eclipse;
pub mod git;
pub mod maven;
pub mod metadata;
pub mod wisteria_project;

pub use metadata::{WISTERIA_METADATA_TEMPLATE, generate_metadata};
pub use wisteria_project::generate_wisteria_project;
