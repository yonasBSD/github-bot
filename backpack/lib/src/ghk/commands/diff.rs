use crate::ghk::{git, util};
use anyhow::{Result, bail};
use git2::{Repository, DiffOptions};

pub fn run() -> Result<()> {
    if !git::isrepo() {
        util::err("Not a git repository");
        bail!("Not a git repository");
    }

    let changes = git::haschanges()?;
    if !changes {
        util::ok("No changes");
        return Ok(());
    }

    // Run git diff with color
    let repo = Repository::open(".")?;

    let mut opts = DiffOptions::new();
    let diff = repo.diff_index_to_workdir(None, Some(&mut opts))?;

    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let text = std::str::from_utf8(line.content()).unwrap();
        let origin = line.origin();

        match origin {
            '+' => print!("\x1b[32m{text}\x1b[0m"), // addition
            '-' => print!("\x1b[31m{text}\x1b[0m"), // deletion
            ' ' => print!("{text}"),                // context

            // hunk header
            'H' => print!("\x1b[1;36m{text}\x1b[0m"),

            // file header
            'F' => print!("\x1b[1;35m{text}\x1b[0m"),

            // metadata (index, mode changes, etc.)
            'B' | 'M' => print!("\x1b[33m{text}\x1b[0m"),

            // fallback
            _ => print!("{text}"),
        }

        true
    })?;

    println!();

    // Show summary
    let files = git::changedfiles()?;
    util::info(&format!("{} file(s) changed", files.len()));

    Ok(())
}
