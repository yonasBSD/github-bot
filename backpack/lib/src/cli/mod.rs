use clap::{Parser, Subcommand};

/// Automate merging and maintenance of Dependabot PRs.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Optional GitHub Personal Access Token (PAT) with 'repo' scope.
    /// If not provided, the program will look for the `GITHUB_TOKEN` environment variable.
    #[arg(short, long, global = true)]
    pub token: Option<String>,

    #[command(flatten)]
    pub verbosity: clap_verbosity_flag::Verbosity,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Maintain one or more repositories (cleanup, rerun, or release)
    Maintain {
        /// The GitHub repository (or repositories) to maintain (e.g., owner/repo).
        #[arg(short, long, default_value = "yonasBSD/github-rs")]
        repo: String,

        /// Specific action to perform: 'rerun' failed jobs, 'release' (clean and create v0.1.0), or no action for cleanup.
        #[arg(required = false)]
        action: Option<String>,
    },

    /// Merge Dependabot PRs for a specific repository
    Merge {
        /// The GitHub repository (or repositories) to maintain (e.g., owner/repo).
        #[arg(short, long, default_value = "yonasBSD/github-rs")]
        repo: String,
    },

    /// Ping test
    Hello {},
}
