use crate::ghk::{git, util};
use anyhow::{Result, bail};

pub fn run(count: Option<usize>) -> Result<()> {
    if !git::isrepo() {
        util::err("Not a git repository");
        bail!("Not a git repository");
    }

    let n = count.unwrap_or(10);
    let commits = git::history(n)?;

    if commits.is_empty() {
        util::warn("No history yet");
        util::dim("Make some changes and run 'ghk push'");
        return Ok(());
    }

    println!();
    util::info("Recent saves:");
    for commit in &commits {
        util::dim(&format!("  {commit}"));
    }

    if commits.len() == n {
        let more_n = n * 2;
        util::dim(&format!("  ... use 'ghk history {more_n}' to see more"));
    }
    println!();

    Ok(())
}
