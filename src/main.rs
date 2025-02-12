mod command;
mod git;
mod gitlab;
mod dolly;


use std::{env, fs};
use std::borrow::Cow;
use std::io::Cursor;

use std::process::Command;
use std::sync::Arc;
use clap::{Args, Parser, Subcommand};
use dotenv::dotenv;
use skim::{Skim, SkimItem, SkimItemReceiver, SkimItemSender, SkimOptions};
use skim::options::SkimOptionsBuilder;
use crate::command::DebugCommandExecutor;
use crate::command::RealCommandExecutor;
use crate::dolly::{parse_url, project_to_repo, GitRepo};
use crate::git::{Git, Projects, RealGit, SettingsConfig};
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
    List, 
    Prototype
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
            let status = git.status().expect("TODO: panic message");
            println!("{}", status)
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
            let host = "localhost";
            // skim with all remotes? autopick if only one?
            if args.gitlab {
                dotenv().ok(); // Load environment variables from .env file
                let config_path = dirs::home_dir().unwrap().join(".config/gits/config.toml");

                let config: SettingsConfig = toml::from_str(&fs::read_to_string(config_path)
                    .expect("Failed to SettingsConfig config file")).expect("Failed to parse SettingsConfig file");
                let gitlab_api_url = &config.remotes.get(host).expect("it to work").gitlab_api_url;
                let token_env_location = &config.remotes.get(host).expect("it to work").token;
                let private_token = env::var(&token_env_location).expect("can't find token");
                let squad = config.remotes.get(host).unwrap().watch_groups.join(",");
                let projects = get_all_projects(gitlab_api_url, &private_token).await.unwrap();
                let repos = project_to_repo(projects);
                sparse_clone_projects(repos).await;
            }

        }
        Commands::List => {
            let host = "localhost";
            // skim with all remotes? autopick if only one?
            // maybe load ALL?
            view_projects(&git, host);
        }
        Commands::Prototype => {
            let input_items = vec!["Option 1", "Option 2", "Option 3"];
            let input = input_items.join("\n"); // Create a single input string joined by newlines

            // Configure skim
            let options = SkimOptionsBuilder::default()
                .prompt("Select an option > ".parse().unwrap()) // Set a custom prompt
                .height("50%".parse().unwrap()) // Restrict height (optional)
                .multi(false) // Disable multi-select
                .build()
                .unwrap();
            let (tx, rx): (SkimItemSender, SkimItemReceiver) =  skim::prelude::unbounded();
            // Send items into the channel
            for item in input_items {
                tx.send(Arc::new(item.to_string())).unwrap(); // Wrap each item in Arc<String>
            }
            drop(tx); // Close the sender so skim knows no more items will be sent

            // Run skim
            let selected_items = Skim::run_with(&options, Some(rx))
                .map(|out| out.selected_items) // Get selected items
                .unwrap_or_else(|| Vec::new()); // Fallback to empty vector if nothing selected

            // Process the result
            if let Some(selected_item) = selected_items.get(0) {
                println!("You selected: {}", selected_item.output());
            } else {
                println!("No selection made");
            }

        }
    }
}


fn view_projects(git: &RealGit, host: &str) {
    
    
    let groups_path = dirs::home_dir().unwrap().join(format!(".config/gits/{}.toml", host));
    let projects = toml::from_str::<Projects>(&fs::read_to_string(groups_path)
        .expect("Failed to read projects file"))
        .expect("Failed to parse projects file")
        .groups;


    let options = SkimOptionsBuilder::default().prompt("Select an option > ".parse().unwrap()) // Set a custom prompt
        .height("50%".parse().unwrap()) // Restrict height (optional)
        .multi(false) // Disable multi-select
        .build()
        .unwrap();
    let (tx, rx): (SkimItemSender, SkimItemReceiver) = skim::prelude::unbounded();

    for (slug, group) in &projects {
        for project in group {
            tx.send(Arc::new(slug.clone() + "/" + project)).expect("TODO: panic message");
        }
    }
    drop(tx); // Close the sender to indicate no more items will be sent

    let x = Skim::run_with(&options, Some(rx));
    let binding = x.expect("should have worked");
    if  binding.is_abort {
        println!("received escape code. exiting");
        std::process::exit(0);
    }
    let binding = binding.selected_items.iter().map(|item| item.output()).collect::<Vec<_>>();
    let repo = parse_url(&format!("git@{}:{}.git",host, binding[0]));
    


    println!("{:?}", repo);
    
    
    let (tx, rx): (SkimItemSender, SkimItemReceiver) = skim::prelude::unbounded();
    let commands = vec![ProjectOptions::Remote, ProjectOptions::Clone];
    for command in commands {
            tx.send(Arc::new(command)).expect("TODO: panic message");
    }
    drop(tx); // Close the sender to indicate no more items will be sent

    let command = Skim::run_with(&options, Some(rx));
    let binding = command.expect("should have worked");
    if  binding.is_abort {
        println!("received escape code. exiting");
        std::process::exit(0);
    }
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
            println!("trying to CD!!");
            git.clone_repo(&repo);
            //print!("TODO: idk if its worth it because I can't cd to the location? {}", &format!("https://{}/{}/{}", repo.host, repo.slug, repo.repo_name));

        }
        _ => {}
    }

}


