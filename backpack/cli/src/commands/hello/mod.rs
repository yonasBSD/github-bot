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

    tracing::info!("Ping Pong");
    println!("Pong");

    Ok(())
}
