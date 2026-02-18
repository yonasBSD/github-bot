use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use git2::Repository;
use strum::Display;

/// Automate merging and maintenance of Dependabot PRs.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(name = "github-bot")]
#[command(about = "GitHub automation bot")]
pub struct Args {
    /// Optional GitHub Personal Access Token (PAT) with 'repo' scope.
    /// If not provided, the program will look for the `GITHUB_TOKEN` environment variable.
    #[arg(short, long, global = true)]
    pub token: Option<String>,

    /// Suppress output (errors still shown)
    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[command(flatten)]
    pub verbosity: Option<clap_verbosity_flag::Verbosity>,

    /// Disable colored output
    #[arg(long, global = true)]
    pub nocolor: bool,

    #[command(subcommand)]
    pub command: Commands,
}

impl Args {
    /// The "Smart Default" logic.
    /// Priority: 1. CLI Argument, 2. Git Discovery, 3. Hardcoded Fallback
    #[must_use]
    pub fn resolve_repo(provided: &Option<String>) -> String {
        if let Some(repo) = provided {
            return repo.clone();
        }

        if let Some(detected) = Self::detect_git_repo() {
            return detected;
        }

        // Final fallback
        "yonasBSD/github-rs".to_string()
    }

    fn detect_git_repo() -> Option<String> {
        let repo = Repository::discover(std::env::current_dir().ok()?).ok()?;
        let remote = repo.find_remote("origin").ok()?;
        let url = remote.url()?;

        if url.contains("github.com") {
            let parts: Vec<&str> = url
                .trim_end_matches(".git")
                .split(&['/', ':'][..])
                .collect();
            if parts.len() >= 2 {
                let repo_name = parts.last()?;
                let owner = parts.get(parts.len() - 2)?;
                return Some(format!("{owner}/{repo_name}"));
            }
        }
        None
    }
}

#[derive(Subcommand, Debug, Display)]
#[strum(serialize_all = "lowercase")]
pub enum Commands {
    /// Maintain one or more repositories (cleanup, rerun, or release)
    Maintain {
        /// The GitHub repository (e.g., owner/repo). If omitted, detects from local git origin.
        #[arg(short, long)]
        repo: String,

        /// Specific action to perform: 'rerun' failed jobs, 'release' (clean and create v0.1.0), or no action for cleanup.
        #[arg(required = false)]
        action: Option<String>,
    },

    /// Merge Dependabot PRs for a specific repository
    Merge {
        /// The GitHub repository (e.g., owner/repo). If omitted, detects from local git origin.
        #[arg(short, long)]
        repo: String,
    },

    /// Work-in-progress commit helper. Push all uncommitted changes using the last commit.
    Wip {
        /// Do not push after amending
        #[arg(long = "no-push")]
        no_push: bool,

        /// Do not show diff before committing
        #[arg(long = "no-diff")]
        no_diff: bool,

        /// Optional number of commits to rewind (default: 1)
        rewind: Option<u32>,
    },

    /// Prune local branches that don't exist remotely
    Prune {
        /// Answer yes to all confirmations
        #[arg(short, long)]
        yes: bool,
    },

    /// Simple GitHub helper. Push code without the complexity.
    Git {
        #[command(subcommand)]
        command: GitCommands,
    },

    /// Ping test
    Hello,
}

#[derive(Subcommand, Debug, Display)]
#[strum(serialize_all = "lowercase")]
pub enum GitCommands {
    /// Check and install requirements
    Setup,

    /// Start tracking this folder
    Init,

    /// Connect to GitHub
    Login,

    /// Disconnect from GitHub
    Logout,

    /// Manage GitHub accounts
    User {
        #[command(subcommand)]
        command: UserCmd,
    },

    /// Create a repository on GitHub
    Create,

    /// Fork a repository on GitHub
    Fork {
        /// Repository (owner/name or URL)
        repo: Option<String>,
    },

    /// Save changes to GitHub
    Push,

    /// Alias for push
    #[command(hide = true)]
    Save,

    /// Download changes from GitHub
    Pull,

    /// Alias for pull
    #[command(hide = true)]
    Sync,

    /// Download a repository
    Clone {
        /// Repository (owner/name or URL)
        repo: Option<String>,
        /// Directory to clone into
        dir: Option<String>,
    },

    /// Alias for clone
    #[command(hide = true)]
    Download {
        repo: Option<String>,
        dir: Option<String>,
    },

    /// Show current status
    Status,

    /// Preview changes before saving
    Diff,

    /// Undo last commit (keeps changes)
    Undo,

    /// Show recent saves
    History {
        /// Number of commits to show
        #[arg(default_value = "10")]
        count: Option<usize>,
    },

    /// Alias for history
    #[command(hide = true)]
    Log {
        #[arg(default_value = "10")]
        count: Option<usize>,
    },

    /// Open repository in browser
    Open,

    /// View or edit settings
    Config {
        /// Setting to view/edit
        key: Option<String>,
        /// New value
        value: Option<String>,
    },

    /// Add .gitignore template
    Ignore {
        /// Template name (node, python, rust, go, etc)
        template: Option<String>,
    },

    /// Add a license file
    License {
        /// License type
        #[arg(value_enum)]
        kind: Option<LicenseKind>,
    },

    /// List or switch branches
    Branch {
        /// Branch to switch to
        name: Option<String>,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Subcommand, Debug, Display)]
#[strum(serialize_all = "lowercase")]
pub enum UserCmd {
    /// Show logged in accounts
    List,

    /// Switch to a different account
    Switch {
        /// GitHub username to switch to
        name: String,
    },
}

#[derive(Clone, Debug, Display, ValueEnum)]
#[strum(serialize_all = "lowercase")]
pub enum LicenseKind {
    Mit,
    Apache,
    Gpl,
    Unlicense,
}
