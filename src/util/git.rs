use std::{
    env,
    io::ErrorKind,
    process::{Command, Output},
};

/// Git's default "no config setting" exit code
const NO_CONFIG_SETTING_CODE: i32 = 1;

const DEFAULT_BRANCH_SETTING: &str = "init.defaultBranch";

pub fn configured_default_branch_name(fallback: &str) -> Result<String, String> {
    let Some(branch_name) = get_global_config_setting(DEFAULT_BRANCH_SETTING)? else {
        return Ok(fallback.to_string());
    };

    validate_branch_name(&branch_name)?;
    Ok(branch_name)
}

pub fn get_global_config_setting(setting: &str) -> Result<Option<String>, String> {
    let command = format!("git config --global --get {setting}");
    let output = match git_command()
        .args(["config", "--global", "--get", setting])
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Failed to run `{command}`: {error}")),
    };

    if let Some(code) = output.status.code()
        && code == NO_CONFIG_SETTING_CODE
    {
        return Ok(None);
    }

    if !output.status.success() {
        return Err(git_command_failure(&command, &output));
    }

    parse_config_output(output.stdout)
}

fn parse_config_output(stdout: Vec<u8>) -> Result<Option<String>, String> {
    String::from_utf8(stdout)
        .map(|value| Some(value.trim().to_string()))
        .map_err(|e| format!("Failed to parse git command output as UTF-8: {e}"))
}

fn validate_branch_name(branch_name: &str) -> Result<(), String> {
    if branch_name.is_empty() {
        return Err(format!(
            "Global git config setting {DEFAULT_BRANCH_SETTING} is empty.\nFix: run `git config --global {DEFAULT_BRANCH_SETTING} main`, or remove the setting to use Wisteria's default branch name."
        ));
    }

    let command = format!("git check-ref-format --branch {branch_name}");
    let output = git_command()
        .args(["check-ref-format", "--branch", branch_name])
        .output()
        .map_err(|e| format!("Failed to run `{command}`: {e}"))?;

    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "Global git config setting {DEFAULT_BRANCH_SETTING} has invalid branch name \"{branch_name}\".\n{}\nFix: run `git config --global {DEFAULT_BRANCH_SETTING} main`, or choose another valid Git branch name.",
        git_command_failure(&command, &output)
    ))
}

fn git_command_failure(command: &str, output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    if stderr.is_empty() {
        return format!("`{command}` exited with status {}.", output.status);
    }

    format!("`{command}` exited with status {}: {stderr}", output.status)
}

fn git_command() -> Command {
    let mut command = Command::new("git");
    command.current_dir(env::temp_dir());
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config_output_trims_git_newline() {
        assert_eq!(
            parse_config_output(b"main\n".to_vec()).unwrap(),
            Some(String::from("main"))
        );
    }

    #[test]
    fn parse_config_output_preserves_empty_config_value() {
        assert_eq!(
            parse_config_output(b"\n".to_vec()).unwrap(),
            Some(String::new())
        );
    }

    #[test]
    fn validate_branch_name_rejects_empty_branch_before_running_git() {
        let error = validate_branch_name("").unwrap_err();

        assert!(error.contains("init.defaultBranch is empty"));
        assert!(error.contains("git config --global init.defaultBranch main"));
    }
}
