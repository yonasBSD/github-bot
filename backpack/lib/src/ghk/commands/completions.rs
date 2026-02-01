use crate::cli::Args;
use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{Shell, generate};
use std::io;

pub fn run(shell: Shell) -> Result<()> {
    let mut cmd = Args::command();
    generate(shell, &mut cmd, "github-bot", &mut io::stdout());
    Ok(())
}
