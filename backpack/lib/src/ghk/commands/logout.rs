use crate::ghk::{gh, util};
use anyhow::Result;

pub fn run() -> Result<()> {
    if !gh::loggedin() {
        util::warn("Not logged in");
        util::dim("Nothing to do");
        return Ok(());
    }

    let user = gh::whoami().unwrap_or_else(|_| "unknown".to_string());
    util::info(&format!("Logging out from {}...", user));

    gh::logout()?;
    util::ok("Logged out");
    Ok(())
}
