use anyhow::{Context, Result};
use reqwest::StatusCode;
use reqwest::blocking::{Client, Response};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use serde::{Deserialize, Serialize};
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
pub fn list_dependabot_prs(client: &Client, repo: &str, token: &str) -> Result<Vec<PullRequest>> {
    let url = format!("{GITHUB_API_BASE}/repos/{repo}/pulls?state=open&per_page=100");

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
pub fn process_pr(client: &Client, repo: &str, token: &str, pr: &PullRequest) -> Result<()> {
    for attempt in 1..=MAX_MERGE_ATTEMPTS {
        // 1. Attempt to merge the PR
        let merge_response = attempt_merge(client, repo, token, pr).context(format!(
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
            if update_pr_branch(client, repo, token, pr)? {
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
    repo: &str,
    token: &str,
    pr: &PullRequest,
) -> Result<Response> {
    let merge_url = format!(
        "{}/repos/{}/pulls/{}/merge",
        GITHUB_API_BASE, repo, pr.number
    );
    let merge_body = serde_json::json!({
        "commit_title": format!("{} (#{})", pr.title, pr.number),
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
    repo: &str,
    token: &str,
    pr: &PullRequest,
) -> Result<bool> {
    let update_url = format!(
        "{}/repos/{}/pulls/{}/update-branch",
        GITHUB_API_BASE, repo, pr.number
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

use colored::Colorize;
use std::env;
use std::process::{Command, Stdio};
use url::Url;

// --- API CLIENT & HELPERS ---
//
pub struct GitHubClient {
    client: Client,
    token: String,
    api_base: Url,
}

impl GitHubClient {
    /// Initializes the client, checking for the `GITHUB_TOKEN` environment variable.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let token = env::var("GITHUB_TOKEN")
            .map_err(|_| {
                eprintln!(
                    "{}",
                    format!("{}", "Error: GITHUB_TOKEN environment variable not set. Please set it to your Personal Access Token.".red())
                );
                "GITHUB_TOKEN required"
            })?;

        // Build the blocking client
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("github-maintain-rs/1.0")
            .build()?;

        let api_base = Url::parse("https://api.github.com/")?;

        Ok(Self {
            client,
            token,
            api_base,
        })
    }

    /// Performs a paginated GET request and collects all items.
    fn fetch_paginated<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
    ) -> Result<Vec<T>, reqwest::Error> {
        let url = self.api_base.join(path).unwrap();
        let mut results = Vec::new();
        let mut page = 1;

        loop {
            let mut current_url = url.clone();
            current_url
                .query_pairs_mut()
                .append_pair("per_page", "100")
                .append_pair("page", &page.to_string());

            let response: Response = self
                .client
                .get(current_url)
                .bearer_auth(&self.token)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .send()?;

            if response.status().is_success() {
                let json_data: serde_json::Value = response.json()?;

                // Check for array response (common for listing items)
                if let Some(array) = json_data.as_array() {
                    for item in array {
                        results.push(serde_json::from_value(item.clone()).unwrap());
                    }
                    if array.is_empty() || array.len() < 100 {
                        break; // End of pagination
                    }
                }
                // Check for object response with 'workflow_runs' field (specific to workflow API)
                else if let Some(runs) = json_data["workflow_runs"].as_array() {
                    for item in runs {
                        results.push(serde_json::from_value(item.clone()).unwrap());
                    }
                    if runs.is_empty() || runs.len() < 100 {
                        break; // End of pagination
                    }
                } else {
                    break; // Unexpected response structure, stop
                }

                page += 1;
            } else if response.status() == StatusCode::NOT_FOUND {
                break; // No more pages or resource not found
            } else {
                return Err(response.error_for_status().unwrap_err());
            }
        }

        Ok(results)
    }

    /// Performs a simple blocking DELETE request.
    /*
    fn delete(&self, path: &str) -> Result<(), reqwest::Error> {
        let url = self.api_base.join(path).unwrap();

        let response = self
            .client
            .delete(url)
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()?;

        // Treat 204 No Content and 202 Accepted as success
        if response.status().is_success()
            || response.status() == StatusCode::NO_CONTENT
            || response.status() == StatusCode::ACCEPTED
        {
            Ok(())
        } else {
            Err(response.error_for_status().unwrap_err())
        }
    }
    */

    /// Performs a simple blocking POST request.
    fn post<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R, reqwest::Error> {
        let url = self.api_base.join(path).unwrap();

        let response = self
            .client
            .post(url)
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(body)
            .send()?;

        response.error_for_status()?.json()
    }
}

// --- DATA STRUCTURES ---

#[derive(Debug, Deserialize)]
pub struct WorkflowRun {
    id: i64,
    name: String,
    conclusion: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Release {
    id: i64,
    tag_name: String,
}

#[derive(Debug, Deserialize)]
pub struct PackageVersion {
    id: i64,
    metadata: Option<PackageMetadata>,
}

#[derive(Debug, Deserialize)]
pub struct PackageMetadata {
    container: Option<ContainerMetadata>,
}

#[derive(Debug, Deserialize)]
pub struct ContainerMetadata {
    tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct CreateRelease {
    tag_name: String,
    target_commitish: String,
    name: String,
    body: String,
    draft: bool,
    prerelease: bool,
    generate_release_notes: bool,
}

// --- CORE LOGIC FUNCTIONS ---

/// Deletes failed/cancelled workflows concurrently using standard threads (max 10 at a time).
pub fn delete_failed_workflows(client: &GitHubClient, repo: &str) {
    println!(
        "{}",
        format!("Deleting failed workflows for {repo}").yellow()
    );

    let path = &format!("repos/{repo}/actions/runs");
    match client.fetch_paginated::<WorkflowRun>(path) {
        Ok(runs) => {
            let failed_or_cancelled_runs: Vec<i64> = runs
                .into_iter()
                .filter(|r| {
                    r.conclusion.as_deref() == Some("failure")
                        || r.conclusion.as_deref() == Some("cancelled")
                })
                .map(|r| r.id)
                .collect();

            let count = failed_or_cancelled_runs.len();
            if count > 0 {
                // Chunk the runs into groups of 10 for concurrent deletion
                let chunked_runs = failed_or_cancelled_runs.chunks(10);
                for chunk in chunked_runs {
                    let mut handles = Vec::new();

                    for id in chunk {
                        // Clone necessary parts for thread ownership
                        let client_clone = client.client.clone();
                        let token_clone = client.token.clone();
                        let api_base_clone = client.api_base.clone();
                        let repo_str = repo.to_string();
                        let id_copy = *id;

                        // Spawn a standard OS thread for deletion
                        handles.push(thread::spawn(move || {
                            let delete_path = format!("repos/{repo_str}/actions/runs/{id_copy}");
                            let url = api_base_clone.join(&delete_path).unwrap();

                            let res = client_clone
                                .delete(url)
                                .bearer_auth(token_clone)
                                .header("Accept", "application/vnd.github+json")
                                .header("X-GitHub-Api-Version", "2022-11-28")
                                .send();

                            if let Err(e) = res {
                                eprintln!(
                                    "{}",
                                    format!("Error deleting workflow run {id_copy}: {e}").red()
                                );
                            }
                        }));
                    }

                    // Wait for the current chunk of threads to finish (blocking)
                    for h in handles {
                        let _ = h.join();
                    }
                }

                println!(
                    "{}",
                    format!("{count} failed/cancelled workflows deleted.").blue()
                );
            } else {
                println!("{}", "No failed/cancelled workflows found.".blue());
            }
        }
        Err(e) => {
            eprintln!("{}", format!("Error fetching workflow runs: {e}").red());
        }
    }
    println!("{}", "Done.".yellow());
}

/// Deletes untagged container versions.
pub fn delete_old_container_versions(client: &GitHubClient, repo: &str) {
    println!("{}", format!("Deleting old containers for {repo}").yellow());

    let parts: Vec<&str> = repo.split('/').collect();
    if parts.len() != 2 {
        eprintln!("{}", format!("Error: Repository format '{repo}' is invalid. Expected 'owner/project'. Skipping container deletion.").red());
        return;
    }
    let org = parts[0];
    let project = parts[1];

    let path = &format!("orgs/{org}/packages/container/{project}/versions");
    match client.fetch_paginated::<PackageVersion>(path) {
        Ok(versions) => {
            let untagged_versions: Vec<i64> = versions
                .into_iter()
                .filter_map(|v| {
                    let tags = v
                        .metadata
                        .and_then(|m| m.container)
                        .and_then(|c| c.tags)
                        .unwrap_or_default();

                    if tags.is_empty() { Some(v.id) } else { None }
                })
                .collect();

            let count = untagged_versions.len();
            if count > 0 {
                let mut handles = Vec::new();
                for id in untagged_versions {
                    // Clone necessary parts for thread ownership
                    let client_clone = client.client.clone();
                    let token_clone = client.token.clone();
                    let api_base_clone = client.api_base.clone();
                    let org_str = org.to_string();
                    let project_str = project.to_string();

                    handles.push(thread::spawn(move || {
                        let delete_path = format!(
                            "orgs/{org_str}/packages/container/{project_str}/versions/{id}"
                        );
                        let url = api_base_clone.join(&delete_path).unwrap();

                        let res = client_clone
                            .delete(url)
                            .bearer_auth(token_clone)
                            .header("Accept", "application/vnd.github+json")
                            .header("X-GitHub-Api-Version", "2022-11-28")
                            .send();

                        if let Err(e) = res {
                            eprintln!(
                                "{}",
                                format!("Error deleting container version {id}: {e}").red()
                            );
                        }
                    }));
                }

                // Wait for all deletions to complete
                for h in handles {
                    let _ = h.join();
                }

                println!(
                    "{}",
                    format!("{count} untagged container versions deleted.").blue()
                );
            } else {
                println!(
                    "{}",
                    "No untagged container versions found to delete.".blue()
                );
            }
        }
        Err(e) => {
            eprintln!(
                "{}",
                format!(
                    "Error fetching container versions: {e}. Check if the repo is an org package."
                )
                .red()
            );
        }
    }
    println!("{}", "Done.".yellow());
}

/// Deletes all releases and their corresponding Git tags.
pub fn delete_all_releases(
    client: &GitHubClient,
    repo: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Delete releases
    println!("{}", format!("Deleting all releases for {repo}").yellow());
    let releases_path = &format!("repos/{repo}/releases");

    match client.fetch_paginated::<Release>(releases_path) {
        Ok(releases) => {
            let count = releases.len();
            let mut handles = Vec::new();
            for r in releases {
                let client_clone = client.client.clone();
                let token_clone = client.token.clone();
                let api_base_clone = client.api_base.clone();
                let repo_str = repo.to_string();

                handles.push(thread::spawn(move || {
                    let delete_path = format!("repos/{}/releases/{}", repo_str, r.id);
                    let url = api_base_clone.join(&delete_path).unwrap();

                    let res = client_clone
                        .delete(url)
                        .bearer_auth(token_clone)
                        .header("Accept", "application/vnd.github+json")
                        .header("X-GitHub-Api-Version", "2022-11-28")
                        .send();

                    if let Err(e) = res {
                        eprintln!(
                            "{}",
                            format!("Error deleting release {}: {}", r.tag_name, e).red()
                        );
                    }
                }));
            }

            for h in handles {
                let _ = h.join();
            }

            println!("{}", format!("{count} releases deleted.").blue());
        }
        Err(e) => {
            eprintln!("{}", format!("Error fetching releases: {e}").red());
        }
    }
    println!("{}", "Done.".yellow());

    // 2. Delete tags (using external git commands, like the original script)
    println!("{}", format!("Deleting all tags for {repo}").yellow());

    // Create a temporary directory
    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path();
    let repo_url = format!("https://github.com/{repo}");

    // Clone the repo
    // We clone a mirror to access tags easily without checking out history
    let clone_output = Command::new("git")
        .arg("clone")
        .arg("--quiet")
        .arg("--mirror")
        .arg(&repo_url)
        .arg(temp_path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .status()?;

    if !clone_output.success() {
        eprintln!(
            "{}",
            format!(
                "Error: Unable to clone repo {repo}. Ensure it exists and you have permission."
            )
            .red()
        );
        return Err("Git clone failed".into());
    }

    // List tags
    let tags_output = Command::new("git")
        .current_dir(temp_path)
        .arg("tag")
        .output()?;

    let tags = String::from_utf8(tags_output.stdout)?
        .lines()
        .map(std::string::ToString::to_string)
        .collect::<Vec<String>>();

    if tags.is_empty() {
        println!("{}", "No tags found to delete.".blue());
        println!("{}", "Done.".yellow());
        return Ok(());
    }

    println!(
        "{}",
        format!("Found {} tags. Deleting...", tags.len()).blue()
    );

    // Delete tags on remote using one push command
    let mut push_command = Command::new("git");
    push_command
        .current_dir(temp_path)
        .arg("push")
        .arg("origin")
        .arg("--delete");

    // Add all tags to the delete command
    for tag in &tags {
        push_command.arg(tag);
    }

    // Execute the push command
    let push_output = push_command
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()?;

    if !push_output.status.success() {
        let stderr = String::from_utf8_lossy(&push_output.stderr);
        eprintln!("{}", format!("Error pushing tag deletions: {stderr}").red());
        return Err("Git push --delete failed".into());
    }

    println!("{}", "Done.".yellow());
    Ok(())
}

/// Creates a new v0.1.0 release.
pub fn create_release(client: &GitHubClient, repo: &str) -> Result<(), reqwest::Error> {
    let release_data = CreateRelease {
        tag_name: "v0.1.0".to_string(),
        target_commitish: "main".to_string(),
        name: "v0.1.0".to_string(),
        body: "First release of github-rs.".to_string(),
        draft: false,
        prerelease: false,
        generate_release_notes: true,
    };

    let path = &format!("repos/{repo}/releases");
    match client.post::<_, serde_json::Value>(path, &release_data) {
        Ok(res) => {
            println!(
                "{}",
                format!("Successfully created release v0.1.0 for {repo}.").green()
            );
            if let Some(url) = res["html_url"].as_str() {
                println!("Release URL: {}", url.cyan());
            }
        }
        Err(e) => {
            eprintln!("{}", format!("Error creating release: {e}").red());
        }
    }

    Ok(())
}

/// Reruns failed workflow jobs.
pub fn rerun_failed_jobs(client: &GitHubClient, repo: &str) {
    println!("{}", format!("Rerun failed jobs for {repo}").yellow());

    let path = &format!("repos/{repo}/actions/runs");
    match client.fetch_paginated::<WorkflowRun>(path) {
        Ok(runs) => {
            let failed_runs: Vec<WorkflowRun> = runs
                .into_iter()
                .filter(|r| r.conclusion.as_deref() == Some("failure"))
                .collect();

            if failed_runs.is_empty() {
                println!("{}", "No failed jobs found to rerun.".blue());
                return;
            }

            for run in failed_runs {
                println!(
                    "{}",
                    format!("Rerunning job \"{}\" ({})", run.name, run.id).green()
                );
                let rerun_path =
                    &format!("repos/{}/actions/runs/{}/rerun-failed-jobs", repo, run.id);

                // Use post with an empty body
                let res = client.post::<_, serde_json::Value>(rerun_path, &serde_json::json!({}));

                if let Err(e) = res {
                    eprintln!("{}", format!("Error rerunning job {}: {}", run.id, e).red());
                } else {
                    // Introduce a slight delay to avoid hitting rate limits too quickly
                    thread::sleep(Duration::from_millis(500));
                }
            }
        }
        Err(e) => {
            eprintln!(
                "{}",
                format!("Error fetching workflow runs for rerun: {e}").red()
            );
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
