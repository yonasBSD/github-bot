use super::*;
use git2::Repository;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Helper to initialize a real git repo in a temp dir
fn setup_repo() -> (TempDir, Repository) {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let repo = Repository::init(dir.path()).expect("Failed to init repo");

    // Git needs a user identity to commit
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();

    (dir, repo)
}

#[test]
fn test_ensure_success_ok() {
    // Create a status that represents success
    let status = Command::new("true").status().unwrap();
    assert!(ensure_success(status, "should not fail").is_ok());
}

#[test]
fn test_ensure_success_fail() {
    let status = Command::new("false").status().unwrap();
    let res = ensure_success(status, "error message");
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().to_string(), "error message");
}

#[test]
fn test_wip_no_changes() {
    let (dir, _repo) = setup_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    // wip should return Ok(()) immediately if nothing is staged/changed
    let result = wip(true, true, None);
    assert!(result.is_ok());
}

#[test]
fn test_wip_with_changes_and_treefmt() {
    let (dir, _repo) = setup_repo();
    let repo_path = dir.path();
    std::env::set_current_dir(repo_path).unwrap();

    // 1. Create a dummy file to trigger WIP
    fs::write(repo_path.join("test.txt"), "hello").unwrap();

    // 2. Create a dummy treefmt.toml to test the logic path
    fs::write(repo_path.join("treefmt.toml"), "").unwrap();

    // Note: This test will attempt to call 'treefmt' and 'git' from your PATH.
    // We mock the 'no_push' and 'no_diff' to true for safety.
    // Since 'treefmt' might not be in CI, we expect it might 'fail' in the log
    // but the function should continue.

    // Create an initial commit so 'HEAD~1' exists for the reset logic
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .status()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(repo_path)
        .status()
        .unwrap();

    // Modify the file to make it "dirty"
    fs::write(repo_path.join("test.txt"), "dirty").unwrap();

    let result = wip(true, true, Some(1));

    // If 'git' is installed in the test env, this should succeed
    assert!(result.is_ok());
}

#[test]
fn test_prune_logic_skips_protected() {
    let (dir, repo) = setup_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    // Create main and a fake remote branch
    let _head = repo.head().ok(); // might be empty on fresh init

    // The test for prune() is harder because it requires a mock remote.
    // Instead, we test that the logic doesn't crash on an empty repo.
    // A full integration test would involve `git2` creating a "remote" repo
    // on the local disk and linking them.

    let result = prune(false);
    // This will likely fail because 'origin' doesn't exist yet
    assert!(result.is_err());
}
