mod pr;
mod release;
mod workflow;

use anyhow::{Context, Result};
use reqwest::blocking::{Client, Response};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use reqwest::StatusCode;
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

use colored::Colorize;
use std::env;
use std::process::{Command, Stdio};
use url::Url;

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

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
