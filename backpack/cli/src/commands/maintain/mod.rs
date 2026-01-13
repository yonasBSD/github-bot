use clap::Parser;
use colored::Colorize;
use rootcause::hooks::Hooks;
use rootcause_backtrace::BacktraceCollector;
use tracing::instrument;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use github_bot_lib::cli::Args;
use github_bot_lib::github;
use github_bot_lib::log::{
    Printer,
    SimpleLogger,
    Verbosity,
    LogFormat,
    ScreenLogger,
};

#[instrument(level = "debug", target = "errors::rootcause", name = "run")]
pub fn run(repo: String, action: &Option<String>) -> anyhow::Result<()> {
    // Capture backtraces for all errors
    Hooks::new()
        .report_creation_hook(BacktraceCollector::new_from_env())
        .install()
        .expect("failed to install hooks");

    let formatter = ModernLogger::new(verbosity);
    let logger = Printer::new(formatter, format);

    //std::thread::sleep(std::time::Duration::from_millis(150));

    log().intro(format!("Starting maintenance for {repo}"));

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
        log().warning("!!! DANGER: 'release' action selected. This will delete all existing releases and tags.".red().bold());

        // Blocking confirmation prompt
        let confirmation =
            confirm("Are you absolutely sure you want to proceed with 'release' cleanup?")
                .interact()?;

        if !confirmation {
            log().outro("Exiting...");
            return Ok(());
        }
    }

    log().sucess(format!("Deleting branch '{}'.", name))?;

    // Cleanup Repo (Always executed unless 'rerun')
    github::delete_failed_workflows(&client, &repo);
    log().success("Deleted failed workflows");

    github::delete_old_container_versions(&client, &repo);
    log().success("Deleted old containers versions");

    // Create new release (only if 'release' action is specified)
    if is_release_action {
        log().intro("Starting full release cleanup");

        match github::delete_all_releases(&client, &repo) {
            Err(e) => {
                log().err(format!(
                    "Failed to complete full release cleanup for {repo}: {e}"
                ));
            }
            Ok(_) => {
                log().ok("Deleted all releases and tags");

                // Then create the new release
                github::create_release(&client, &repo)?;

                log().ok("Created new release");
            }
        }

        log().outro("Release cleanup complete");
    }

    Ok(())
}
