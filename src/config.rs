use crate::dolly::GitRepo;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::{env, fs};
use toml::Value;
#[derive(Deserialize, Serialize, Debug)]
pub struct RepoProjects {
    pub(crate) groups: HashMap<String, Vec<String>>,
}
type Slug = String;
type ProjectName = String;
type ProjectGroups = HashMap<Slug, Vec<ProjectName>>;
pub trait GitsConfig {
    // fn host(&self) -> String;
    fn build() -> Self;

    fn get_projects(&self) -> ProjectGroups;
    fn get_repos(&self) -> Vec<GitRepo>;
    fn get_private_token(&self) -> String;
    fn get_last_sync(&self) -> DateTime<Utc>;
    fn get_gitlab_api_url(&self) -> String;
    fn get_settings_config(&self) -> SettingsConfig;
    fn add_to_global(&self, git_repo: &GitRepo) -> ();
}
pub struct RealGitsConfig {
    pub host: String,
    groups_path: PathBuf,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SettingsConfig {
    pub remotes: HashMap<String, RemoteSettings>,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct RemoteSettings {
    pub token: String,
    pub project_directory: String,
    pub gitlab_api_url: String,
    pub watch_groups: Vec<String>,
    pub watch_projects: Vec<String>,
    pub last_pull: String,
}

impl GitsConfig for RealGitsConfig {
    fn build() -> Self {
        let config_path = dirs::home_dir().unwrap().join(".config/gits/config.toml");
        let config: SettingsConfig = toml::from_str(
            &fs::read_to_string(config_path).expect("Failed to SettingsConfig config file"),
        )
        .expect("Failed to parse SettingsConfig file");

        let host = config.remotes.keys().next().expect("it to work"); // defaulting to one host
        let groups_path = dirs::home_dir()
            .unwrap()
            .join(format!(".config/gits/{}-watched.toml", host));

        RealGitsConfig {
            host: host.clone(),
            groups_path,
        }
    }
    fn get_settings_config(&self) -> SettingsConfig {
        let config_string = ".config/gits/config.toml";
        let config_path = dirs::home_dir().unwrap().join(config_string);
        //
        let settings_config: SettingsConfig =
            toml::from_str(&fs::read_to_string(config_path).expect(&format!(
                "Failed to find SettingsConfig config file at location: {}",
                config_string
            )))
            .expect("Failed to parse SettingsConfig file");
        settings_config
    }
    fn get_projects(&self) -> ProjectGroups {
        toml::from_str::<RepoProjects>(
            &fs::read_to_string(&self.groups_path).expect("Failed to read projects file"),
        )
        .expect("Failed to parse projects file")
        .groups
    }
    fn get_repos(&self) -> Vec<GitRepo> {
        self.get_projects()
            .into_iter()
            .flat_map(|(slug, projects)| {
                projects.into_iter().map(move |repo_name| GitRepo {
                    host: self.host.clone(),
                    slug: slug.clone(),
                    repo_name,
                })
            })
            .collect::<Vec<GitRepo>>()
    }

    fn get_private_token(&self) -> String {
        let token_env_location = self
            .get_settings_config()
            .remotes
            .get(&self.host)
            .expect("it to work")
            .token
            .clone();

        env::var(token_env_location).expect("can't find token")
    }

    fn get_last_sync(&self) -> DateTime<Utc> {
        self.get_settings_config()
            .remotes
            .get(&self.host)
            .expect("it to work")
            .last_pull
            .parse::<DateTime<Utc>>()
            .expect("failed to parse datetime")
    }
    fn get_gitlab_api_url(&self) -> String {
        self.get_settings_config()
            .remotes
            .get(&self.host)
            .expect("it to work")
            .gitlab_api_url
            .clone()
    }
    fn add_to_global(&self, git_repo: &GitRepo) {
        let config_path = dirs::home_dir()
            .unwrap()
            .join(format!(".config/gits/{}-watched.toml", git_repo.host));
        let mut config: Value = toml::from_str(
            &fs::read_to_string(&config_path)
                .unwrap_or_else(|_| "[groups]\nprojects = []".to_string()),
        )
        .expect("Failed to parse config file");

        if let Some(groups) = config.get_mut("groups") {
            if let Some(table) = groups.as_table_mut() {
                let group = &git_repo.slug;
                let project = &git_repo.repo_name;
                table.entry(group).or_insert(Value::Array(vec![]));
                table
                    .get_mut(group)
                    .expect("we just put it there")
                    .as_array_mut()
                    .expect("we put an array")
                    .push(Value::String(project.to_string()));
                table
                    .get_mut(group)
                    .expect("we just put it there")
                    .as_array_mut()
                    .unwrap()
                    .dedup();
            } else {
                eprintln!("Error mut toml config",);
            }
            print!("final config: {:?}", &config);
        }

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
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
