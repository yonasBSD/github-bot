use crate::ghk::{gh, util};
use anyhow::Result;

pub fn run() -> Result<()> {
    if gh::loggedin() {
        let user = gh::whoami().unwrap_or_else(|_| "unknown".to_string());
        util::ok(&format!("Already logged in as {user}"));
        util::dim("Run 'ghk logout' first if you want to switch accounts");
        return Ok(());
    }

    util::info("Opening GitHub login...");
    gh::login()?;

    if gh::loggedin() {
        let user = gh::whoami().unwrap_or_else(|_| "unknown".to_string());
        util::ok(&format!("Connected as {user}"));
    } else {
        util::warn("Login was cancelled");
    }

    Ok(())
}
