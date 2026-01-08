use clap::Parser;
use rootcause::hooks::Hooks;
use rootcause_backtrace::BacktraceCollector;
use tracing::instrument;

use github_bot_lib::cli::Args;
use github_bot_lib::ghk;

#[instrument(level = "debug", target = "errors::rootcause", name = "run")]
pub fn run() -> anyhow::Result<()> {
    // Capture backtraces for all errors
    Hooks::new()
        .report_creation_hook(BacktraceCollector::new_from_env())
        .install()
        .expect("failed to install hooks");

    let args = Args::parse();
    ghk::main(args)
}
