use clap::Parser;

/// Defines the structure for command-line arguments using `clap`.
#[derive(Parser, Debug)]
#[command(author, version, about = "Automate merging of Dependabot PRs.", long_about = None)]
pub struct Args {
    /// GitHub repository owner (e.g., 'rust-lang')
    #[arg(short, long)]
    pub owner: String,

    /// GitHub repository name (e.g., 'cargo')
    #[arg(short, long)]
    pub repo: String,

    /// Optional GitHub Personal Access Token (PAT) with 'repo' scope.
    /// If not provided, the program will look for the GITHUB_TOKEN environment variable.
    #[arg(short, long)]
    pub token: Option<String>,
}
