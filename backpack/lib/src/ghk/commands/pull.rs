use crate::ghk::{gh, git, util};
use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    // Check prerequisites
    if !git::isrepo() {
        util::err("Not a git repository");
        util::dim("Run 'ghk init' first");
        bail!("Not a git repository");
    }

    if !git::hasremote() {
        util::err("Not connected to GitHub");
        util::dim("Run 'ghk create' first");
        bail!("No remote configured");
    }

    // Check if online
    if !gh::isonline() {
        util::err("Cannot reach GitHub");
        util::dim("Check your internet connection");
        bail!("Offline");
    }

    // Check for local changes that might conflict
    if git::haschanges()? {
        util::warn("You have unsaved changes");
        util::dim("Consider running 'ghk push' first");
    }

    util::info("Syncing from GitHub...");
    git::pull()?;

    util::ok("Synced!");
    Ok(())
}
