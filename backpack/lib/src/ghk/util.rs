use crate::ghk::config;

/// Print success message with green checkmark
pub fn ok(m: &str) {
    if config::isquiet() {
        return;
    }
    if config::isnocolor() {
        println!("+ {m}");
    } else {
        println!("\x1b[32m✔\x1b[0m {m}");
    }
}

/// Print warning message with yellow warning sign
pub fn warn(m: &str) {
    if config::isquiet() {
        return;
    }
    if config::isnocolor() {
        println!("! {m}");
    } else {
        println!("\x1b[33m⚠\x1b[0m {m}");
    }
}

/// Print error message with red X (always shown)
pub fn err(m: &str) {
    if config::isnocolor() {
        eprintln!("X {m}");
    } else {
        eprintln!("\x1b[31m✗\x1b[0m {m}");
    }
}

/// Print info message (no prefix)
pub fn info(m: &str) {
    if config::isquiet() {
        return;
    }
    println!("  {m}");
}

/// Print a dim/muted message
pub fn dim(m: &str) {
    if config::isquiet() {
        return;
    }
    if config::isnocolor() {
        println!("  {m}");
    } else {
        println!("\x1b[90m  {m}\x1b[0m");
    }
}
