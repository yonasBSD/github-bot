mod app;
mod commands;
mod config;
mod error;
mod gh;
mod git;
mod util;

use crate::cli::Args;

pub fn main(cli: Args) -> anyhow::Result<()> {
    app::run(cli)
}
