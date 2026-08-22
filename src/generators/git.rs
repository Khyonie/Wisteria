use std::{
    fs::{self},
    path::Path,
};

use crate::util::git;

pub const GIT_DIRECTORY: &str = ".git";
pub const GIT_IGNORE: &str = ".gitignore";
pub const GIT_IGNORE_CONTENTS: &str = ".wisteria"; // Ignore wisteria file by default

pub const GIT_HEAD_PATH: &str = "HEAD";
pub const HEAD_CONTENTS: &str = "ref: refs/heads";
pub const DEFAULT_BRANCH_NAME: &str = "master";

pub const GIT_OBJECTS_INFO: &str = "objects/info";
pub const GIT_OBJECTS_PACK: &str = "objects/pack";
pub const GIT_REFS_HEAD: &str = "refs/heads";
pub const GIT_REFS_TAGS: &str = "refs/tags";

pub const GIT_CONFIG_PATH: &str = "config";
pub const GIT_CONFIG_CONTENTS: &str = "\
[core]
\trepositoryformatversion = 0
\tbare = false
\tlogallrefupdates = true
";

pub fn initialize_git_repository(project_path: &Path) -> Result<(), String> {
    let git_path = project_path.join(GIT_DIRECTORY);

    // The folder should be very new, so it shouldn't exist and we don't need to check if it does.

    // Folder structure
    fs::create_dir_all(git_path.join(GIT_OBJECTS_INFO)).map_err(|e| e.to_string())?;
    fs::create_dir_all(git_path.join(GIT_OBJECTS_PACK)).map_err(|e| e.to_string())?;
    fs::create_dir_all(git_path.join(GIT_REFS_HEAD)).map_err(|e| e.to_string())?;
    fs::create_dir_all(git_path.join(GIT_REFS_TAGS)).map_err(|e| e.to_string())?;

    // Files
    fs::write(
        git_path.join(GIT_HEAD_PATH),
        head_contents_for_branch(&git::configured_default_branch_name(DEFAULT_BRANCH_NAME)?),
    )
    .map_err(|e| e.to_string())?;
    fs::write(git_path.join(GIT_CONFIG_PATH), GIT_CONFIG_CONTENTS).map_err(|e| e.to_string())?;

    // Gitignore
    fs::write(project_path.join(GIT_IGNORE), GIT_IGNORE_CONTENTS).map_err(|e| e.to_string())?;

    Ok(())
}

fn head_contents_for_branch(branch_name: &str) -> String {
    format!("{HEAD_CONTENTS}/{branch_name}\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_contents_for_branch_uses_single_trailing_newline() {
        assert_eq!(
            head_contents_for_branch("main"),
            String::from("ref: refs/heads/main\n")
        );
    }
}
