use crate::ghk::{gh, git, util};
use anyhow::Result;

pub fn run() -> Result<()> {
    println!();

    // Git status
    if git::isrepo() {
        util::ok("Git: Ready");

        // Branch info
        if let Ok(branch) = git::currentbranch() {
            util::dim(&format!("Branch: {}", branch));
        }

        // Remote info
        if git::hasremote() {
            if let Ok(url) = git::remoteurl() {
                util::dim(&format!("Remote: {}", url));
            }
        } else {
            util::dim("Remote: Not connected (run 'ghk create')");
        }

        // Changes
        match git::haschanges() {
            Ok(true) => {
                let files = git::changedfiles().unwrap_or_default();
                util::warn(&format!("{} unsaved changes", files.len()));
                for file in files.iter().take(5) {
                    util::dim(&format!("  {}", file));
                }
                if files.len() > 5 {
                    util::dim(&format!("  ... and {} more", files.len() - 5));
                }
            }
            Ok(false) => {
                util::dim("No unsaved changes");
            }
            Err(_) => {}
        }
    } else {
        util::warn("Git: Not initialized");
        util::dim("Run 'ghk init' to set up");
    }

    println!();

    // GitHub status
    if gh::loggedin() {
        let user = gh::whoami().unwrap_or_else(|_| "unknown".to_string());
        util::ok(&format!("GitHub: Logged in as {}", user));
    } else {
        util::warn("GitHub: Not logged in");
        util::dim("Run 'ghk login' to connect");
    }

    println!();
    Ok(())
}
