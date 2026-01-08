use crate::ghk::{git, util};
use anyhow::{Result, bail};
use std::process::Command;

pub fn run(name: Option<String>) -> Result<()> {
    if !git::isrepo() {
        util::err("Not a git repository");
        bail!("Not a git repository");
    }

    match name {
        // Switch to branch
        Some(branch) => {
            util::info(&format!("Switching to {}...", branch));

            let status = Command::new("git").args(["checkout", &branch]).status()?;

            if status.success() {
                util::ok(&format!("Now on {}", branch));
            } else {
                // Maybe it's a new branch?
                util::info("Branch not found, creating it...");
                let status = Command::new("git")
                    .args(["checkout", "-b", &branch])
                    .status()?;

                if status.success() {
                    util::ok(&format!("Created and switched to {}", branch));
                } else {
                    util::err("Could not switch branch");
                }
            }
        }
        // List branches
        None => {
            let current = git::currentbranch().unwrap_or_default();

            let output = Command::new("git").args(["branch", "--list"]).output()?;

            let text = String::from_utf8_lossy(&output.stdout);

            println!();
            util::info("Branches:");
            for line in text.lines() {
                let name = line.trim().trim_start_matches("* ");
                if name == current {
                    println!("  \x1b[32m▶ {}\x1b[0m (current)", name);
                } else {
                    util::dim(&format!("  {}", name));
                }
            }
            println!();
        }
    }

    Ok(())
}
