use crate::cli::Args;

use clap::CommandFactory;
use clap_complete::{Shell, generate};
use std::io;

pub fn run(shell: Shell) {
    let mut cmd = Args::command();
    generate(shell, &mut cmd, "github-bot", &mut io::stdout());
}
