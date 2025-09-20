use crate::add_to_watched_projects;
use crate::dolly::GitRepo;
use crate::git::{Project, SettingsConfig};
use chrono::{DateTime, Utc};
use reqwest::Client;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::Command;
use toml::{toml, Value};

pub async fn get_all_projects(
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
fn project_is_cloned_local(repo: GitRepo) -> bool {
    let base_dir = dirs::home_dir().unwrap().join(&repo.host);
    let project_dir = base_dir.join(&repo.slug).join(&repo.repo_name);
    // todo - it could not have a .git
    project_dir.exists()
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
// new function to sync and clone all watched projects - do we want watched projects in the same
// config? it may be too messy. . . I guess if we sync it, it might be ok. Hey, you cloned
// something, we
pub async fn sparse_clone_projects(projects: Vec<GitRepo>) {
    let config_path = dirs::home_dir()
        .unwrap()
        .join(format!(".config/gits/{}.toml", projects[0].host));

    // let mut config: Value = toml::from_str(&fs::read_to_string(&config_path).unwrap_or_else(|_| "[groups]\nprojects = []".to_string())).expect("Failed to parse config file");
    let mut config: Value = toml! {
        [groups]
    }
    .into();
    for repo in projects {
        println!("found repo {:?}", repo);

        // if project_is_cloned_local(repo) {
        //     continue;
        // } else {
        //     println!("does not exist {:?}", repo);
        // }
        // we don't care if its local - we want them all I think. Maybe we can change the commands for
        // list-project based on if it is local and watched or not. it should both be cloned and
        // watched, I don't want to have dangling

        // write to watched config

        if is_watch_set_in_base_config(&repo) {
            add_to_watched_projects(&repo);
        }

        // write to ALL config
        if let Some(groups) = config.get_mut("groups") {
            if let Some(table) = groups.as_table_mut() {
                let group = &repo.slug;
                let project = &repo.repo_name;

                table.entry(group).or_insert(Value::Array(vec![]));
                table
                    .get_mut(group)
                    .expect("we just put it there")
                    .as_array_mut()
                    .expect("we put an array")
                    .push(Value::String(project.to_string()));
            } else {
                eprintln!("Error mut toml config",);
            }
            print!("final config: {:?}", &config);
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&config_path)
                .expect("Failed to open config file");
            file.write_all(
                toml::to_string(&config)
                    .expect("Failed to serialize config")
                    .as_bytes(),
            )
            .expect("Failed to write config file");
        }
    }
}

pub(crate) async fn fetch_and_log_new_commits(slug_filter: &str) {
    let base_dir = dirs::home_dir().unwrap().join("gitlab.cj.dev");

    let project_dir = base_dir.join(slug_filter);
    let entries = fs::read_dir(project_dir).expect("Failed to read base directory");

    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let project_dir = entry.path();

        if project_dir.is_dir() {
            let fetch_output = Command::new("git")
                .arg("fetch")
                .current_dir(&project_dir)
                .output()
                .expect("Failed to execute git fetch");

            if fetch_output.status.success() {
                let branches = ["main", "master"];
                for branch in &branches {
                    let remote_branch = format!("origin/{}", branch);
                    let branch_exists = Command::new("git")
                        .arg("show-ref")
                        .arg("--verify")
                        .arg(format!("refs/heads/{}", branch))
                        .current_dir(&project_dir)
                        .output()
                        .expect("Failed to check if branch exists")
                        .status
                        .success();
                    if !branch_exists {
                        continue;
                    }
                    let merge_output = Command::new("git")
                        .arg("merge-base")
                        .arg(branch)
                        .arg(&remote_branch)
                        .current_dir(&project_dir)
                        .output()
                        .expect("Failed to execute git merge-base");

                    if merge_output.status.success() {
                        let merge_base = String::from_utf8_lossy(&merge_output.stdout);

                        let log_output = Command::new("git")
                            .args(&[
                                "log",
                                &format!("{}..{}", merge_base.trim(), remote_branch),
                                "--color",
                                "--pretty=format:'%C(bold white)%h%Creset %C(bold green)%ad%Creset %C(bold yellow)%an%Creset %C(bold blue)%s%Creset'",
                                "--abbrev-commit",
                                "--date=relative",

                            ])
                            .current_dir(&project_dir)
                            .output()
                            .expect("Failed to execute git log");

                        if log_output.status.success() {
                            println!(
                                "New commits for {:?}:\n{}",
                                project_dir,
                                String::from_utf8_lossy(&log_output.stdout)
                            );
                        } else {
                            eprintln!(
                                "Error getting commit log for {:?}: {}",
                                project_dir,
                                String::from_utf8_lossy(&log_output.stderr)
                            );
                        }
                    } else {
                        eprintln!(
                            "Error getting merge base for {:?}: {}",
                            project_dir,
                            String::from_utf8_lossy(&merge_output.stderr)
                        );
                    }
                }
            } else {
                eprintln!(
                    "Error fetching updates for {:?}: {}",
                    project_dir,
                    String::from_utf8_lossy(&fetch_output.stderr)
                );
            }
        } else {
            println!("Project directory does not exist: {:?}", project_dir);
        }
    }
}
