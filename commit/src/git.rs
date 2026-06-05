// SPDX-License-Identifier: MIT
use std::path::Path;
use std::process::Command;

pub fn staged_diff(dir: &Path) -> Result<String, String> {
    let out = Command::new("git")
        .current_dir(dir)
        .args(["diff", "--cached"])
        .output()
        .map_err(|e| format!("running git: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn commit(dir: &Path, message: &str, signoff: bool) -> Result<(), String> {
    let mut args = vec!["commit", "-m", message];
    if signoff {
        args.push("-s");
    }
    let out = Command::new("git")
        .current_dir(dir)
        .args(&args)
        .output()
        .map_err(|e| format!("running git: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    Ok(())
}
