mod commands;

use clap::Parser;
use commands::{maintain, merge};
use github_bot_lib::cli::{Args, Commands};
use std::env;

#[cfg(not(target_arch = "wasm32"))]
use human_panic::{metadata, setup_panic};

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    setup_panic!(
        metadata!()
            .authors("Acme Inc. <support@example.com")
            .homepage("www.example.com")
            .support("- Open a support request by email to support@example.com")
    );

    let cli = Args::parse();

    match &cli.command {
        Commands::Maintain { repo, action } => {
            let _ = maintain::run(repo, action);
        }

        Commands::Merge { owner, repo } => {
            let _ = merge::run(owner, repo);
        }
    }
}
