use anyhow::{Context, Result};
use reqwest::blocking::{Client, Response};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use serde::Deserialize;
use std::thread;
use std::time::Duration;

// --- Constants ---
pub const DEPENDABOT_USER: &str = "dependabot[bot]";
pub const GITHUB_API_BASE: &str = "https://api.github.com";
pub const MAX_MERGE_ATTEMPTS: u8 = 2;
pub const UPDATE_WAIT_SECS: u64 = 5;

// --- GitHub API Data Structures ---

#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct User {
    pub login: String,
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub user: User,
}

#[derive(Deserialize, Debug)]
pub struct MergeResponse {
    pub message: Option<String>,
    pub sha: Option<String>,
}

// --- GitHub API Functions ---

/// Lists all open PRs and filters them to only include those created by Dependabot.
pub fn list_dependabot_prs(
    client: &Client,
    owner: &str,
    repo: &str,
    token: &str,
) -> Result<Vec<PullRequest>> {
    let url = format!("{GITHUB_API_BASE}/repos/{owner}/{repo}/pulls?state=open&per_page=100");

    let response = client
        .get(&url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(ACCEPT, "application/vnd.github.v3+json")
        .header(USER_AGENT, "DependabotAutoMerger")
        .send()
        .context("Failed to send list PRs request")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        eprintln!("GitHub API Error (List PRs): Status {status}, Body: {body}");
        return Err(anyhow::anyhow!(
            "Failed to list PRs from GitHub API. Check token scope."
        ));
    }

    let all_prs: Vec<PullRequest> = response
        .json()
        .context("Failed to parse list PRs response")?;

    let dependabot_prs: Vec<PullRequest> = all_prs
        .into_iter()
        .filter(|pr| pr.user.login == DEPENDABOT_USER)
        .collect();

    Ok(dependabot_prs)
}

/// Core function to attempt merge, handle stale branch errors, and retry.
pub fn process_pr(
    client: &Client,
    owner: &str,
    repo: &str,
    token: &str,
    pr: &PullRequest,
) -> Result<()> {
    for attempt in 1..=MAX_MERGE_ATTEMPTS {
        // 1. Attempt to merge the PR
        let merge_response = attempt_merge(client, owner, repo, token, pr).context(format!(
            "Failed to send merge request for PR #{}",
            pr.number
        ))?;

        if merge_response.status().is_success() {
            let response_body: MergeResponse = merge_response.json()?;
            println!(
                "  ✅ Successfully MERGED. Commit SHA: {}",
                response_body.sha.unwrap_or_else(|| "N/A".to_string())
            );
            return Ok(());
        }

        let error_message = merge_response.json::<MergeResponse>().map_or_else(
            |_| "Failed to parse error response".to_string(),
            |r| r.message.unwrap_or_else(|| "Unknown API Error".to_string()),
        );

        // 2. Handle failure based on reason
        if error_message.contains("Base branch was modified") {
            println!("  ⚠️ Merge FAILED (Attempt {attempt}). Reason: Base branch modified.");

            if attempt == MAX_MERGE_ATTEMPTS {
                println!("  ⏭️ Final attempt failed. Skipping PR (leaving open).");
                return Ok(());
            }

            // Otherwise, attempt to update the branch and retry
            if update_pr_branch(client, owner, repo, token, pr)? {
                continue; // Continue to the next iteration (retry)
            }
            println!("  ⏭️ Branch update failed. Skipping PR (leaving open).");
            return Ok(());
        }
        // Other merge failures (e.g., CI failure, conflicts, etc.)
        println!("  ⏭️ Merge FAILED. Reason: {error_message}. Skipping PR (leaving open).");
        return Ok(());
    }

    Ok(())
}

/// Performs the PUT request to merge the PR.
pub fn attempt_merge(
    client: &Client,
    owner: &str,
    repo: &str,
    token: &str,
    pr: &PullRequest,
) -> Result<Response> {
    let merge_url = format!(
        "{}/repos/{}/{}/pulls/{}/merge",
        GITHUB_API_BASE, owner, repo, pr.number
    );
    let merge_body = serde_json::json!({
        "commit_title": format!("Merge Dependabot PR #{} ({})", pr.number, pr.title),
        "commit_message": "Automated merge by Rust utility.",
        "merge_method": "squash" // You can change this to "merge" or "rebase"
    });

    client
        .put(&merge_url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(ACCEPT, "application/vnd.github.v3+json")
        .header(CONTENT_TYPE, "application/json")
        .header(USER_AGENT, "DependabotAutoMerger")
        .json(&merge_body)
        .send()
        .map_err(anyhow::Error::from)
}

/// Triggers a branch update (rebase/merge) on the PR's head branch from the base branch.
pub fn update_pr_branch(
    client: &Client,
    owner: &str,
    repo: &str,
    token: &str,
    pr: &PullRequest,
) -> Result<bool> {
    let update_url = format!(
        "{}/repos/{}/{}/pulls/{}/update-branch",
        GITHUB_API_BASE, owner, repo, pr.number
    );

    let response = client
        .put(&update_url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(ACCEPT, "application/vnd.github.v3+json")
        .header(USER_AGENT, "DependabotAutoMerger")
        .header(CONTENT_TYPE, "application/json")
        .send()
        .context("Failed to send branch update request")?;

    let status = response.status();

    if status.is_success() || status.as_u16() == 202 {
        println!(
            "  🔄 Branch update ACCEPTED (queued). Waiting {UPDATE_WAIT_SECS} seconds to allow update/CI run..."
        );
        thread::sleep(Duration::from_secs(UPDATE_WAIT_SECS));
        Ok(true)
    } else {
        let error_message = response
            .text()
            .unwrap_or_else(|_| "Failed to get error body".to_string());
        eprintln!("  🚨 Branch update FAILED. Status: {status}. Body: {error_message}");
        Ok(false)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
