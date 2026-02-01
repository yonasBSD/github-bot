use rootcause::hooks::Hooks;
use rootcause_backtrace::BacktraceCollector;
use tracing::instrument;

use github_bot_lib::git;

#[instrument(level = "debug", target = "errors::rootcause", name = "run")]
pub fn run(no_push: bool, no_diff: bool, rewind: Option<u32>) -> anyhow::Result<()> {
    // Capture backtraces for all errors
    // Install hooks only if they are not already installed (helps tests run multiple times)
    let _ = Hooks::new()
        .report_creation_hook(BacktraceCollector::new_from_env())
        .install();

    let _ = git::wip(no_push, no_diff, rewind);

    Ok(())
}
