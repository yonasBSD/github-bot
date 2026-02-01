use std::process::{Command, ExitStatus};
use tracing::{debug, warn};

fn run(cmd: &mut Command) -> anyhow::Result<ExitStatus> {
    let status = cmd.status()?;
    Ok(status)
}

fn ensure_success(status: ExitStatus, msg: &str) -> anyhow::Result<()> {
    if !status.success() {
        anyhow::bail!("{}", msg);
    }
    Ok(())
}

pub fn wip(no_push: bool, no_diff: bool, rewind: Option<u32>) -> anyhow::Result<()> {
    // Check if working directory is clean
    debug!(
        command = "git status -s",
        "Checking working directory status"
    );
    let output = Command::new("git").args(["status", "-s"]).output()?;

    debug!(
        stdout = %String::from_utf8_lossy(&output.stdout),
        stderr = %String::from_utf8_lossy(&output.stderr),
        "Git status output received"
    );

    if output.stdout.is_empty() {
        debug!("Working directory is clean; nothing to do");
        return Ok(());
    }

    // Format code
    let repo = git2::Repository::discover(".")?;
    let repo_root = repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("Cannot find repository working directory"))?;

    debug!(repo_root = %repo_root.display(), "Located git repository root");

    let has_treefmt_config =
        repo_root.join("treefmt.toml").exists() || repo_root.join(".treefmt.toml").exists();

    if has_treefmt_config {
        debug!("Found treefmt config; executing formatting...");
        let mut cmd = Command::new("treefmt");
        cmd.current_dir(repo_root);

        // We run the command but don't use 'ensure_success' or '?'
        // because we don't want check failures to block our WIP commit.
        match run(&mut cmd) {
            Ok(status) if status.success() => {
                debug!("treefmt completed successfully");
            }
            Ok(status) => {
                // treefmt returns non-zero if a --check fails or files were changed.
                // We log this as a debug message and continue to the git add/commit stage.
                debug!(
                    exit_code = ?status.code(),
                    "treefmt finished with non-zero status (likely due to diffs), continuing to commit..."
                );
            }
            Err(e) => {
                // This triggers if the treefmt binary itself is missing or cannot execute.
                // We warn the user but don't crash the program.
                warn!(error = %e, "Failed to execute treefmt binary; skipping format stage");
            }
        }
    }

    // Show diff unless suppressed
    if !no_diff {
        run(&mut Command::new("git").args(["--no-pager", "diff"]))?;
    }

    // Add and commit
    let status = run(&mut Command::new("git").args(["add", "--all"]))?;
    ensure_success(status, "Failed to stage files")?;

    let status = run(&mut Command::new("git").args(["commit", "-am", "wip 🚧: work-in-progress"]))?;
    ensure_success(status, "Unable to create WIP commit")?;

    // Determine rewind count
    let rewind = rewind.unwrap_or(1);
    let head_arg = format!("HEAD~{rewind}");

    // Soft reset and amend
    ensure_success(
        run(&mut Command::new("git").args(["reset", "--soft", &head_arg]))?,
        "git reset failed",
    )?;

    ensure_success(
        run(&mut Command::new("git").args(["commit", "--all", "--amend", "--no-edit"]))?,
        "git amend failed",
    )?;

    // Push unless suppressed
    if !no_push {
        ensure_success(
            run(&mut Command::new("git").args(["push", "-f"]))?,
            "git push failed",
        )?;
    }

    Ok(())
}

fn delete_stale_local_branches(confirm: bool) -> anyhow::Result<()> {
    // Open repo
    let repo = git2::Repository::discover(".")?;
    let git_config = git2::Config::open_default()?;
    let auth = auth_git2::GitAuthenticator::default();

    let remote_name = "origin";
    let mut remote = repo.find_remote(remote_name)?;

    // Build the callbacks
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(auth.credentials(&git_config));

    // Build FetchOptions and attach callbacks
    let mut fetch_options = git2::FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);
    fetch_options.prune(git2::FetchPrune::On);

    // Perform the fetch
    remote.fetch(
        &["+refs/heads/*:refs/remotes/origin/*"],
        Some(&mut fetch_options),
        None,
    )?;

    let mut branches_to_delete = Vec::new();
    let local_branches = repo.branches(Some(git2::BranchType::Local))?;

    for branch_res in local_branches {
        let (branch, _) = branch_res?;
        let branch_name = branch.name()?.unwrap_or("unknown").to_string();

        // Safety: Never delete current branch or main/master
        if branch.is_head() || branch_name == "main" || branch_name == "master" {
            continue;
        }

        // Logic: If there is no 'refs/remotes/origin/<name>', the remote is gone.
        // We check this directly instead of relying on the 'upstream' config,
        // which can be buggy if the remote was deleted via a Web UI.
        let remote_ref_path = format!("refs/remotes/{remote_name}/{branch_name}");

        if repo.find_reference(&remote_ref_path).is_err() {
            // Check if this branch ever even tried to track the remote.
            // If it has upstream config OR just matches the remote name, we prune it.
            let has_tracking_config = branch.upstream().is_ok();

            if has_tracking_config {
                println!(
                    "  - Branch '{}' tracks a deleted remote branch. Deleting...",
                    branch_name
                );
                branches_to_delete.push(branch_name);
            } else {
                use std::io::IsTerminal;

                if std::io::stdin().is_terminal() {
                    //cliclack::log::remark("This branch exists locally but not on origin/main.")?;
                    let ans = match confirm {
                        true => true,
                        false => {
                            cliclack::confirm(format!(
                                "Branch '{}' has no remote counterpart. Delete locally?",
                                branch_name
                            ))
                            .initial_value(false) // Default to 'No'
                            .interact()?
                        }
                    };

                    match ans {
                        true => branches_to_delete.push(branch_name),
                        false => cliclack::log::remark(format!(
                            "\x1b[90m  - Skipping '{}'.\x1b[0m",
                            branch_name
                        ))?,
                    };
                } else {
                    match confirm {
                        true => branches_to_delete.push(branch_name),
                        false => println!("\x1b[90m  - Skipping '{}'.\x1b[0m", branch_name),
                    };
                }
            }
        }
    }

    for name in branches_to_delete {
        let mut b = repo.find_branch(&name, git2::BranchType::Local)?;

        cliclack::log::remark(format!("\x1b[32m✔\x1b[0m Deleting branch '{}'.", name))?;
        b.delete()?;
    }

    Ok(())
}

pub fn prune(confirm: bool) -> anyhow::Result<()> {
    cliclack::intro("Cleaning up stale branches")?;

    match delete_stale_local_branches(confirm) {
        Ok(_) => cliclack::outro("You're all set!")?,
        Err(e) => {
            eprintln!("Error managing branches: {}", e);
            return Err(e);
        }
    };

    Ok(())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
