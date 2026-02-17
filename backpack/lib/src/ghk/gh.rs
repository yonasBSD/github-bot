use anyhow::{Context, bail};
use indicatif::{ProgressBar, ProgressStyle};
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::ghk::config::Config;

/// Login to GitHub via gh CLI
pub fn login() -> anyhow::Result<()> {
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
pub fn logout() -> anyhow::Result<()> {
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

/// Get copyright holder name
pub fn copyright() -> anyhow::Result<String> {
    let cfg = Config::load();
    match cfg.org.as_deref() {
        Some(org) => Ok(org.to_string()),
        _ => whoami(),
    }
}

/// Get current logged in username
pub fn whoami() -> anyhow::Result<String> {
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
pub fn listusers() -> anyhow::Result<()> {
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
pub fn switchuser(name: &str) -> anyhow::Result<()> {
    let status = Command::new("gh")
        .args(["auth", "switch", "-u", name])
        .status()
        .context("Failed to switch user")?;

    if !status.success() {
        println!("Account '{name}' not found locally. Please log in:");
        return login();
    }
    Ok(())
}

/// Create a new repository on GitHub with spinner
pub fn createrepo(name: &str, private: bool) -> anyhow::Result<()> {
    let spinner = makespinner("Creating repository on GitHub...");

    let mut args = vec!["repo", "create", name, "--source=.", "--push"];
    if private {
        args.push("--private");
    } else {
        args.push("--public");
    }

    let output = Command::new("gh")
        .args(&args)
        .output()
        .context("Failed to create repository")?;

    spinner.finish_and_clear();

    if !output.status.success() {
        bail!("Failed to create repository");
    }

    // Set branch rules
    createruleset(name)?;

    // Enable Dependency Graph / Alerts
    enable_dep_graph(name)?;

    // Enable Auto-fix PRs
    enable_security_updates(name)?;

    Ok(())
}

/// Fork an existing repository
pub fn forkrepo(repo: &str, owner: &str) -> anyhow::Result<()> {
    let spinner = makespinner("Forking repository on GitHub...");

    let repo_name = repo
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .context("Could not determine repo name")?;

    let mut fork_target = format!("{owner}/{repo_name}");

    let mut args = vec![
        "repo",
        "fork",
        repo,
        "--default-branch-only",
        "--clone=false",
    ];

    let me = whoami().unwrap_or_default();
    if owner != me {
        args.extend(["--org", owner]);
    }

    let output = Command::new("gh")
        .args(&args)
        .output()
        .context("Failed to fork repository")?;

    spinner.finish_and_clear();

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to fork repository: {}", err.trim());
    }

    // Capture actual fork name from gh output in case GitHub renamed it (e.g. openclaw-1)
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if let Some(actual) = combined
        .split_whitespace()
        .find(|s| s.contains(&format!("{owner}/")))
    {
        if let Some(idx) = actual.find(&format!("{owner}/")) {
            fork_target = actual[idx..].trim_end_matches('/').to_string();
        }
    }

    // Give GitHub a moment to finish provisioning the fork
    let spinner = makespinner("Waiting for GitHub to provision fork...");
    std::thread::sleep(Duration::from_secs(3));
    spinner.finish_and_clear();

    createruleset(&fork_target)?;
    enable_dep_graph(&fork_target)?;
    enable_security_updates(&fork_target)?;

    Ok(())
}

/// Clone a repository by owner/repo name
pub fn clonerepo(repo: &str, dir: Option<&str>) -> anyhow::Result<()> {
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
pub fn openrepo() -> anyhow::Result<()> {
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
#[allow(clippy::literal_string_with_formatting_args)]
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

/// Create default ruleset
pub fn createruleset(name: &str) -> anyhow::Result<()> {
    let (owner, repo) = name
        .split_once('/')
        .expect("input must be in the form owner/repo");

    let endpoint = format!("repos/{owner}/{repo}/rulesets");

    let body = r#"
{
  "name": "default",
  "target": "branch",
  "enforcement": "active",
  "conditions": {
    "ref_name": {
      "include": ["~DEFAULT_BRANCH"],
      "exclude": []
    }
  },
  "bypass_actors": [
    {
      "actor_type": "OrganizationAdmin",
      "bypass_mode": "always"
    }
  ],
  "rules": [
    { "type": "required_signatures", "parameters": {} },
    { "type": "pull_request", "parameters": {
        "dismiss_stale_reviews_on_push": false,
        "require_code_owner_review": false,
        "require_last_push_approval": false,
        "required_approving_review_count": 0,
        "required_review_thread_resolution": false,
        "allowed_merge_methods": [
          "squash",
          "rebase"
        ]
      }
    },
    { "type": "non_fast_forward", "parameters": {} },
    { "type": "deletion", "parameters": {} }
  ]
}
"#;

    let mut child = Command::new("gh")
        .args([
            "api",
            "-X",
            "POST",
            &endpoint,
            "--input",
            "-",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "X-GitHub-Api-Version: 2022-11-28",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .context("Failed to run gh api")?;

    // Write JSON body into stdin AFTER spawning
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin.write_all(body.as_bytes()).ok();
    }

    let output = child
        .wait_with_output()
        .context("Failed to capture gh api output")?;

    if !output.status.success() {
        bail!(
            "gh api failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(())
}

/// Enable Dependency Graph and Security Analysis
pub fn enable_dep_graph(name: &str) -> anyhow::Result<()> {
    let (owner, repo) = name
        .split_once('/')
        .context("Repository name must be in the format 'owner/repo'")?;

    // Enable Vulnerability Alerts (this ensures dependency graph is active)
    // Documentation: https://docs.github.com/en/rest/vulnerability-alerts/vulnerability-alerts
    let endpoint = format!("repos/{owner}/{repo}/vulnerability-alerts");

    let output = Command::new("gh")
        .args([
            "api",
            "-X",
            "PUT",
            &endpoint,
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "X-GitHub-Api-Version: 2022-11-28",
        ])
        .output()
        .context("Failed to enable dependency graph via gh api")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        // Note: Some repos (like public ones) might have this enabled by default
        // We log the error but you might want to handle "already enabled" silently
        bail!("Failed to enable dependency graph: {}", err.trim());
    }

    Ok(())
}

/// Enable Dependabot Security Updates
pub fn enable_security_updates(name: &str) -> anyhow::Result<()> {
    let (owner, repo) = name
        .split_once('/')
        .context("Repository name must be in the format 'owner/repo'")?;

    // Enable automated security fixes
    // Documentation: https://docs.github.com/en/rest/vulnerability-alerts/automated-security-fixes
    let endpoint = format!("repos/{owner}/{repo}/automated-security-fixes");

    let output = Command::new("gh")
        .args([
            "api",
            "-X",
            "PUT",
            &endpoint,
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "X-GitHub-Api-Version: 2022-11-28",
        ])
        .output()
        .context("Failed to enable Dependabot security updates")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to enable security updates: {}", err.trim());
    }

    Ok(())
}
