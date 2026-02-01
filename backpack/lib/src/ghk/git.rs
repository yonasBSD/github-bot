use anyhow::{Context, Result, bail};
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    process::{Command, Stdio},
    time::Duration,
};

/// Check if current directory is inside a git repo
pub fn isrepo() -> bool {
    git2::Repository::discover(".").is_ok()
}

/// Initialize a new git repository
pub fn init() -> Result<()> {
    let status = Command::new("git")
        .arg("init")
        .status()
        .context("Failed to run git")?;

    if !status.success() {
        bail!("git init failed");
    }
    Ok(())
}

/// Check if a remote named 'origin' exists
pub fn hasremote() -> bool {
    Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// Check if there are uncommitted changes
pub fn haschanges() -> Result<bool> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .context("Failed to run git status")?;

    Ok(!output.stdout.is_empty())
}

/// Get list of changed files (for display)
pub fn changedfiles() -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .context("Failed to run git status")?;

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

/// Stage all changes
pub fn addall() -> Result<()> {
    let status = Command::new("git")
        .args(["add", "-A"])
        .status()
        .context("Failed to run git add")?;

    if !status.success() {
        bail!("git add failed");
    }
    Ok(())
}

/// Commit with message
pub fn commit(msg: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["commit", "-m", msg])
        .status()
        .context("Failed to run git commit")?;

    if !status.success() {
        bail!("git commit failed");
    }
    Ok(())
}

/// Push to origin with spinner
pub fn push() -> Result<()> {
    let spinner = makespinner("Pushing to GitHub...");

    let output = Command::new("git")
        .args(["push", "-u", "origin", "HEAD"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("Failed to run git push")?;

    spinner.finish_and_clear();

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        if err.contains("rejected") || err.contains("non-fast-forward") {
            bail!("Push rejected - run 'ghk pull' first to sync changes");
        }
        bail!("git push failed - check your permissions and try again");
    }
    Ok(())
}

/// Pull from origin with spinner
pub fn pull() -> Result<()> {
    let spinner = makespinner("Syncing from GitHub...");

    let output = Command::new("git")
        .args(["pull", "--rebase"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("Failed to run git pull")?;

    spinner.finish_and_clear();

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        if err.contains("CONFLICT") {
            bail!("Merge conflict detected - please resolve manually");
        }
        bail!("git pull failed");
    }
    Ok(())
}

/// Clone a repository with spinner
#[allow(dead_code)]
pub fn clone(url: &str, dir: Option<&str>) -> Result<()> {
    let spinner = makespinner("Downloading repository...");

    let mut args = vec!["clone", "--progress", url];
    if let Some(d) = dir {
        args.push(d);
    }

    let output = Command::new("git")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("Failed to run git clone")?;

    spinner.finish_and_clear();

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        if err.contains("already exists") {
            bail!("Directory already exists");
        }
        bail!("Clone failed - check the URL and try again");
    }
    Ok(())
}

/// Get current branch name
pub fn currentbranch() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .context("Failed to get current branch")?;

    if !output.status.success() {
        bail!("Not on any branch");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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

/// Undo last commit (keep changes)
pub fn undolast() -> Result<()> {
    let status = Command::new("git")
        .args(["reset", "--soft", "HEAD~1"])
        .status()
        .context("Failed to undo")?;

    if !status.success() {
        bail!("Undo failed - may be no commits to undo");
    }
    Ok(())
}

/// Get recent commit history
pub fn history(count: usize) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["log", "--oneline", "-n", &count.to_string()])
        .output()
        .context("Failed to get history")?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.lines().map(std::string::ToString::to_string).collect())
}

/// Check if there are unpushed commits
#[allow(dead_code)]
pub fn hasunpushed() -> bool {
    Command::new("git")
        .args(["log", "@{u}..", "--oneline"])
        .output()
        .map(|out| !out.stdout.is_empty())
        .unwrap_or(false)
}

/// Check if there are unpulled commits
#[allow(dead_code)]
pub fn hasunpulled() -> bool {
    // fetch first to check
    let _ = Command::new("git").args(["fetch", "--quiet"]).status();

    Command::new("git")
        .args(["log", "..@{u}", "--oneline"])
        .output()
        .map(|out| !out.stdout.is_empty())
        .unwrap_or(false)
}

/// Get git version
pub fn version() -> Option<String> {
    Command::new("git")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// Create a spinner
pub fn makespinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}
