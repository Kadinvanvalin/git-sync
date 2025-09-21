use crate::dolly::GitRepo;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::{env, fs};
use toml::Value;
#[derive(Deserialize, Serialize, Debug)]
pub struct InventoryToml {
    pub(crate) groups: InventoryGroups,
}
type Slug = String;
type ProjectName = String;
type Host = String;
type InventoryGroups = HashMap<Slug, Vec<ProjectName>>;
pub trait GitsConfig {
    // fn host(&self) -> String;
    fn build() -> Self;

    fn get_inventory(&self) -> anyhow::Result<HashMap<Host, InventoryGroups>>;
    fn get_repos(&self) -> Vec<GitRepo>;
    fn get_private_token(&self, host: Host) -> String;
    fn get_last_sync(&self, host: Host) -> DateTime<Utc>;
    fn get_gitlab_api_url(&self, host: Host) -> String;

    fn add_to_inventory(&self, git_repo: &GitRepo) -> ();
    fn get_remotes_config(&self) -> anyhow::Result<RemotesConfig>;
}
pub struct RealGitsConfig {
    // pub host: String,
    // groups_path: PathBuf,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RemotesConfig {
    pub remotes: HashMap<Host, RemoteSettings>,
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
        // let remotes_config: RemotesConfig =  <RealGitsConfig as GitsConfig>::load_settings_config().unwrap();

        // let host = remotes_config.remotes.keys().next().expect("it to work"); // defaulting to one host
        // let groups_path = dirs::home_dir()
        //     .unwrap()
        //     .join(format!(".config/gits/{}-watched.toml", host));

        RealGitsConfig {
            // host: host.clone(),
            // groups_path,
        }
    }

    fn get_remotes_config(&self) -> anyhow::Result<RemotesConfig> {
        let config_path = dirs::home_dir().unwrap().join(".config/gits/config.toml");

        if !config_path.exists() {
            anyhow::bail!(
                "No config file found at {}.\n\
             Please create one with at least a [remotes.<name>] section.\n\
             Example:\n\
             [remotes.gitlab]\n\
             token = \"GITLAB_TOKEN_ENV\"\n\
             project_directory = \"~/projects\"\n\
             gitlab_api_url = \"https://gitlab.com\"\n\
             watch_groups = []\n\
             watch_projects = []",
                config_path.display()
            );
        }

        let raw = fs::read_to_string(&config_path)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", config_path.display(), e))?;

        let config: RemotesConfig = toml::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", config_path.display(), e))?;

        Ok(config)
    }

    fn get_inventory(&self) -> anyhow::Result<HashMap<Host, InventoryGroups>> {
        // self.get_remotes_config().expect("Failed to get remotes config")
        let remotes_config: RemotesConfig = self.get_remotes_config().unwrap();

        remotes_config
            .remotes
            .keys()
            .into_iter()
            .map(|host| {
                let inventory_path = dirs::home_dir()
                    .unwrap()
                    .join(format!(".config/gits/{}.toml", host));

                if !inventory_path.exists() {
                    anyhow::bail!(
                        "No inventory file found at {}.",
                        inventory_path.display()
                    );
                }
                
                let raw = fs::read_to_string(&inventory_path)
                    .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", inventory_path.display(), e))?;

                let inventory  = toml::from_str::<InventoryToml>(&raw)
                    .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", inventory_path.display(), e))?.groups;

                Ok((host.clone(), inventory))
            })
            .collect()
    }
    fn get_repos(&self) -> Vec<GitRepo> {
        let mut repos = Vec::new();

        for (host, groups) in self.get_inventory().unwrap() {
            for (slug, projects) in groups {
                for project in projects {
                    repos.push(GitRepo {
                        host: host.clone(),
                        slug: slug.clone(),
                        repo_name: project,
                    });
                }
            }
        }

        repos
    }

    fn get_private_token(&self, host: Host) -> String {
        let token_env_location = self
            .get_remotes_config()
            .unwrap()
            .remotes
            .get(&host)
            .expect("it to work")
            .token
            .clone();

        env::var(token_env_location).expect("can't find auth token")
    }

    fn get_last_sync(&self, host: Host) -> DateTime<Utc> {
        self.get_remotes_config()
            .unwrap()
            .remotes
            .get(&host)
            .expect("it to work")
            .last_pull
            .parse::<DateTime<Utc>>()
            .expect("failed to parse datetime")
    }
    fn get_gitlab_api_url(&self, host: Host) -> String {
        self.get_remotes_config()
            .unwrap()
            .remotes
            .get(&host)
            .expect("it to work")
            .gitlab_api_url
            .clone()
    }
    fn add_to_inventory(&self, git_repo: &GitRepo) {
        let inventory_path = dirs::home_dir()
            .unwrap()
            .join(format!(".config/gits/{}.toml", git_repo.host));
        let mut inventory: Value = toml::from_str(
            &fs::read_to_string(&inventory_path).unwrap_or_else(|_| "[groups]\n".to_string()),
        )
        .expect("Failed to parse config file");

        if let Some(groups) = inventory.get_mut("groups") {
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
                eprintln!("Error mut toml inventory",);
            }
            print!("final inventory: {:?}", &inventory);
        }

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&inventory_path)
            .expect("Failed to open config file");
        file.write_all(
            toml::to_string(&inventory)
                .expect("Failed to serialize config")
                .as_bytes(),
        )
        .expect("Failed to write config file");
    }
}
