use clap::Parser;
use reqwest::blocking::Client;
use rootcause::hooks::Hooks;
use rootcause::prelude::*;
use rootcause_backtrace::BacktraceCollector;
use tracing::instrument;

use github_bot_lib::{
    cli::Args,
    utils::get_repo,
    github,
};

#[instrument(level = "debug", target = "errors::rootcause", name = "run")]
pub fn run(target: Option<String>) -> anyhow::Result<()> {
    // Capture backtraces for all errors
    // Install hooks only if they are not already installed (helps tests run multiple times)
    let _ = Hooks::new()
        .report_creation_hook(BacktraceCollector::new_from_env())
        .install();

    // 1. Parse command-line arguments
    let cli = Args::parse();

    // 2. Determine the authentication token
    let token = match cli.token {
        Some(t) => t,
        None => std::env::var("GITHUB_TOKEN")
            .context("Missing Token")
            .attach("Please provide the token via --token or set the GITHUB_TOKEN environment variable.")
            .map_err(|report| anyhow::anyhow!("{report}"))?, // Manually convert Report to anyhow::Error
    };

    // Get target repo
    let repo = get_repo(target)?;

    // Determine repo to merge
    println!("--- Dependabot PR Auto-Processor ---");
    println!("Target: {repo}");

    // 3. Initialize the blocking HTTP client
    let client = Client::builder().build()?;

    // 4. List and filter Dependabot PRs
    let dependabot_prs = github::list_dependabot_prs(&client, &repo, &token)?;

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
        let _ = github::process_pr(&client, &repo, &token, &pr);
    }

    println!("\n--- Processing Complete ---");

    Ok(())
}
