mod command;
mod git;
mod gitlab;
mod dolly;
use std::{env, fs};
use std::borrow::Cow;
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use clap::{Args, Parser, Subcommand};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use skim::{Skim, SkimItem, SkimItemReceiver, SkimItemSender, SkimOptions};
use crate::command::DebugCommandExecutor;
use crate::command::RealCommandExecutor;
use crate::dolly::GitRepo;
use crate::git::{Git, Group, Projects, RealGit, SettingsConfig};
use crate::gitlab::{get_all_projects, sparse_clone_projects};
#[derive(Args, Debug)]
struct CommitMessage {
    #[clap(trailing_var_arg=true)]
    commit_message: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    Status,
    Commit(CommitMessage),
    Remote,
    Sync,
    List
}
enum ProjectOptions {
    Readme,
    Remote,
    Clone,
}

impl SkimItem for ProjectOptions {
    fn text(&self) -> Cow<str> {
        match self {
            ProjectOptions::Readme => Cow::Borrowed("Readme"),
            ProjectOptions::Remote => Cow::Borrowed("Remote"),
            ProjectOptions::Clone => Cow::Borrowed("Clone"),
        }
    }
}
#[derive(Parser)]
pub struct App {
    #[clap(subcommand)]
    cmd: Commands,
    #[arg(short, long, action)]
    debug: bool,
    #[arg(short, long, action)]
    gitlab: bool,
}


#[tokio::main]
async fn main() {
    let args = App::parse();

    let git = if args.debug {
        println!("running in debug mode");
        RealGit::new(&DebugCommandExecutor)
    } else {
        RealGit::new(&RealCommandExecutor)
    };

    match args.cmd {
        Commands::Status => {
            git.status().expect("TODO: panic message");
            println!("status")
        }
        Commands::Commit(message) => {
            git.commit(message.commit_message.join(" ").as_str()).expect("TODO: panic message");
            git.push();
            println!("commit")
        }
        Commands::Remote => {
            git.remote();
            // Command::new("open").args("https://github.com".split(" ")).output();
        }
        Commands::Sync =>{
            if args.gitlab {
                dotenv().ok(); // Load environment variables from .env file
                let config_path = dirs::home_dir().unwrap().join(".config/gits/config.toml");

                let config: SettingsConfig = toml::from_str(&fs::read_to_string(config_path)
                    .expect("Failed to SettingsConfig config file")).expect("Failed to parse SettingsConfig file");
                let gitlab_api_url = &config.remotes.get("gitlab").expect("it to work").gitlab_api_url;
                let private_token = env::var("PRIVATE_TOKEN").expect("PRIVATE_TOKEN not set");
                let squad = config.remotes.get("gitlab").unwrap().watch_groups.join(",");
                let projects = get_all_projects(gitlab_api_url, &private_token).await.unwrap();
                sparse_clone_projects(projects).await;
            }

        }
        Commands::List => {
            view_projects();
        }
    }
}

fn view_projects() {
    let groups_path = dirs::home_dir().unwrap().join(".config/gits/gitlab.cj.dev.toml");
    let projects = toml::from_str::<Projects>(&fs::read_to_string(groups_path)
        .expect("Failed to read projects file"))
        .expect("Failed to parse projects file")
        .groups;


    //
    let options = SkimOptions::default();
    let (tx, rx): (SkimItemSender, SkimItemReceiver) = skim::prelude::unbounded();

    for (slug, group) in &projects {
        for project in group {
            tx.send(Arc::new(slug.clone() + "/" + project)).expect("TODO: panic message");
        }
    }
    drop(tx); // Close the sender to indicate no more items will be sent

    let x = Skim::run_with(&options, Some(rx));
    let binding = x.expect("should have worked");
    let binding = binding.selected_items.iter().map(|item| item.output()).collect::<Vec<_>>();
    let selectedProject = binding[0].split("/");
    let repo = GitRepo {
        host: String::from("gitlab.cj.dev"),
        slug: String::from(selectedProject.clone().collect::<Vec<_>>()[0]),
        repo_name: String::from(selectedProject.clone().collect::<Vec<_>>()[1]),
    };


    println!("{:?}", selectedProject);


    let options = SkimOptions::default();
    let (tx, rx): (SkimItemSender, SkimItemReceiver) = skim::prelude::unbounded();
    let commands = vec![ProjectOptions::Readme, ProjectOptions::Remote];
    for command in commands {
            tx.send(Arc::new(command)).expect("TODO: panic message");
    }
    drop(tx); // Close the sender to indicate no more items will be sent

    let command = Skim::run_with(&options, Some(rx));
    let binding = command.expect("should have worked");
    let selected_command = binding.selected_items.get(0).unwrap();

    print!("selectedCommand: {}", selected_command.text());

    match selected_command.text().as_ref() {
        "Readme" => {
            print!("TODO multiple readme names");
            Command::new("open").arg(
                &format!("https://{}/{}/{}{}", repo.host, repo.slug, repo.repo_name, "/blob/master/README.md")
            ).output().expect("TODO: panic message");
         }
       "Remote" => {
            print!("opening remote {}", &format!("https://{}/{}/{}", repo.host, repo.slug, repo.repo_name));
            Command::new("open").arg(
                &format!("https://{}/{}/{}", repo.host, repo.slug, repo.repo_name)
            ).output().expect("TODO: panic message");
        }
        "Clone" => {
            print!("TODO: idk if its worth it because I can't cd to the location? {}", &format!("https://{}/{}/{}", repo.host, repo.slug, repo.repo_name));
           
        }
        _ => {}
    }

}


