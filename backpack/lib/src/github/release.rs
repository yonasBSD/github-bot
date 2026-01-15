#[derive(Debug, Deserialize)]
pub struct Release {
    id: u64,
    tag_name: String,
}

#[derive(Debug, Deserialize)]
pub struct PackageVersion {
    id: u64,
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
            let untagged_versions: Vec<u64> = versions
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
