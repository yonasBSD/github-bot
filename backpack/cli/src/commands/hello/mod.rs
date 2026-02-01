use rootcause::hooks::Hooks;
use rootcause_backtrace::BacktraceCollector;
use tracing::instrument;

#[instrument(level = "debug", target = "errors::rootcause", name = "run")]
pub fn run() -> anyhow::Result<()> {
    // Capture backtraces for all errors
    // Install hooks only if they are not already installed (helps tests run multiple times)
    let _ = Hooks::new()
        .report_creation_hook(BacktraceCollector::new_from_env())
        .install();

    tracing::info!("Ping Pong");
    println!("Pong");

    Ok(())
}
