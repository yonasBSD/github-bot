use std::{
    path::Path,
    process::{Command, ExitStatus},
};

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
    let output = Command::new("git").args(["status", "-s"]).output()?;

    if output.stdout.is_empty() {
        // Nothing to do
        return Ok(());
    }

    // Format code
    // Check for treefmt config before running treefmt
    let has_treefmt_config =
        Path::new("treefmt.toml").exists() || Path::new(".treefmt.toml").exists();

    if has_treefmt_config {
        ensure_success(run(&mut Command::new("treefmt"))?, "treefmt failed")?;
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
    let head_arg = format!("HEAD~{}", rewind);

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
