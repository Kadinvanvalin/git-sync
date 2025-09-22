use crate::git::Project;
use chrono::{DateTime, Utc};
use reqwest::Client;

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
