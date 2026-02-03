use serde::Deserialize;
use std::error::Error;
use std::process::Command;

//use crate::{github::GitHubClient, log::log};
use crate::github::GitHubClient;
use log_rs::logging::log::*;
use colored::Colorize;
use std::thread;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct WorkflowRun {
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
    client: &GitHubClient,
    repo: &str,
    commit: &str,
) -> Result<Vec<WorkflowRun>, Box<dyn Error>> {
    let url = format!("https://api.github.com/repos/{repo}/actions/runs?head_sha={commit}");

    let http_client = reqwest::Client::new();
    let response = http_client
        .get(&url)
        .header("Authorization", format!("Bearer {}", client.token))
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

async fn rerun_workflow(
    client: &GitHubClient,
    repo: &str,
    run_id: u64,
) -> Result<(), Box<dyn Error>> {
    let url =
        format!("https://api.github.com/repos/{repo}/actions/runs/{run_id}/rerun-failed-jobs");

    let http_client = reqwest::Client::new();
    let response = http_client
        .post(&url)
        .header("Authorization", format!("Bearer {}", client.token))
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

pub async fn rerun_workflows(
    client: &GitHubClient,
    commit: Option<String>,
    repo: Option<String>,
) -> Result<(), Box<dyn Error>> {
    // Get commit SHA
    let commit = if let Some(c) = commit {
        c
    } else {
        println!("No commit specified, using latest commit...");
        get_latest_commit()?
    };

    println!("Using commit: {commit}");

    // Get repository
    let repo = if let Some(r) = repo {
        r
    } else {
        println!("No repository specified, detecting from git remote...");
        get_repo_from_git()?
    };

    println!("Repository: {repo}\n");

    // Get workflow runs for the commit
    println!("Fetching workflow runs...");
    let runs = get_workflow_runs(client, &repo, &commit).await?;

    if runs.is_empty() {
        println!("No workflow runs found for this commit.");
        return Ok(());
    }

    // Filter for failed runs
    let failed_runs: Vec<_> = runs
        .iter()
        .filter(|run| {
            run.conclusion.as_deref() == Some("failure")
                || run.conclusion.as_deref() == Some("timed_out")
                || run.conclusion.as_deref() == Some("cancelled")
        })
        .collect();

    if failed_runs.is_empty() {
        println!("No failed workflow runs found for this commit.");
        println!("\nAll workflows:");
        for run in &runs {
            println!("  - {} ({}): {:?}", run.name, run.status, run.conclusion);
        }
        return Ok(());
    }

    println!("Found {} failed workflow run(s):\n", failed_runs.len());

    for run in &failed_runs {
        println!("  - {} (ID: {})", run.name, run.id);
        println!("    Status: {}", run.status);
        println!("    Conclusion: {:?}", run.conclusion);
        println!("    URL: {}\n", run.html_url);
    }

    // Re-run failed workflows
    println!("Re-running failed workflows...\n");
    for run in &failed_runs {
        print!("Re-running '{}'... ", run.name);
        match rerun_workflow(client, &repo, run.id).await {
            Ok(()) => ok(""),
            Err(e) => err(&format!("Failed: {e}")),
        };
    }

    done();
    Ok(())
}

/// Deletes failed/cancelled workflows concurrently using standard threads (max 10 at a time).
pub fn delete_failed_workflows(client: &GitHubClient, repo: &str) {
    intro(&format!("Deleting failed workflows for {repo}"));

    let path = &format!("repos/{repo}/actions/runs");
    match client.fetch_paginated::<WorkflowRun>(path) {
        Ok(runs) => {
            let failed_or_cancelled_runs: Vec<u64> = runs
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
                                err(&format!(
                                    "{}",
                                    format!("Error deleting workflow run {id_copy}: {e}").red()
                                ));
                            }
                        }));
                    }

                    // Wait for the current chunk of threads to finish (blocking)
                    for h in handles {
                        let _ = h.join();
                    }
                }

                ok(&format!("{count} failed/cancelled workflows deleted."));
            } else {
                info("No failed/cancelled workflows found.");
            }
        }
        Err(e) => {
            err(&format!("Error fetching workflow runs: {e}"));
        }
    }
    done();
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
