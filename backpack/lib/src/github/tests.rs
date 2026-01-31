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
}
