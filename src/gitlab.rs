use crate::dolly::GitRepo;
use crate::git::{Project, SettingsConfig};
use chrono::{DateTime, Utc};
use reqwest::Client;
use std::fs;

pub async fn get_all_gitlab_projects(
    gitlab_api_url: &str,
    private_token: &str,
    last_pull: &DateTime<Utc>,
) -> Result<Vec<Project>, reqwest::Error> {
    let client = Client::new();
    let mut projects = Vec::new();
    let mut page = 1;
    loop {
        print!("Fetching page {}...", page);
        let response = client
            .get(format!("{}/projects", gitlab_api_url))
            .header("Private-Token", private_token)
            .query(&[
                ("per_page", "100"),
                ("page", &page.to_string()),
                ("order_by", "created_at"),
                ("sort", "desc"),
            ])
            .send()
            .await?;
        //order_by=created_a maybe I can only get new?
        // response with    "created_at": "2025-08-20T00:33:16.526Z",
        match response.error_for_status() {
            Ok(response) => {
                let url = response.url().clone();
                let mut page_projects: Vec<Project> = response.json().await?;
                if page_projects.is_empty() {
                    println!("Projects page {:?} is empty", page);
                    break;
                }
                let created_at = &page_projects[0]
                    .created_at
                    .parse::<DateTime<Utc>>()
                    .expect("failed to parse json created_at");
                if created_at < last_pull {
                    println!(
                        "have latest {:?} created_at: {:?}, last_pull: {:?} ",
                        url, created_at, last_pull
                    );
                    break;
                }
                println!("Found projects page {:?}", page);
                projects.append(&mut page_projects);
                page += 1;
            }
            Err(e) => {
                println!("Check vpn connection? {:?}", e);
                return Err(e);
            }
        }
    }

    Ok(projects)
}
fn is_watch_set_in_base_config(repo: &GitRepo) -> bool {
    let base_config_path = dirs::home_dir()
        .unwrap()
        .join(format!(".config/gits/config.toml"));

    let config: SettingsConfig = toml::from_str(
        &fs::read_to_string(base_config_path).expect("Failed to SettingsConfig config file"),
    )
    .expect("Failed to parse SettingsConfig file");
    let watch_groups = &config.remotes.get(&repo.host).unwrap().watch_groups;
    let watch_projects = &config.remotes.get(&repo.host).unwrap().watch_projects;
    watch_groups.contains(&repo.slug)
        || watch_projects.contains(&format!("{}/{}", repo.slug, repo.repo_name))
}
