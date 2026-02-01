use crate::ghk::{gh, util};
use anyhow::Result;
use dialoguer::Input;

pub fn run(repo: Option<String>, dir: Option<String>) -> Result<()> {
    // Check if online
    if !gh::isonline() {
        util::err("Cannot reach GitHub");
        util::dim("Check your internet connection");
        anyhow::bail!("Offline");
    }

    // Get repo name if not provided
    let reponame = match repo {
        Some(r) => r,
        None => Input::new()
            .with_prompt("Repository (owner/name or URL)")
            .interact_text()?,
    };

    util::info(&format!("Cloning {reponame}..."));
    gh::clonerepo(&reponame, dir.as_deref())?;

    let dirname = dir.unwrap_or_else(|| {
        reponame
            .split('/')
            .next_back()
            .unwrap_or(&reponame)
            .to_string()
    });

    util::ok(&format!("Downloaded to '{dirname}'"));
    util::dim(&format!("cd {dirname} to start working"));
    Ok(())
}
