use rootcause::hooks::Hooks;
use rootcause_backtrace::BacktraceCollector;
use tracing::instrument;

#[instrument(level = "debug", target = "errors::rootcause", name = "run")]
pub fn run() -> anyhow::Result<()> {
    // Capture backtraces for all errors
    Hooks::new()
        .report_creation_hook(BacktraceCollector::new_from_env())
        .install()
        .expect("failed to install hooks");

    let args = Args::parse();

    // Get commit SHA
    let commit = match args.commit {
        Some(c) => c,
        None => {
            println!("No commit specified, using latest commit...");
            get_latest_commit()?
        }
    };

    println!("Using commit: {}", commit);

    // Get repository
    let repo = match args.repo {
        Some(r) => r,
        None => {
            println!("No repository specified, detecting from git remote...");
            get_repo_from_git()?
        }
    };

    println!("Repository: {}\n", repo);

    // Get workflow runs for the commit
    println!("Fetching workflow runs...");
    let runs = get_workflow_runs(&args.token, &repo, &commit).await?;

    if runs.is_empty() {
        println!("No workflow runs found for this commit.");
        return Ok(());
    }

    // Filter for failed runs
    let failed_runs: Vec<_> = runs
        .iter()
        .filter(|run| {
            run.conclusion.as_deref() == Some("failure")
                || run.conclusion.as_deref() == Some("timed_out")
                || run.conclusion.as_deref() == Some("cancelled")
        })
    .collect();

    if failed_runs.is_empty() {
        println!("No failed workflow runs found for this commit.");
        println!("\nAll workflows:");
        for run in &runs {
            println!(
                "  - {} ({}): {:?}",
                run.name, run.status, run.conclusion
            );
        }
        return Ok(());
    }

    println!("Found {} failed workflow run(s):\n", failed_runs.len());

    for run in &failed_runs {
        println!("  - {} (ID: {})", run.name, run.id);
        println!("    Status: {}", run.status);
        println!("    Conclusion: {:?}", run.conclusion);
        println!("    URL: {}\n", run.html_url);
    }

    // Re-run failed workflows
    println!("Re-running failed workflows...\n");
    for run in &failed_runs {
        print!("Re-running '{}'... ", run.name);
        match rerun_workflow(&args.token, &repo, run.id).await {
            Ok(_) => println!("✓ Success"),
            Err(e) => println!("✗ Failed: {}", e),
        }
    }

    println!("\nDone!");
    Ok(())
}
