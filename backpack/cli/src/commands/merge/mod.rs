use anyhow::{Context, Result};
use clap::Parser;
use reqwest::blocking::Client;

use github_bot_lib::cli::Args;
use github_bot_lib::github;

pub fn run(owner: &String, repo: &String) -> Result<()> {
    // 1. Parse command-line arguments
    let cli = Args::parse();

    println!("--- Dependabot PR Auto-Processor ---");
    println!("Target: {owner}/{repo}");

    // 2. Determine the authentication token
    let token = match cli.token {
        Some(t) => t,
        None => std::env::var("GITHUB_TOKEN")
            .context("Error: GitHub token not found. Please provide it via the --token argument or set the GITHUB_TOKEN environment variable.")?,
    };

    // 3. Initialize the blocking HTTP client
    let client = Client::builder().build()?;

    // 4. List and filter Dependabot PRs
    let dependabot_prs = github::list_dependabot_prs(&client, owner, repo, &token)?;

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
        let _ = github::process_pr(&client, owner, repo, &token, &pr);
    }

    println!("\n--- Processing Complete ---");

    Ok(())
}
