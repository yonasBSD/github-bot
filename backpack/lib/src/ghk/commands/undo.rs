use crate::ghk::{git, util};
use anyhow::{Result, bail};
use dialoguer::Confirm;

pub fn run() -> Result<()> {
    if !git::isrepo() {
        util::err("Not a git repository");
        bail!("Not a git repository");
    }

    // Show what will be undone
    let history = git::history(1)?;
    if history.is_empty() {
        util::warn("No commits to undo");
        return Ok(());
    }

    util::info("Last commit:");
    util::dim(&format!("  {0}", history[0]));

    if !Confirm::new()
        .with_prompt("Undo this commit? (changes will be kept)")
        .default(false)
        .interact()?
    {
        util::dim("Cancelled");
        return Ok(());
    }

    git::undolast()?;
    util::ok("Commit undone");
    util::dim("Your changes are still there, just uncommitted");
    Ok(())
}
