use clap::Parser;
use colored::Colorize;
use rootcause::hooks::Hooks;
use rootcause_backtrace::BacktraceCollector;
use tracing::instrument;

use github_bot_lib::cli::Args;
use github_bot_lib::github;

#[instrument(level = "debug", target = "errors::rootcause", name = "run")]
pub fn run(repos: &Vec<String>, action: &Option<String>) -> anyhow::Result<()> {
    // Capture backtraces for all errors
    Hooks::new()
        .report_creation_hook(BacktraceCollector::new_from_env())
        .install()
        .expect("failed to install hooks");

    let _cli = Args::parse();

    let Ok(client) = github::GitHubClient::new() else {
        return Ok(());
    };

    // Rerunning failed jobs is handled outside the main cleanup loop
    if *action == Some("rerun".to_string()) {
        for repo in repos {
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

    for repo in repos {
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
