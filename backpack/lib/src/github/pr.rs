use crate::github::{DEPENDABOT_USER, User, Client};
use serde::Deserialize;
use std::process::{Command, exit};

#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub user: User,
}

pub fn list_dependabot_prs(
    _client: &Client,
    repo: &str,
    _token: &str,
) -> Result<Vec<PullRequest>, Box<dyn std::error::Error + Send + Sync>> {
    let output = Command::new("gh")
        .args([
            "pr", "list",
            "--repo", repo,
            "--state", "open",
            "--author", DEPENDABOT_USER,
            "--json", "number,title,author",
        ])
        .output()?;

    if !output.status.success() {
        eprintln!("❌ `gh pr list` failed: {}", String::from_utf8_lossy(&output.stderr));
        exit(1);
    }

    #[derive(Deserialize)]
    struct RawPR {
        number: u64,
        title: String,
        author: RawAuthor,
    }

    #[derive(Deserialize)]
    struct RawAuthor {
        login: String,
    }

    let raw: Vec<RawPR> = serde_json::from_slice(&output.stdout)?;

    let prs = raw
        .into_iter()
        .map(|r| PullRequest {
            number: r.number,
            title: r.title,
            user: User { login: r.author.login },
        })
        .collect();

    Ok(prs)
}

pub fn process_pr(
    _client: &Client,
    repo: &str,
    _token: &str,
    pr: &PullRequest,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let pr_id = pr.number.to_string();

    let merged = merge_pr(repo, &pr_id);
    if merged {
        println!("✅ Successfully merged #{}", pr_id);
    } else {
        println!("❌ Failed to merge #{}", pr_id);
    }

    Ok(merged)
}

fn merge_pr(repo: &str, pr_id: &str) -> bool {
    println!("🚀 Merging PR #{}...", pr_id);

    let status = Command::new("gh")
        .args([
            "pr", "merge", pr_id,
            "--repo", repo,
            "--squash",
            "--delete-branch",
        ])
        .status()
        .expect("Failed to execute `gh pr merge`.");

    status.success()
}
