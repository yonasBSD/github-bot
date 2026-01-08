use anyhow::{Context, Result, bail};
use indicatif::{ProgressBar, ProgressStyle};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Login to GitHub via gh CLI
pub fn login() -> Result<()> {
    let status = Command::new("gh")
        .args(["auth", "login"])
        .status()
        .context("Failed to run gh - is it installed?")?;

    if !status.success() {
        bail!("Login was cancelled or failed");
    }
    Ok(())
}

/// Logout from GitHub
pub fn logout() -> Result<()> {
    let status = Command::new("gh")
        .args(["auth", "logout"])
        .status()
        .context("Failed to run gh")?;

    if !status.success() {
        bail!("Logout failed");
    }
    Ok(())
}

/// Check if user is logged in
pub fn loggedin() -> bool {
    Command::new("gh")
        .args(["auth", "status"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Get current logged in username
pub fn whoami() -> Result<String> {
    let output = Command::new("gh")
        .args(["api", "user", "-q", ".login"])
        .output()
        .context("Failed to get current user")?;

    if !output.status.success() {
        bail!("Not logged in");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// List logged in accounts
pub fn listusers() -> Result<()> {
    let status = Command::new("gh")
        .args(["auth", "status"])
        .status()
        .context("Failed to run gh")?;

    if !status.success() {
        bail!("No accounts found");
    }
    Ok(())
}

/// Switch to a different account
pub fn switchuser(name: &str) -> Result<()> {
    let status = Command::new("gh")
        .args(["auth", "switch", "-u", name])
        .status()
        .context("Failed to switch user")?;

    if !status.success() {
        println!("Account '{}' not found locally. Please log in:", name);
        return login();
    }
    Ok(())
}

/// Create a new repository on GitHub with spinner
pub fn createrepo(name: &str, private: bool) -> Result<()> {
    let spinner = makespinner("Creating repository on GitHub...");

    let mut args = vec!["repo", "create", name, "--source=.", "--push"];
    if private {
        args.push("--private");
    } else {
        args.push("--public");
    }

    let output = Command::new("gh")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("Failed to create repository")?;

    spinner.finish_and_clear();

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        if err.contains("already exists") {
            bail!("Repository already exists with that name");
        }
        bail!(
            "Failed to create repository - {}",
            if err.contains("Name already exists") {
                "name is taken"
            } else {
                "check your connection"
            }
        );
    }
    Ok(())
}

/// Clone a repository by owner/repo name
pub fn clonerepo(repo: &str, dir: Option<&str>) -> Result<()> {
    let spinner = makespinner("Downloading repository...");

    let mut args = vec!["repo", "clone", repo];
    if let Some(d) = dir {
        args.push(d);
    }

    let output = Command::new("gh")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("Failed to clone repository")?;

    spinner.finish_and_clear();

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        if err.contains("already exists") {
            bail!("Directory already exists");
        }
        if err.contains("Could not resolve") {
            bail!("Repository not found - check the name");
        }
        bail!("Clone failed");
    }
    Ok(())
}

/// Open repository in browser
pub fn openrepo() -> Result<()> {
    let status = Command::new("gh")
        .args(["repo", "view", "--web"])
        .status()
        .context("Failed to open browser")?;

    if !status.success() {
        bail!("Could not open in browser");
    }
    Ok(())
}

/// Get gh CLI version
pub fn version() -> Option<String> {
    Command::new("gh").arg("--version").output().ok().map(|o| {
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .to_string()
    })
}

/// Check if we have SSH key configured
pub fn hassshkey() -> bool {
    Command::new("gh")
        .args(["ssh-key", "list"])
        .output()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false)
}

/// Check if we can reach GitHub (online check)
pub fn isonline() -> bool {
    Command::new("gh")
        .args(["api", "rate_limit"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Create a spinner
fn makespinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}
