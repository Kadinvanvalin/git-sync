use std::collections::HashMap;
use serde::{Deserialize, Serialize};




#[derive(Deserialize, Debug)]
pub struct Project {
    pub ssh_url_to_repo: String,
    pub path_with_namespace: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct Projects {
    groups: HashMap<String, Group>,

}

#[derive(Deserialize, Serialize, Debug)]
pub struct SettingsConfig {
    pub remotes: HashMap<String, RemoteSettings>,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct RemoteSettings {
    pub project_directory: String,
    pub gitlab_api_url: String,
    pub watch_groups: Vec<String>,
    pub watch_projects: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug)]
struct Group {
    projects: Vec<String>,
}
pub trait Git {
    fn commit(&self, message: &str) -> Result<(), String>;
    fn status(&self) -> Result<String, String>;
    fn remote(&self) -> ();
    fn push(&self) -> ();
}

use crate::command::CommandExecutor;
use crate::dolly::{make_url, valid_ssh_url};

pub struct RealGit<'a> {
    executor: &'a dyn CommandExecutor, // Reference to the executor
}

impl<'a> RealGit<'a> {
    pub fn new(executor: &'a dyn CommandExecutor) -> Self {
        Self { executor }
    }
}

impl<'a> Git for RealGit<'a> {
    fn push(&self) -> () {
        let stdout = self.executor.run_command("git", "push");
        println!("{}", stdout)
    }
    fn remote(&self) {
        let url = self.executor
            .run_command("git", "remote get-url origin");

        if valid_ssh_url(&*url) {

            let url = make_url(&url);
            self.executor
                .run_command("open", &*url);
        } else {
            self.executor
                .run_command("open", &*url);
        }


    }
    fn commit(&self, message: &str)  -> Result<(), String> {
        let trunk = find_trunk(self.executor);

        self.executor.run_command("git", &format!("fetch origin {0}", trunk));
        let merge_base = format!("merge-base HEAD origin/{0}", trunk);

            let last_shared_commit = self.executor
                .run_command("git", &merge_base);

            let last_commit_trunk = self.executor
                .run_command("git", &format!("rev-parse origin/{}", trunk));

            if last_shared_commit == last_commit_trunk {
                println!("git commit -m {}", message);
                self.executor.run_explicit_command("git",
                                                   vec!["commit",  "-m", format!("{}", message).as_str()]
                );
                Ok(())
            } else {
                Err("okok".to_string())
                //drift(last_shared_commit)
            }







    }

    fn status(&self) -> Result<String, String> {
        Ok(self.executor.run_command("git", "status"))
    }
}
pub(crate) fn find_trunk(executor: &dyn CommandExecutor) -> String {
    let possible_trunks = ["main", "master"];
    for trunk in &possible_trunks {
        let exists = executor.command_success("git", &format!("show-ref --verify refs/heads/{}", trunk));
        if !exists {
            continue
        }
        return trunk.to_string();
    }
    let assumption1 = "assuming remote is origin";
    let assumption2 = "assuming we don't use master AND main";
    let assumption3 = "assuming  one is trunk";
    panic!("Something happened while looking for trunk: {:?}. Some Assumption: {}, {}, {}", [possible_trunks], assumption1, assumption2, assumption3)
}