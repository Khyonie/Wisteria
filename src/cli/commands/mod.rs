use std::collections::HashMap;

use regex::Regex;

use crate::dependency::{Dependency, UpdateContext};
use crate::util::consts;

pub mod clean;
pub mod create;
pub mod info;
pub mod refresh;
pub mod switch;
pub mod task;
pub mod update;

pub(crate) fn print_header() {
    println!("Wisteria v{}", consts::VERSION);
    println!("Copyright © 2026 Hailey-Jane \"Khyonie\" Garrett <http://www.khyonieheart.coffee/>");
}

pub(crate) fn envvar_regexes() -> HashMap<&'static str, Regex> {
    let mut regexes: HashMap<&str, Regex> = HashMap::new();
    regexes.insert("envvars", Regex::new(r#"\{(.+?)}"#).unwrap());
    regexes
}

pub(crate) fn update_dependencies_with_context(
    targets: &[String],
    dependencies: &HashMap<String, Dependency>,
    environment: &HashMap<String, String>,
    regexes: &HashMap<&str, Regex>,
    context: UpdateContext,
) -> Vec<(String, String)> {
    let mut width: usize = usize::MIN;
    for name in targets.iter() {
        width = usize::max(name.len(), width);
    }

    width += 5;

    let mut failed_downloads: Vec<(String, String)> = Vec::new();
    let size = targets.len();
    for (index, target) in targets.iter().enumerate() {
        match dependencies.get_key_value(target) {
            Some((name, dep)) => {
                print!(
                    "({}/{size}) Updating {:width$}",
                    index + 1,
                    format!("{name} ... ")
                );
                let _ = match dep.resolve(name, environment, regexes, context) {
                    Ok(p) => p,
                    Err(e) => {
                        println!("Could not download {name}: {}", e.0);
                        failed_downloads.push((name.clone(), e.0));
                        continue;
                    }
                };
            },
            None => {
                println!("Usage of undeclared dependency \"{target}\"");
            }
        }
    }

    failed_downloads
}
