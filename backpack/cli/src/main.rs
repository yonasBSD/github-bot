use anyhow::{Context, Result};
use clap::Parser;
use reqwest::blocking::Client;

use std::env;

use colored::Colorize;

#[cfg(not(target_arch = "wasm32"))]
use human_panic::{metadata, setup_panic};

use github_bot_lib::cli::{Args, Commands};
use github_bot_lib::github;

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
            let _ = maintain(repo, action);
        }

        Commands::Merge { owner, repo } => {
            let _ = merge(owner, repo);
        }
    }
}

fn maintain(repo: &Vec<String>, action: &Option<String>) -> Result<()> {
    let _cli = Args::parse();

    let Ok(client) = github::GitHubClient::new() else {
        return Ok(());
    };

    // Rerunning failed jobs is handled outside the main cleanup loop
    if *action == Some("rerun".to_string()) {
        for repo in repo {
            github::rerun_failed_jobs(&client, repo);
        }
        return Ok(());
    }

    let is_release_action = *action == Some("release".to_string());
    if is_release_action {
        println!("{}", "!!! DANGER: 'release' action selected. This will delete all existing releases and tags.".red().bold());

        // Blocking confirmation prompt
        let confirmation = dialoguer::Confirm::new()
            .with_prompt("Are you absolutely sure you want to proceed with 'release' cleanup?")
            .interact()
            .unwrap_or(false);

        if !confirmation {
            println!("{}", "Exiting...".red());
            return Ok(());
        }
    }

    for repo in repo {
        println!(
            "{}",
            format!("\n--- Starting maintenance for {repo} ---")
                .cyan()
                .bold()
        );

        // Cleanup Repo (Always executed unless 'rerun')
        github::delete_failed_workflows(&client, repo);
        github::delete_old_container_versions(&client, repo);

        println!("{}", "Deleted failed workflows.".green());
        println!("{}", "Deleted old containers versions.".green());
        println!();

        // Create new release (only if 'release' action is specified)
        if is_release_action {
            // Delete all releases and tags first
            if let Err(e) = github::delete_all_releases(&client, repo) {
                eprintln!(
                    "{}",
                    format!("Failed to complete full release cleanup for {repo}: {e}").red()
                );
                continue;
            }

            // Then create the new release
            github::create_release(&client, repo)?;
        }
    }

    Ok(())
}

fn merge(owner: &String, repo: &String) -> Result<()> {
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
