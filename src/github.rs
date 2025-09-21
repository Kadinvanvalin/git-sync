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

// pub async fn get_all_projects(
//     gitlab_api_url: &str,
//     private_token: &str,
//     last_pull: &DateTime<Utc>,
// ) -> Result<Vec<Project>, reqwest::Error> {
//     let client = Client::new();
//     let mut projects = Vec::new();
//     let mut page = 1;
//     loop {
//         print!("Fetching page {}...", page);
//         let response = client
//             .get(format!("{}/projects", gitlab_api_url))
//             .header("Private-Token", private_token)
//             .query(&[
//                 ("per_page", "100"),
//                 ("page", &page.to_string()),
//                 ("order_by", "created_at"),
//                 ("sort", "desc"),
//             ])
//             .send()
//             .await?;
//     }