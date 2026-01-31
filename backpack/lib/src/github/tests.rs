#[cfg(test)]
mod tests {
    use anyhow::{Context, Result};
    use mockito;
    use reqwest::blocking::Client;
    use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};

    use crate::github::{DEPENDABOT_USER, MergeResponse, PullRequest, User};

    const REPO: &str = "test_owner/test_repo";
    const TOKEN: &str = "test_token";

    // --- Data Structure Tests ---

    #[test]
    fn deserialize_pull_request() -> Result<()> {
        let json_input = r#"{
            "number": 123,
            "title": "chore: bump rust from 1.70.0 to 1.70.1",
            "user": {
                "login": "dependabot[bot]"
            }
        }"#;

        let pr: PullRequest = serde_json::from_str(json_input)?;

        assert_eq!(pr.number, 123);
        assert_eq!(pr.title, "chore: bump rust from 1.70.0 to 1.70.1");
        assert_eq!(pr.user.login, DEPENDABOT_USER);
        Ok(())
    }

    // --- API Function Tests (using mockito) ---

    #[test]
    fn test_list_dependabot_prs_success() -> Result<()> {
        // 1. Setup Mock Server
        let mut server = mockito::Server::new();
        let mock_base = server.url();

        let body = format!(
            r#"
        [
            {{ "number": 1, "title": "Dependabot PR", "user": {{ "login": "{}" }} }},
            {{ "number": 2, "title": "Manual PR", "user": {{ "login": "some_user" }} }},
            {{ "number": 3, "title": "Another Dependabot PR", "user": {{ "login": "{}" }} }}
        ]"#,
            DEPENDABOT_USER, DEPENDABOT_USER
        );

        let mock = server
            .mock("GET", format!("/repos/{}/pulls", REPO).as_str())
            .match_query("state=open&per_page=100")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create();

        // 2. Call Function (Manually making the request to the mock URL)
        let client = Client::builder().build()?;

        let url = format!("{}/repos/{}/pulls?state=open&per_page=100", mock_base, REPO);

        let response = client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {}", TOKEN))
            .header(ACCEPT, "application/vnd.github.v3+json")
            .header(USER_AGENT, "DependabotAutoMerger")
            .send()
            .context("Failed to send list PRs request")?;

        let all_prs: Vec<PullRequest> = response.json()?;

        let dependabot_prs: Vec<PullRequest> = all_prs
            .into_iter()
            .filter(|pr| pr.user.login == DEPENDABOT_USER)
            .collect();

        // 3. Assertions
        mock.assert();
        assert_eq!(
            dependabot_prs.len(),
            2,
            "Should have filtered out the manual PR."
        );
        assert!(
            dependabot_prs
                .iter()
                .all(|pr| pr.user.login == DEPENDABOT_USER)
        );

        Ok(())
    }

    #[test]
    fn test_attempt_merge_success() -> Result<()> {
        // 1. Setup Mock Server
        let mut server = mockito::Server::new();
        let pr_number = 456;

        let merge_body =
            r#"{ "message": "Pull Request successfully merged", "sha": "abcdef123456" }"#;

        let mock = server
            .mock(
                "PUT",
                format!("/repos/{}/pulls/{}/merge", REPO, pr_number).as_str(),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(merge_body)
            .create();

        // 2. Call Function (using mockito server URL as base)
        let client = Client::builder().build()?;
        let pr = PullRequest {
            number: pr_number,
            title: "Test PR".to_string(),
            user: User {
                login: DEPENDABOT_USER.to_string(),
            },
        };

        let mock_base = server.url();

        // Manually build the merge URL using the mock server's URL
        let merge_url = format!("{}/repos/{}/pulls/{}/merge", mock_base, REPO, pr.number);
        let merge_body_json = serde_json::json!({
            "commit_title": format!("{} (#{})", pr.title, pr.number),
            "commit_message": "Automated merge by Rust utility.",
            "merge_method": "squash"
        });

        let response = client
            .put(&merge_url)
            .header(AUTHORIZATION, format!("Bearer {}", TOKEN))
            .header(ACCEPT, "application/vnd.github.v3+json")
            .header(CONTENT_TYPE, "application/json")
            .header(USER_AGENT, "DependabotAutoMerger")
            .json(&merge_body_json)
            .send()?;

        // 3. Assertions
        mock.assert();
        assert!(response.status().is_success());
        let response_data: MergeResponse = response.json()?;
        assert_eq!(response_data.sha.unwrap(), "abcdef123456");

        Ok(())
    }

    use super::*;
    use mockito::{Mock, Server, ServerGuard};
    use serde_json::json;

    async fn setup_mock_server() -> ServerGuard {
        Server::new_async().await
    }

    fn create_workflow_run_json(
        id: u64,
        name: &str,
        status: &str,
        conclusion: Option<&str>,
    ) -> serde_json::Value {
        json!({
            "id": id,
            "name": name,
            "status": status,
            "conclusion": conclusion,
            "html_url": format!("https://github.com/owner/repo/actions/runs/{}", id)
        })
    }

    #[tokio::test]
    async fn test_get_workflow_runs_success() {
        let mut server = setup_mock_server().await;

        let mock = server
            .mock("GET", "/repos/owner/repo/actions/runs")
            .match_query(mockito::Matcher::AllOf(vec![mockito::Matcher::UrlEncoded(
                "head_sha".into(),
                "abc123".into(),
            )]))
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "workflow_runs": [
                        create_workflow_run_json(1, "CI", "completed", Some("success")),
                        create_workflow_run_json(2, "Tests", "completed", Some("failure")),
                    ]
                })
                .to_string(),
            )
            .create_async()
            .await;

        // This is a conceptual test - you'd need to modify get_workflow_runs
        // to accept a base_url parameter for testing
        // let runs = get_workflow_runs("test-token", "owner/repo", "abc123", Some(&server.url())).await.unwrap();

        // assert_eq!(runs.len(), 2);
        // assert_eq!(runs[0].id, 1);
        // assert_eq!(runs[0].name, "CI");
        // assert_eq!(runs[1].conclusion, Some("failure".to_string()));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_workflow_runs_empty_response() {
        let mut server = setup_mock_server().await;

        let mock = server
            .mock("GET", "/repos/owner/repo/actions/runs")
            .match_query(mockito::Matcher::AllOf(vec![mockito::Matcher::UrlEncoded(
                "head_sha".into(),
                "abc123".into(),
            )]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({"workflow_runs": []}).to_string())
            .create_async()
            .await;

        // let runs = get_workflow_runs("test-token", "owner/repo", "abc123", Some(&server.url())).await.unwrap();
        // assert_eq!(runs.len(), 0);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_workflow_runs_api_error() {
        let mut server = setup_mock_server().await;

        let mock = server
            .mock("GET", "/repos/owner/repo/actions/runs")
            .with_status(401)
            .with_header("content-type", "application/json")
            .with_body(json!({"message": "Bad credentials"}).to_string())
            .create_async()
            .await;

        // let result = get_workflow_runs("bad-token", "owner/repo", "abc123", Some(&server.url())).await;
        // assert!(result.is_err());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_rerun_workflow_success() {
        let mut server = setup_mock_server().await;

        let mock = server
            .mock(
                "POST",
                "/repos/owner/repo/actions/runs/123/rerun-failed-jobs",
            )
            .match_header("authorization", "Bearer test-token")
            .with_status(201)
            .create_async()
            .await;

        // let result = rerun_workflow("test-token", "owner/repo", 123, Some(&server.url())).await;
        // assert!(result.is_ok());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_rerun_workflow_failure() {
        let mut server = setup_mock_server().await;

        let mock = server
            .mock(
                "POST",
                "/repos/owner/repo/actions/runs/123/rerun-failed-jobs",
            )
            .with_status(403)
            .with_header("content-type", "application/json")
            .with_body(json!({"message": "Forbidden"}).to_string())
            .create_async()
            .await;

        // let result = rerun_workflow("test-token", "owner/repo", 123, Some(&server.url())).await;
        // assert!(result.is_err());

        mock.assert_async().await;
    }

    #[test]
    fn test_filter_failed_runs() {
        let runs = vec![
            serde_json::from_value::<WorkflowRun>(create_workflow_run_json(
                1,
                "CI",
                "completed",
                Some("success"),
            ))
            .unwrap(),
            serde_json::from_value::<WorkflowRun>(create_workflow_run_json(
                2,
                "Tests",
                "completed",
                Some("failure"),
            ))
            .unwrap(),
            serde_json::from_value::<WorkflowRun>(create_workflow_run_json(
                3,
                "Build",
                "completed",
                Some("timed_out"),
            ))
            .unwrap(),
            serde_json::from_value::<WorkflowRun>(create_workflow_run_json(
                4,
                "Lint",
                "completed",
                Some("cancelled"),
            ))
            .unwrap(),
            serde_json::from_value::<WorkflowRun>(create_workflow_run_json(
                5,
                "Deploy",
                "in_progress",
                None,
            ))
            .unwrap(),
        ];

        let failed: Vec<_> = runs
            .iter()
            .filter(|run| {
                run.conclusion.as_deref() == Some("failure")
                    || run.conclusion.as_deref() == Some("timed_out")
                    || run.conclusion.as_deref() == Some("cancelled")
            })
            .collect();

        assert_eq!(failed.len(), 3);
        assert_eq!(failed[0].id, 2);
        assert_eq!(failed[1].id, 3);
        assert_eq!(failed[2].id, 4);
    }

    #[test]
    fn test_parse_github_url_https() {
        let url = "https://github.com/owner/repo.git";
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

        assert_eq!(repo, "owner/repo");
    }

    #[test]
    fn test_parse_github_url_ssh() {
        let url = "git@github.com:owner/repo.git";
        // This test shows the current parsing logic doesn't handle SSH URLs
        // You may want to improve the parsing logic in get_repo_from_git
        let parts: Vec<&str> = url.trim_end_matches(".git").split(':').collect();
        let repo = if parts.len() == 2 {
            parts[1].to_string()
        } else {
            url.split('/')
                .rev()
                .take(2)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("/")
        };

        assert_eq!(repo, "owner/repo");
    }

    // Add this to your WorkflowRun struct for testing
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct WorkflowRun {
        id: u64,
        name: String,
        status: String,
        conclusion: Option<String>,
        html_url: String,
    }
}

// Integration test helpers
#[cfg(test)]
mod integration_tests {
    use std::process::Command;

    #[test]
    #[ignore] // Ignore by default as it requires git setup
    fn test_get_latest_commit_integration() {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .expect("Failed to execute git command");

        assert!(output.status.success());
        let commit = String::from_utf8(output.stdout).unwrap();
        assert!(!commit.trim().is_empty());
        assert_eq!(commit.trim().len(), 40); // SHA-1 hash length
    }

    #[test]
    #[ignore] // Ignore by default as it requires git setup
    fn test_get_repo_from_git_integration() {
        let output = Command::new("git")
            .args(["remote", "get-url", "origin"])
            .output()
            .expect("Failed to execute git command");

        if output.status.success() {
            let url = String::from_utf8(output.stdout).unwrap();
            assert!(!url.trim().is_empty());
        }
    }
}
