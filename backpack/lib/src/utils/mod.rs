use dialoguer::Input;
use anyhow::{bail, Result, Context};
use std::process::Command;

/// Check if current directory is inside a git repo
pub fn isrepo() -> bool {
    git2::Repository::discover(".").is_ok()
}

/// Get remote URL
pub fn remoteurl() -> Result<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .context("Failed to get remote URL")?;

    if !output.status.success() {
        bail!("No remote configured");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Get the target repo
pub fn get_repo(target: Option<String>) -> Result<String> {
    let repo = if let Some(t) = target {
        t
    } else if isrepo() {
        remoteurl().context("Could not determine remote URL of current repo")?
    } else {
        Input::new()
            .with_prompt("Repository to merge (owner/repo or URL)")
            .interact_text()?
    };

    Ok(repo)
}
