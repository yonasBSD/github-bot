use crate::ghk::{config::Config, gh, git, util};
use anyhow::{Context, Result, bail};
use dialoguer::Input;

pub fn run(target: Option<String>) -> Result<()> {
    if !gh::loggedin() {
        util::err("Not logged in to GitHub");
        util::dim("Run 'ghk login' first to connect your account");
        bail!("Not logged in");
    }

    // Determine upstream repo to fork
    let upstream = if let Some(t) = target {
        t.to_string()
    } else if git::isrepo() {
        git::remoteurl().context("Could not determine remote URL of current repo")?
    } else {
        Input::new()
            .with_prompt("Repository to fork (owner/repo or URL)")
            .interact_text()?
    };

    // Determine destination owner (org or personal account)
    let cfg = Config::load();
    let owner = if let Some(org) = cfg.org.as_deref() {
        org.to_string()
    } else {
        gh::whoami()?
    };

    gh::forkrepo(&upstream, &owner)?;

    util::ok(&format!("Repository forked into '{owner}'!"));
    util::dim("Security features have been enabled:");
    util::ok("  dependency graph");
    util::ok("  security updates");
    util::dim("Run 'ghk push' to save your changes");
    Ok(())
}
