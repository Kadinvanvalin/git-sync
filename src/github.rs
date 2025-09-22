use std::collections::HashMap;
use crate::git::GitRepo;
use chrono::{DateTime, Utc};
use regex::Regex;
use reqwest::Client;
use reqwest::header::{HeaderMap, ACCEPT, AUTHORIZATION};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct GitHubResponse {
    full_name: String,
    created_at: String,
}
fn api_url_to_host(url: &str) -> String {
    let re = Regex::new("https?://(.+)").expect("failed to parse regex");
    let caps = re.captures(&url).unwrap();
    let host = caps.get(1).map_or("", |m| m.as_str());
    String::from(host)
}

pub async fn get_watched_github_projects(
    api_url: &str,
    private_token: &str,
    last_pull: &DateTime<Utc>,
    user: String,
    id: String,
) -> Result<Vec<GitRepo>, reqwest::Error> {
    let client = Client::new();
    let mut repos: Vec<GitRepo> = Vec::new();
    let mut headers = HeaderMap::new();

    headers.insert(ACCEPT, "application/vnd.github+json".parse().unwrap());
    headers.insert("X-GitHub-Api-Version", "2022-11-28".parse().unwrap());
    
    if !private_token.is_empty() {
        headers.insert(AUTHORIZATION, format!("Bearer: {}", private_token).parse().unwrap());
    }
    let response = client
        .get(format!("{}/users/{}/repos", api_url, user))
        .headers(headers)
        .send()
        .await?;
    
    
        match response.error_for_status() {
        Ok(response) => {
            let page_projects: Vec<GitHubResponse> = response.json().await?;
            page_projects.iter().for_each(|project| {
                let repo_name = project.full_name.split('/').last().unwrap();

                repos.push(GitRepo {
                    host: api_url_to_host(api_url),
                    slug: user.clone(),
                    repo_name: repo_name.to_string(),
                })
            })
        }
        Err(e) => {
            println!("Check vpn connection? {:?}", e);
            return Err(e);
        }
    }
    Ok(repos)
}
