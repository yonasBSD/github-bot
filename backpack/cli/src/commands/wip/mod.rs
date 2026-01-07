use rootcause::hooks::Hooks;
use rootcause_backtrace::BacktraceCollector;
use tracing::instrument;

use github_bot_lib::git;

#[instrument(level = "debug", target = "errors::rootcause", name = "run")]
pub fn run(no_push: bool, no_diff: bool, rewind: Option<u32>) -> anyhow::Result<()> {
    // Capture backtraces for all errors
    Hooks::new()
        .report_creation_hook(BacktraceCollector::new_from_env())
        .install()
        .expect("failed to install hooks");

    let _ = git::wip(no_push, no_diff, rewind);

    Ok(())
}
