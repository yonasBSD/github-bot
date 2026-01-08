use crate::ghk::{git, util};
use anyhow::Result;

pub fn run() -> Result<()> {
    if git::isrepo() {
        util::warn("Already a git repository");
        util::dim("Your project folder is already set up");
    } else {
        git::init()?;
        util::ok("Project folder ready");
        util::dim("Created .git folder to track your changes");
    }
    Ok(())
}
