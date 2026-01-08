use crate::ghk::{gh, git, util};
use anyhow::{Result, bail};
use dialoguer::{Confirm, Input};

pub fn run() -> Result<()> {
    // Check prerequisites
    if !git::isrepo() {
        util::err("Not a git repository");
        util::dim("Run 'ghk init' first to set up your project");
        bail!("Not a git repository");
    }

    if !gh::loggedin() {
        util::err("Not logged in to GitHub");
        util::dim("Run 'ghk login' first to connect your account");
        bail!("Not logged in");
    }

    // Check if remote already exists
    if git::hasremote() {
        let url = git::remoteurl().unwrap_or_else(|_| "unknown".to_string());
        util::warn("Repository already connected to GitHub");
        util::dim(&format!("  {}", url));
        util::dim("Run 'ghk push' to save your changes");
        return Ok(());
    }

    // Get repo name
    let defaultname = std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "my-project".to_string());

    let name: String = Input::new()
        .with_prompt("Repository name")
        .default(defaultname)
        .interact_text()?;

    let private = Confirm::new()
        .with_prompt("Make it private?")
        .default(false)
        .interact()?;

    // Make sure there's at least one commit
    if git::haschanges()? || !hasanycommits() {
        util::info("Creating initial save...");
        git::addall()?;
        let _ = git::commit("Initial commit");
    }

    util::info("Creating repository on GitHub...");
    gh::createrepo(&name, private)?;

    util::ok(&format!("Repository '{}' created!", name));
    util::dim("Run 'ghk push' to save your changes");
    Ok(())
}

fn hasanycommits() -> bool {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}
