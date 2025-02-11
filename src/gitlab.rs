use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::Command;
use reqwest::Client;
use toml::Value;
use crate::dolly::GitRepo;
use crate::git::Project;

pub async fn get_all_projects(gitlab_api_url: &str, private_token: &str) -> Result<Vec<Project>, reqwest::Error> {
    let client = Client::new();
    let mut projects = Vec::new();
    let mut page = 1;
    loop {
        print!("Fetching page {}...", page);
        let response = client
            .get(format!("{}/projects", gitlab_api_url))
            .header("Private-Token", private_token)
            .query(&[("per_page", "100"), ("page", &page.to_string())])
            .send()
            .await?;
        match response.error_for_status() {
            Ok(response) => {
                let mut page_projects: Vec<Project> = response.json().await?;
                if page_projects.is_empty() {
                    break;
                }

                projects.append(&mut page_projects);
                page += 1;
            }
            Err(e) => {
                println!("Check vpn connection? {:?}", e);
                return Err(e)
            }
        }
    }

    Ok(projects)
}


pub async fn sparse_clone_projects(projects: Vec<GitRepo>) {

    let config_path = dirs::home_dir().unwrap().join(".config/gits/gitlab.cj.dev.toml");

    let mut config: Value = toml::from_str(&fs::read_to_string(&config_path).unwrap_or_else(|_| "[groups]\nprojects = []".to_string())).expect("Failed to parse config file");
    print!("{:?}", &projects);
    for repo in projects {

        let base_dir = dirs::home_dir().unwrap().join(&repo.host);
        let project_dir = base_dir.join(&repo.slug).join(&repo.repo_name);
        if project_dir.exists() {
            continue;
        } else {
            println!("does not exist {:?}", project_dir);
        }


        if let Some(groups) = config.get_mut("groups") {
            println!("in {:?}", &groups);
            if let Some(table) = groups.as_table_mut() {
                println!("table {:?}", &table);

                let group = &repo.slug;
                let project = &repo.repo_name;
                table.entry(group).or_insert(Value::Array(vec![]));
                println!("{:?}", &table);
                table.get_mut(group).expect("we just put it there").as_array_mut().expect("we put an array").push(Value::String(project.to_string()));



                println!("{:?}", &table);
            } else {
                eprintln!(
                    "Error saving project to {:?}: ",
                    project_dir,
                );
            }
            print!("final config: {:?}", &config);
            let mut file = OpenOptions::new().write(true).truncate(true).open(&config_path).expect("Failed to open config file");
            file.write_all(toml::to_string(&config).expect("Failed to serialize config").as_bytes()).expect("Failed to write config file");
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
                            println!("New commits for {:?}:\n{}", project_dir, String::from_utf8_lossy(&log_output.stdout));
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

