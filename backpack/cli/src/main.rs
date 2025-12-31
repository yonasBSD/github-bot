use anyhow::{Context, Result};
use clap::Parser;
use reqwest::blocking::Client;

#[cfg(not(target_arch = "wasm32"))]
use human_panic::{metadata, setup_panic};

use github_bot_lib::cli::Args;
use github_bot_lib::github;

fn main() -> Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    setup_panic!(
        metadata!()
            .authors("Acme Inc. <support@example.com")
            .homepage("www.example.com")
            .support("- Open a support request by email to support@example.com")
    );

    // 1. Parse command-line arguments
    let args = Args::parse();

    println!("--- Dependabot PR Auto-Processor ---");
    println!("Target: {}/{}", args.owner, args.repo);

    // 2. Determine the authentication token
    let token = match args.token {
        Some(t) => t,
        None => std::env::var("GITHUB_TOKEN")
            .context("Error: GitHub token not found. Please provide it via the --token argument or set the GITHUB_TOKEN environment variable.")?,
    };

    // 3. Initialize the blocking HTTP client
    let client = Client::builder().build()?;

    // 4. List and filter Dependabot PRs
    let dependabot_prs = github::list_dependabot_prs(&client, &args.owner, &args.repo, &token)?;

    if dependabot_prs.is_empty() {
        println!("\n✅ No open Dependabot PRs found. Exiting.");
        return Ok(());
    }

    println!(
        "\nFound {} open Dependabot PRs. Starting processing...",
        dependabot_prs.len()
    );

    // 5. Process each PR
    for pr in dependabot_prs {
        println!("\nProcessing PR #{}: {}", pr.number, pr.title);
        // We ignore the individual result of process_pr to ensure we try all PRs.
        let _ = github::process_pr(&client, &args.owner, &args.repo, &token, &pr);
    }

    println!("\n--- Processing Complete ---");

    Ok(())
}
