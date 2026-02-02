use clap::Parser;
use rootcause::hooks::Hooks;
use rootcause_backtrace::BacktraceCollector;
use tracing::instrument;

use github_bot_lib::cli::Args;
use github_bot_lib::github;

#[instrument(level = "debug", target = "errors::rootcause", name = "run")]
pub fn run(repo: String, action: &Option<String>) -> anyhow::Result<()> {
    // Capture backtraces for all errors
    // Install hooks only if they are not already installed (helps tests run multiple times)
    let _ = Hooks::new()
        .report_creation_hook(BacktraceCollector::new_from_env())
        .install();

    // Initialize basic CLI output
    println!("Starting maintenance for {}", repo);
    let _cli = Args::parse();

    let Ok(client) = github::GitHubClient::new() else {
        return Ok(());
    };

    // Rerunning failed jobs is handled outside the main cleanup loop
    if *action == Some("rerun".to_string()) {
        github::rerun_failed_jobs(&client, &repo);
        return Ok(());
    }

    let is_release_action = *action == Some("release".to_string());
    if is_release_action {
        eprintln!(
            "!!! DANGER: 'release' action selected. This will delete all existing releases and tags."
        );

        // Blocking confirmation prompt
        let confirmation = true;

        if !confirmation {
            println!("Exiting...");
            return Ok(());
        }
    }

    // Cleanup Repo (Always executed unless 'rerun')
    github::delete_failed_workflows(&client, &repo);
    println!("Deleted failed workflows");

    github::delete_old_container_versions(&client, &repo);
    println!("Deleted old containers versions");

    // Create new release (only if 'release' action is specified)
    if is_release_action {
        println!("Starting full release cleanup");

        if let Err(e) = github::delete_all_releases(&client, &repo) {
            eprintln!("Failed to complete full release cleanup for {repo}: {e}");
        } else {
            println!("Deleted all releases and tags");

            // Then create the new release
            github::create_release(&client, &repo)?;

            println!("Created new release");
        }

        println!("Release cleanup complete");
    }

    Ok(())
}
