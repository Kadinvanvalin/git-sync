use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct Project {
    pub ssh_url_to_repo: String,
    pub created_at: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Projects {
    pub(crate) groups: HashMap<String, Vec<String>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SettingsConfig {
    pub remotes: HashMap<String, RemoteSettings>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")] // maps GitHub -> "github", GitLab -> "gitlab"
pub enum HostKind {
    GitHub,
    GitLab,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct RemoteSettings {
    pub token: String,
    pub project_directory: String,
    pub api_url: String,
    pub watch_groups: Vec<String>,
    pub watch_projects: Vec<String>,
    pub last_pull: String,
    pub host_kind: HostKind,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Group {
    projects: Vec<String>,
}
pub trait Git {
    fn commit(&self, message: &str) -> Result<(), String>;
    fn status(&self) -> Result<String, String>;
    fn remote(&self) -> ();
    fn push(&self) -> ();
    fn clone_repo(&self, repo: &GitRepo) -> ();
    fn get_remote_url(&self) -> String;
}

use crate::command_executor::CommandExecutor;

pub struct RealGit<'a> {
    executor: &'a dyn CommandExecutor, // Reference to the executor
}

impl<'a> RealGit<'a> {
    pub fn new(executor: &'a dyn CommandExecutor) -> Self {
        Self { executor }
    }
}

impl<'a> Git for RealGit<'a> {
    fn clone_repo(&self, repo: &GitRepo) -> () {
        let home_dir = dirs::home_dir().unwrap();
        self.executor.run_command(
            "mkdir",
            &format!("-p {}/{}/{}", home_dir.display(), repo.host, repo.slug),
        );
        let clone = &format!(
            "clone git@{1}:{2}/{3}.git {0}/{1}/{2}/{3}",
            home_dir.display(),
            repo.host,
            repo.slug,
            repo.repo_name
        );
        self.executor.run_command("git", clone);
        println!(
            "cd {}/{}/{}/{}",
            home_dir.display(),
            repo.host,
            repo.slug,
            repo.repo_name
        );
    }

    fn push(&self) -> () {
        let stdout = self.executor.run_command("git", "push");
        println!("Pushing: {}", stdout)
    }

    fn remote(&self) -> () {
        self.executor
            .run_command("open", self.get_remote_url().as_str());
    }
    fn get_remote_url(&self) -> String {
        let url = self.executor.run_command("git", "remote get-url origin");

        if valid_ssh_url(&*url) {
            make_url(&url)
        } else {
            (&*url).to_string()
        }
    }
    fn commit(&self, message: &str) -> Result<(), String> {
        let trunk = find_trunk(self.executor);

        self.executor
            .run_command("git", &format!("fetch origin {0}", trunk));
        let merge_base = format!("merge-base HEAD origin/{0}", trunk);

        let last_shared_commit = self.executor.run_command("git", &merge_base);

        let last_commit_trunk = self
            .executor
            .run_command("git", &format!("rev-parse origin/{}", trunk));

        if last_shared_commit == last_commit_trunk {
            println!("git commit -m {}", message);
            self.executor
                .run_explicit_command("git", vec!["commit", "-m", format!("{}", message).as_str()]);
            Ok(())
        } else {
            Err("okok".to_string())
        }
    }

    fn status(&self) -> Result<String, String> {
        Ok(self.executor.run_command("git", "status"))
    }
}
pub(crate) fn find_trunk(executor: &dyn CommandExecutor) -> String {
    let possible_trunks = ["main", "master"];
    for trunk in &possible_trunks {
        let exists =
            executor.command_success("git", &format!("show-ref --verify refs/heads/{}", trunk));
        if !exists {
            continue;
        }
        return trunk.to_string();
    }
    let assumption1 = "assuming remote is origin";
    let assumption2 = "assuming we don't use master AND main";
    let assumption3 = "assuming  one is trunk";
    panic!(
        "Something happened while looking for trunk: {:?}. Some Assumption: {}, {}, {}",
        [possible_trunks],
        assumption1,
        assumption2,
        assumption3
    )
}

#[derive(PartialEq, Debug)]
pub struct GitRepo {
    pub host: String,
    pub slug: String,
    pub repo_name: String,
}
pub fn valid_ssh_url(url: &str) -> bool {
    let matches = Regex::new(r"(git)@([^/:]+):([^/:]+)/(.+)(.git)");

    match matches {
        Ok(content) => content.is_match(url),
        Err(_) => false,
    }
}

pub fn make_url(url: &str) -> String {
    make_url_private(parse_url(url))
}
pub fn make_url_private(git_repo: GitRepo) -> String {
    String::from(&format!(
        "https://{}/{}/{}",
        git_repo.host, git_repo.slug, git_repo.repo_name
    ))
}
pub fn parse_url(url: &str) -> GitRepo {
    let re = Regex::new(r"(git)@([^/:]+):(.+)/([^/:]+)(.git)").expect("failed to parse regex");

    let caps = re.captures(&url).unwrap();
    let host = caps.get(2).map_or("", |m| m.as_str());
    let slug = caps.get(3).map_or("", |m| m.as_str());
    let repo_name = caps.get(4).map_or("", |m| m.as_str());
    GitRepo {
        host: host.parse().unwrap(),
        slug: slug.parse().unwrap(),
        repo_name: repo_name.parse().unwrap(),
    }
}

pub fn project_to_repo(projects: Vec<Project>) -> Vec<GitRepo> {
    projects
        .iter()
        .map(|p| parse_url(&*p.ssh_url_to_repo))
        .collect()
}
