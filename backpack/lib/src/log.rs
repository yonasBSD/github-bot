// Logger utilities

use crate::ghk::config;
use once_cell::sync::OnceCell;
use std::{cell::RefCell, sync::Arc, time::Instant};
use terminal_banner::Banner;
use tracing::{Level, debug, error, info, span, span::EnteredSpan, trace, warn};

/// A global, thread-safe screen logger.
///
/// Applications call `log().ok("message")` without needing to pass
/// logger instances around. This keeps the API clean and ergonomic.
///
/// The logger must be initialized once at startup using `set_logger()`.
static LOGGER: OnceCell<Arc<dyn ScreenLogger + Send + Sync>> = OnceCell::new();

/// Set the global logger.
///
/// This should be called once during program initialization.
/// Subsequent calls are ignored.
pub fn set_logger<L: ScreenLogger + Send + Sync + 'static>(logger: L) {
    let _ = LOGGER.set(Arc::new(logger));
}

/// Retrieve the global logger.
///
/// Panics if the logger has not been initialized.
/// Applications should call `set_logger()` early in `main()`.
pub fn log() -> &'static Arc<dyn ScreenLogger + Send + Sync> {
    LOGGER.get().expect("Logger not initialized")
}

pub fn init() {
    // 1. Define the formatted output (The Layer)
    let telemetry_fmt = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .without_time()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false);

    // 2. Define the first filter (Environment variable)
    let env_filter = EnvFilter::from_default_env();

    // --- This does not work ---
    // 3. Combine the filters: Apply both the environment filter AND the max level filter.
    // Note: When chaining filters (env_filter.and(max_level_filter)), the filter that
    // allows an event to pass is the intersection of both.
    //let combined_filter = env_filter.and(max_level_filter);

    // 4. Construct the registry, applying the format layer and the combined filter layer
    let registry = Registry::default()
        // Apply formatting layer, filtered by the combined filter
        //.with(telemetry_fmt.with_filter(combined_filter))
        .with(telemetry_fmt.with_filter(env_filter))
        // Send traces to tokio console
        .with(console_subscriber::spawn());

    tracing::subscriber::set_global_default(registry)?;

    tracing::debug!("Logging initialized!");
    tracing::trace!("Tracing initialized!");
    tracing::debug!("Ready to begin...");

    if std::env::var("RUST_LOG").is_ok()
        && ["debug", "trace"].contains(&std::env::var("RUST_LOG").unwrap().to_lowercase().as_str())
    {
        let banner = Banner::new()
            .text(format!("Welcome to {PROJECT_NAME}!").into())
            .text(PROJECT_DESC.into())
            .render();

        println!("{banner}");
    }
}

/// Cargo-style verbosity levels.
///
/// Quiet   → suppress almost all output
/// Normal  → standard CLI output (println)
/// Verbose → use tracing spans + debug-level logs
/// Trace   → full tracing, including trace-level logs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    Quiet,   // -q
    Normal,  // default
    Verbose, // -v
    Trace,   // -vv
}

/// Output format for the logger.
///
/// Text → human‑friendly CLI output (println / eprintln)
/// Json → machine‑friendly structured output (one JSON object per event)
///
/// JSON mode is ideal for CI logs, log aggregation, or tools that parse output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Text,
    Json,
}

/// A span that tracks when it was entered so we can compute
/// how long the task took when outro() / done() is called.
struct TimedSpan {
    /// The active tracing span (dropped to exit)
    entered: EnteredSpan,

    /// Timestamp when the span was entered
    start: Instant,
}

/// A logger that *only formats* messages into strings.
///
/// This trait provides:
///   - A set of `_raw` methods that concrete loggers must implement.
///   - Default high-level methods (`ok`, `warn`, etc.) that:
///         • automatically apply quiet-mode suppression
///         • wrap the raw formatting
///
/// The goal: avoid code duplication across loggers while keeping
/// formatting and quiet-mode logic cleanly separated.
pub trait FormatLogger {
    /// Returns true if output should be suppressed.
    /// Override if a logger wants custom quiet-mode behavior.
    fn is_quiet(&self) -> bool {
        config::isquiet()
    }

    /// Verbose mode flag (for debug/trace)
    fn is_verbose(&self) -> bool {
        config::isverbose()
    }

    /// Current verbosity level
    fn verbosity(&self) -> Verbosity;

    // ---------------------------------------------------------------------
    // RAW METHODS (must be implemented by each logger)
    // These return *unconditional* formatted strings with no quiet-mode logic.
    // ---------------------------------------------------------------------

    /// Format a success message (e.g., green checkmark)
    fn ok_raw(&self, m: &str) -> String;

    /// Format a warning message (e.g., yellow warning sign)
    fn warn_raw(&self, m: &str) -> String;

    /// Format an error message (always shown)
    fn err_raw(&self, m: &str) -> String;

    /// Format an informational message
    fn info_raw(&self, m: &str) -> String;

    /// Format a dim/muted message
    fn dim_raw(&self, m: &str) -> String;

    /// Format an intro message (start of a task)
    fn intro_raw(&self, m: &str) -> String;

    /// Format an outro message (end of a task)
    fn outro_raw(&self, m: &str) -> String;

    /// Format a done message (end of a task)
    fn done_raw(&self, m: &str) -> String;

    /// Format a step/progress message
    fn step_raw(&self, m: &str) -> String;

    /// Debug-level message (verbose only)
    fn debug_raw(&self, m: &str) -> String;

    /// Trace-level message (verbose only)
    fn trace_raw(&self, m: &str) -> String;

    // ---------------------------------------------------------------------
    // DEFAULT METHODS (quiet-mode aware)
    // These wrap the raw methods and apply quiet-mode suppression.
    // ---------------------------------------------------------------------

    /// Success message (suppressed in quiet mode)
    fn ok(&self, m: &str) -> Option<String> {
        if self.is_quiet() {
            None
        } else {
            Some(self.ok_raw(m))
        }
    }

    /// Warning message (suppressed in quiet mode)
    fn warn(&self, m: &str) -> Option<String> {
        if self.is_quiet() {
            None
        } else {
            Some(self.warn_raw(m))
        }
    }

    /// Error message (never suppressed)
    fn err(&self, m: &str) -> String {
        self.err_raw(m)
    }

    /// Info message (suppressed in quiet mode)
    fn info(&self, m: &str) -> Option<String> {
        if self.is_quiet() {
            None
        } else {
            Some(self.info_raw(m))
        }
    }

    /// Dim/muted message (suppressed in quiet mode)
    fn dim(&self, m: &str) -> Option<String> {
        if self.is_quiet() {
            None
        } else {
            Some(self.dim_raw(m))
        }
    }

    /// Intro message (suppressed in quiet mode)
    fn intro(&self, m: &str) -> Option<String> {
        if self.is_quiet() {
            None
        } else {
            Some(self.intro_raw(m))
        }
    }

    /// Outro message (suppressed in quiet mode)
    fn outro(&self, m: &str) -> Option<String> {
        if self.is_quiet() {
            None
        } else {
            Some(self.outro_raw(m))
        }
    }

    /// Done message (suppressed in quiet mode)
    fn done(&self, m: &str) -> Option<String> {
        if self.is_quiet() {
            None
        } else {
            Some(self.done_raw(m))
        }
    }

    /// Step/progress message (suppressed in quiet mode)
    fn step(&self, m: &str) -> Option<String> {
        if self.is_quiet() {
            None
        } else {
            Some(self.step_raw(m))
        }
    }

    /// Debug messages only appear when verbose mode is enabled.
    fn debug(&self, m: &str) -> Option<String> {
        if self.is_verbose() {
            Some(self.debug_raw(m))
        } else {
            None
        }
    }

    /// Trace messages only appear when verbose mode is enabled.
    fn trace(&self, m: &str) -> Option<String> {
        if self.is_verbose() {
            Some(self.trace_raw(m))
        } else {
            None
        }
    }
}

/// A simple ANSI-based logger that formats messages using
/// plain ASCII or ANSI escape codes depending on configuration.
///
/// This logger does *not* print anything — it only formats strings.
/// Quiet-mode suppression is handled by the FormatLogger trait.
pub struct SimpleLogger;

impl FormatLogger for SimpleLogger {
    fn ok_raw(&self, m: &str) -> String {
        // Green checkmark (or ASCII fallback)
        if config::isnocolor() {
            format!("+ {}", m)
        } else {
            format!("\x1b[32m✔\x1b[0m {}", m)
        }
    }

    fn warn_raw(&self, m: &str) -> String {
        // Yellow warning sign (or ASCII fallback)
        if config::isnocolor() {
            format!("! {}", m)
        } else {
            format!("\x1b[33m⚠\x1b[0m {}", m)
        }
    }

    fn err_raw(&self, m: &str) -> String {
        // Red X (always shown)
        if config::isnocolor() {
            format!("X {}", m)
        } else {
            format!("\x1b[31m✗\x1b[0m {}", m)
        }
    }

    fn info_raw(&self, m: &str) -> String {
        // Simple indented info line
        format!("  {}", m)
    }

    fn dim_raw(&self, m: &str) -> String {
        // Dim gray (or plain)
        if config::isnocolor() {
            format!("  {}", m)
        } else {
            format!("\x1b[90m  {}\x1b[0m", m)
        }
    }

    fn intro_raw(&self, m: &str) -> String {
        // Start of a task
        format!("→ {}", m)
    }

    fn outro_raw(&self, m: &str) -> String {
        // End of a task
        format!("✓ {}", m)
    }

    fn done_raw(&self, m: &str) -> String {
        // End of a task
        format!("✓ Done!", m)
    }

    fn step_raw(&self, m: &str) -> String {
        // Progress indicator
        if config::isnocolor() {
            format!("* {}", m)
        } else {
            format!("\x1b[36m⠿\x1b[0m {}", m)
        }
    }

    fn debug_raw(&self, m: &str) -> String {
        if config::isnocolor() {
            format!("[debug] {m}")
        } else {
            format!("\x1b[34m[debug]\x1b[0m {m}")
        }
    }

    fn trace_raw(&self, m: &str) -> String {
        if config::isnocolor() {
            format!("[trace] {m}")
        } else {
            format!("\x1b[90m[trace]\x1b[0m {m}")
        }
    }
}

/// A logger that *prints* messages to the screen.
///
/// This trait represents the high-level logging API that most users interact with.
/// It does not format messages itself — instead, it delegates to a `FormatLogger`
/// implementation and then prints the resulting strings.
///
/// All quiet-mode logic is handled in the FormatLogger layer.
pub trait ScreenLogger {
    /// Print a success message
    fn ok(&self, m: &str);

    /// Print a warning message
    fn warn(&self, m: &str);

    /// Print an error message (never suppressed)
    fn err(&self, m: &str);

    /// Print an informational message
    fn info(&self, m: &str);

    /// Print a dim/muted message
    fn dim(&self, m: &str);

    /// Print an intro message (start of a task)
    fn intro(&self, m: &str);

    /// Print an outro message (end of a task)
    fn outro(&self, m: &str);

    /// Print a done message (end of a task)
    fn done(&self, m: &str);

    /// Print a step/progress message
    fn step(&self, m: &str);

    /// Verbose-only debug message
    fn debug(&self, m: &str);

    /// Verbose-only trace message
    fn trace(&self, m: &str);
}

/// A screen logger that prints formatted messages and, in verbose/trace mode,
/// also emits structured tracing spans.
///
/// This logger supports:
///   - Nested task spans via intro() / outro() / done()
///   - Nested step spans inside tasks
///   - Timing of task spans
///   - Cargo-style verbosity levels
pub struct Printer<L: FormatLogger> {
    /// The underlying formatter (SimpleLogger, ModernLogger, etc.)
    inner: L,

    /// Stack of active task spans created by intro()
    tasks: RefCell<Vec<TimedSpan>>,

    /// Stack of active step spans created by step()
    steps: RefCell<Vec<EnteredSpan>>,

    /// Output format: Text (default) or Json
    ///
    /// When Json is selected:
    ///   - All log events are emitted as JSON objects
    ///   - stdout/stderr is still respected based on log level
    ///   - tracing spans are still created in verbose/trace mode
    format: LogFormat,
}

impl<L: FormatLogger> Printer<L> {
    /// Create a new printer with the given formatter and output format.
    pub fn new(inner: L, format: LogFormat) -> Self {
        Self {
            inner,
            tasks: RefCell::new(Vec::new()),
            steps: RefCell::new(Vec::new()),
            format,
        }
    }
}

impl<L: FormatLogger> ScreenLogger for Printer<L> {
    fn intro(&self, m: &str) {
        // Format intro message (suppressed in Quiet mode)
        if let Some(s) = self.inner.intro(m) {
            match (self.inner.verbosity(), self.format) {
                // JSON mode: emit structured event
                (_, LogFormat::Json) => {
                    self.emit_json("info", &s);
                }

                // Verbose/Trace: create a tracing span + timing
                (Verbosity::Verbose | Verbosity::Trace, LogFormat::Text) => {
                    // Create a new tracing span for this task
                    let sp = span!(Level::INFO, "task", message = %m);

                    // Enter the span and push it onto the task stack
                    let entered = sp.enter();
                    self.tasks.borrow_mut().push(TimedSpan {
                        entered,
                        start: Instant::now(),
                    });

                    // Emit intro message through tracing
                    info!("{s}");
                }

                // Normal mode: plain stdout
                _ => println!("{s}"),
            }
        }
    }

    fn outro(&self, m: &str) {
        // Format outro message (suppressed in Quiet mode)
        if let Some(s) = self.inner.outro(m) {
            match (self.inner.verbosity(), self.format) {
                // JSON mode: structured output
                (_, LogFormat::Json) => {
                    self.emit_json("info", &s);
                }

                // Verbose/Trace: close spans + timing
                (Verbosity::Verbose | Verbosity::Trace, LogFormat::Text) => {
                    // Close all step spans first (automatic cleanup)
                    let mut steps = self.steps.borrow_mut();
                    while let Some(step) = steps.pop() {
                        drop(step);
                    }

                    // Close the task span
                    if let Some(TimedSpan { entered, start }) = self.tasks.borrow_mut().pop() {
                        drop(entered); // exiting the span
                        let elapsed = start.elapsed();

                        // Emit outro message with timing
                        info!("{s} (took {:?})", elapsed);
                    }
                }

                // Normal mode
                _ => println!("{s}"),
            }
        }
    }

    fn done(&self, m: &str) {
        // Format done message (suppressed in Quiet mode)
        if let Some(s) = self.inner.done(m) {
            match (self.inner.verbosity(), self.format) {
                // JSON mode: structured output
                (_, LogFormat::Json) => {
                    self.emit_json("info", &s);
                }

                // Verbose/Trace: close spans + timing
                (Verbosity::Verbose | Verbosity::Trace, LogFormat::Text) => {
                    // Close all step spans first (automatic cleanup)
                    let mut steps = self.steps.borrow_mut();
                    while let Some(step) = steps.pop() {
                        drop(step);
                    }

                    // Close the task span
                    if let Some(TimedSpan { entered, start }) = self.tasks.borrow_mut().pop() {
                        drop(entered); // exiting the span
                        let elapsed = start.elapsed();

                        // Emit done message with timing
                        info!("{s} (took {:?})", elapsed);
                    }
                }

                // Normal mode
                _ => println!("{s}"),
            }
        }
    }

    fn step(&self, m: &str) {
        // Format step message (suppressed in Quiet mode)
        if let Some(s) = self.inner.step(m) {
            match (self.inner.verbosity(), self.format) {
                // JSON mode: structured output
                (_, LogFormat::Json) => {
                    self.emit_json("info", &s);
                }

                // Verbose/Trace: nested spans
                (Verbosity::Verbose | Verbosity::Trace, LogFormat::Text) => {
                    // Automatically close previous step span if one exists
                    if let Some(prev) = self.steps.borrow_mut().pop() {
                        drop(prev);
                    }

                    // Create a new step span
                    let sp = span!(Level::INFO, "step", message = %m);
                    let entered = sp.enter();

                    // Push onto step stack
                    self.steps.borrow_mut().push(entered);

                    // Emit step message through tracing
                    info!("{s}");
                }

                // Normal mode
                _ => println!("{s}"),
            }
        }
    }

    fn ok(&self, m: &str) {
        // Format success message (suppressed in Quiet mode)
        if let Some(s) = self.inner.ok(m) {
            match (self.inner.verbosity(), self.format) {
                // JSON mode
                (_, LogFormat::Json) => self.emit_json("info", &s),

                // Verbose/Trace: tracing event
                (Verbosity::Verbose | Verbosity::Trace, LogFormat::Text) => info!("{s}"),

                // Normal mode
                _ => println!("{s}"),
            }
        }
    }

    fn warn(&self, m: &str) {
        // Format warning message (suppressed in Quiet mode)
        if let Some(s) = self.inner.warn(m) {
            match (self.inner.verbosity(), self.format) {
                // JSON mode
                (_, LogFormat::Json) => self.emit_json("warn", &s),

                // Verbose/Trace
                (Verbosity::Verbose | Verbosity::Trace, LogFormat::Text) => warn!("{s}"),

                // Normal mode
                _ => println!("{s}"),
            }
        }
    }

    fn err(&self, m: &str) {
        // Errors are never suppressed
        let s = self.inner.err(m);

        match (self.inner.verbosity(), self.format) {
            // JSON mode
            (_, LogFormat::Json) => self.emit_json("error", &s),

            // Verbose/Trace
            (Verbosity::Verbose | Verbosity::Trace, LogFormat::Text) => error!("{s}"),

            // Normal mode
            _ => eprintln!("{s}"),
        }
    }

    fn info(&self, m: &str) {
        // Format info message (suppressed in Quiet mode)
        if let Some(s) = self.inner.info(m) {
            match (self.inner.verbosity(), self.format) {
                (_, LogFormat::Json) => self.emit_json("info", &s),
                (Verbosity::Verbose | Verbosity::Trace, LogFormat::Text) => info!("{s}"),
                _ => println!("{s}"),
            }
        }
    }

    fn dim(&self, m: &str) {
        // Dim messages (suppressed in Quiet mode)
        if let Some(s) = self.inner.dim(m) {
            match (self.inner.verbosity(), self.format) {
                (_, LogFormat::Json) => self.emit_json("debug", &s),
                (Verbosity::Verbose | Verbosity::Trace, LogFormat::Text) => debug!("{s}"),
                _ => println!("{s}"),
            }
        }
    }

    fn debug(&self, m: &str) {
        // Debug messages only appear in Verbose/Trace mode
        if let Some(s) = self.inner.debug(m) {
            match (self.inner.verbosity(), self.format) {
                (_, LogFormat::Json) => self.emit_json("debug", &s),
                (Verbosity::Verbose | Verbosity::Trace, LogFormat::Text) => debug!("{s}"),
                _ => println!("{s}"),
            }
        }
    }

    fn trace(&self, m: &str) {
        // Trace messages only appear in Trace mode
        if let Some(s) = self.inner.trace(m) {
            match (self.inner.verbosity(), self.format) {
                (_, LogFormat::Json) => self.emit_json("trace", &s),
                (Verbosity::Trace, LogFormat::Text) => trace!("{s}"),
                _ => println!("{s}"),
            }
        }
    }
}

impl<L: FormatLogger> Printer<L> {
    /// Emit a single log event as a JSON object.
    ///
    /// This function:
    ///   - Builds a structured JSON object using serde_json
    ///   - Includes a timestamp for easier log correlation
    ///   - Sends errors to stderr and all other levels to stdout
    ///   - Does NOT create tracing spans (that happens elsewhere)
    ///
    /// Example output:
    /// {
    ///   "level": "info",
    ///   "message": "Uploading assets",
    ///   "timestamp": "2026-01-15T01:17:22.123Z"
    /// }
    fn emit_json(&self, level: &str, message: &str) {
        // Construct a structured JSON object
        let obj = serde_json::json!({
            "level": level,
            "message": message,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        // Print to stderr for errors, stdout otherwise
        match level {
            "error" => eprintln!("{}", obj.to_string()),
            _ => println!("{}", obj.to_string()),
        }
    }
}

/// A modern, minimal logger inspired by cliclack's visual style.
///
/// This logger only *formats* messages — it does not print.
/// Quiet-mode suppression is handled by the FormatLogger trait.
pub struct ModernLogger;

impl FormatLogger for ModernLogger {
    fn ok_raw(&self, m: &str) -> String {
        // Clean success checkmark
        format!("✔ {}", m)
    }

    fn warn_raw(&self, m: &str) -> String {
        // Clean warning symbol
        format!("⚠ {}", m)
    }

    fn err_raw(&self, m: &str) -> String {
        // Clean error symbol
        format!("✗ {}", m)
    }

    fn info_raw(&self, m: &str) -> String {
        // Info symbol
        format!("ℹ {}", m)
    }

    fn dim_raw(&self, m: &str) -> String {
        // Muted remark-style prefix
        format!("› {}", m)
    }

    fn intro_raw(&self, m: &str) -> String {
        // Start of a task
        format!("→ {}", m)
    }

    fn outro_raw(&self, m: &str) -> String {
        // End of a task
        format!("✔ {}", m)
    }

    fn done_raw(&self, m: &str) -> String {
        // End of a task
        format!("✔ {}", m)
    }

    fn step_raw(&self, m: &str) -> String {
        // Progress indicator
        format!("⠿ {}", m)
    }

    fn debug_raw(&self, m: &str) -> String {
        format!("🔍 {}", m)
    }

    fn trace_raw(&self, m: &str) -> String {
        format!("… {}", m)
    }
}
