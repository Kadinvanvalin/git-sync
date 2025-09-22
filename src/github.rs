// curl -L \
// -H "Accept: application/vnd.github+json" \
// -H "Authorization: Bearer <YOUR-TOKEN>" \
// -H "X-GitHub-Api-Version: 2022-11-28" \
// https://api.github.com/users/USERNAME/repos

// Lists public repositories for the specified user.

//[
//   {
//     "id": 1296269,
//     "node_id": "MDEwOlJlcG9zaXRvcnkxMjk2MjY5",
//     "name": "Hello-World",
//     "full_name": "octocat/Hello-World",
//     "owner": {
//       "login": "octocat",
// do I need a watchlist?

use crate::git::Project;
use chrono::{DateTime, Utc};
use reqwest::Client;
use reqwest::header::HeaderMap;
use rouille::url::quirks::host;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct GitHubResponse {
    full_name: String,
    created_at: String,
}
pub async fn get_watched_github_projects(
    api_url: &str,
    private_token: &str,
    last_pull: &DateTime<Utc>,
    user: String,
) -> Result<Vec<Project>, reqwest::Error> {
    let client = Client::new();
    let mut projects: Vec<Project> = Vec::new();
    
    
    
        // / -H "Accept: application/vnd.github+json" \
        // // -H "Authorization: Bearer <YOUR-TOKEN>" \
        // // -H "X-GitHub-Api-Version: 2022-11-28" \
        let response = client
            .get(format!("{}/users/{}/repos", api_url, user))
            .header("Authorization", format!("Bearer: {}", private_token))
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?;
        match response.error_for_status() {
            Ok(response) => {
                let url = response.url().clone();
                let  page_projects: Vec<GitHubResponse> = response.json().await?;
                page_projects.iter().for_each(|project| {
                    let host = "github.com";
                    projects.push( Project {
                        ssh_url_to_repo: format!("git@{}:{}.git", host, project.full_name.clone()),
                        created_at: project.created_at.clone(),
                    });
                })
            }
            Err(e) => {
                println!("Check vpn connection? {:?}", e);
                return Err(e);
            }
        }
    Ok(projects)
}