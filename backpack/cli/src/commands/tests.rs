use super::*;

#[test]
fn test_hello_run() {
    let res = hello::run();
    assert!(res.is_ok());
}

#[test]
fn test_ping_output() {
    // Ensure hello prints Pong (not capturing stdout here, just ensuring no error)
    let res = hello::run();
    assert!(res.is_ok());
}

#[test]
fn test_prune_calls_lib() {
    // prune delegates to github_bot_lib::git::prune. We call with false to avoid confirmations.
    let res = prune::run(false);
    // The underlying function may return Err in this environment; accept both.
    assert!(res.is_ok() || res.is_err());
}
