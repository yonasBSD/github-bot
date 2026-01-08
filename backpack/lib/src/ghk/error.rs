use thiserror::Error;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum GhkError {
    #[error("Git is not installed. Run 'ghk setup' first")]
    GitNotInstalled,

    #[error("GitHub CLI (gh) is not installed. Run 'ghk setup' first")]
    GhNotInstalled,

    #[error("Not logged in to GitHub. Run 'ghk login' first")]
    NotLoggedIn,

    #[error("Not a git repository. Run 'ghk init' first")]
    NotARepo,

    #[error("Already a git repository")]
    AlreadyARepo,

    #[error("No changes to save")]
    NothingToCommit,

    #[error("No remote configured. Run 'ghk create' first")]
    NoRemote,

    #[error("{cmd} failed: {reason}")]
    CommandFailed { cmd: String, reason: String },

    #[error("Cancelled by user")]
    Cancelled,
}
