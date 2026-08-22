use std::process::Command;

use which::which;

/// Git's default "no config setting" exit code
const NO_CONFIG_SETTING_CODE: i32 = 1;
pub fn get_global_config_setting(setting: &str) -> Option<String>
{
    // Check if git is installed
    if which("git").is_err()
    {
        return None
    }
    
    let output = Command::new("git")
        .args([ "config", "--global", setting ])
        .output()
        .ok()?;

    if let Some(code) = output.status.code() && code == NO_CONFIG_SETTING_CODE
    {
        return None
    }

    String::from_utf8(output.stdout)
        .ok()
}
