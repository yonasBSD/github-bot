use crate::ghk::{gh, git, util};
use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    if !git::isrepo() {
        util::err("Not a git repository");
        bail!("Not a git repository");
    }

    if !git::hasremote() {
        util::err("Not connected to GitHub");
        util::dim("Run 'ghk create' first");
        bail!("No remote configured");
    }

    util::info("Opening in browser...");
    gh::openrepo()?;
    util::ok("Opened");
    Ok(())
}
