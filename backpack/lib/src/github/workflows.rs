use clap::Parser;
use serde::Deserialize;
use std::error::Error;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct WorkflowRun {
    id: u64,
    name: String,
    status: String,
    conclusion: Option<String>,
    html_url: String,
}

#[derive(Debug, Deserialize)]
struct WorkflowRunsResponse {
    workflow_runs: Vec<WorkflowRun>,
}

fn get_latest_commit() -> Result<String, Box<dyn Error>> {
    let output = Command::new("git").args(["rev-parse", "HEAD"]).output()?;

    if !output.status.success() {
        return Err("Failed to get latest commit".into());
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn get_repo_from_git() -> Result<String, Box<dyn Error>> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()?;

    if !output.status.success() {
        return Err("Failed to get git remote".into());
    }

    let url = String::from_utf8(output.stdout)?.trim().to_string();

    // Parse GitHub URL to extract owner/repo
    let repo = url
        .trim_end_matches(".git")
        .split('/')
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("/");

    Ok(repo)
}

async fn get_workflow_runs(
    token: &str,
    repo: &str,
    commit: &str,
) -> Result<Vec<WorkflowRun>, Box<dyn Error>> {
    let url = format!(
        "https://api.github.com/repos/{}/actions/runs?head_sha={}",
        repo, commit
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "github-workflow-rerunner")
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("GitHub API error: {}", response.status()).into());
    }

    let runs: WorkflowRunsResponse = response.json().await?;
    Ok(runs.workflow_runs)
}

async fn rerun_workflow(token: &str, repo: &str, run_id: u64) -> Result<(), Box<dyn Error>> {
    let url = format!(
        "https://api.github.com/repos/{}/actions/runs/{}/rerun-failed-jobs",
        repo, run_id
    );

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "github-workflow-rerunner")
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Failed to rerun workflow: {}", response.status()).into());
    }

    Ok(())
}
