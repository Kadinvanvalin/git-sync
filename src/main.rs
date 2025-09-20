mod command;
mod dolly;
mod git;
mod gitlab;

use crate::command::DebugCommandExecutor;
use crate::command::RealCommandExecutor;
use crate::dolly::{parse_url, project_to_repo, GitRepo};
use crate::git::{Git, Projects, RealGit, SettingsConfig};
use crate::gitlab::{get_all_projects, sparse_clone_projects};
use chrono::{DateTime, Utc};
use clap::{Args, Parser, Subcommand};
use dotenv::dotenv;
use skim::options::SkimOptionsBuilder;
use skim::{Skim, SkimItem, SkimItemReceiver, SkimItemSender, SkimOptions};
use std::borrow::Cow;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::{env, fs};
use toml::Value;
#[derive(Args, Debug)]
struct CommitMessage {
    #[clap(trailing_var_arg = true)]
    commit_message: Vec<String>,
}

/// git-sync keeps lots of Git repos up to date with one command.
/// Point it at a directory (or read from a config), and it will discover repositories, check for uncommitted changes, and run the appropriate Git operations (pull/push/fetch) across them.
/// It favors safety (dry-run by default, dirty-tree guards) and clarity (one compact report at the end), so you can automate daily updates without surprises.
#[derive(Subcommand)]
#[command(version, about, long_about = None)]
enum Commands {
    Status,
    Commit(CommitMessage),
    Remote,
    // Syncs the list of projects from your remote to local for faster project search
    Sync,
    // Lists projects for quicker clone and/or remote viewing
    List,
    SyncWatched,
    Prototype,
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
    // Enable debug logging (repeat for more verbosity)
    #[arg(short, long, action)]
    dryrun: bool,
    #[arg(short, long, action)]
    gitlab: bool,
    #[arg(short, long, action)]
    output: bool,
}

#[tokio::main]
async fn main() {
    let args = App::parse();

    let git = if args.dryrun {
        println!("running in dryrun mode");
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
            git.commit(message.commit_message.join(" ").as_str())
                .expect("TODO: panic message");
            git.push();
            println!("commit")
        }
        Commands::Remote => {
            if args.output {
                print!("{}", git.remote());
            } else {
                if args.dryrun {
                    println!("DRY RUN:: command open remote {:?}", git.remote().as_str())
                } else {
                    Command::new("open")
                        .arg(git.remote().as_str())
                        .output()
                        .expect(
                            format!("Failed to open remote  {:?}", git.remote().as_str()).as_str(),
                        );
                }
            }
        }
        Commands::SyncWatched => {
            let config_path = dirs::home_dir().unwrap().join(".config/gits/config.toml");
            let config: SettingsConfig = toml::from_str(
                &fs::read_to_string(config_path).expect("Failed to SettingsConfig config file"),
            )
            .expect("Failed to parse SettingsConfig file");
            let host = config.remotes.keys().next().expect("it to work"); // defaulting to one host
            let groups_path = dirs::home_dir()
                .unwrap()
                .join(format!(".config/gits/{}-watched.toml", host));
            let projects = toml::from_str::<Projects>(
                &fs::read_to_string(groups_path).expect("Failed to read projects file"),
            )
            .expect("Failed to parse projects file")
            .groups;
            for (slug, group) in &projects {
                for project in group {
                    let repo = GitRepo {
                        slug: slug.to_string(),
                        repo_name: project.to_string(),
                        host: host.to_string(),
                    };
                    //.git/FETCH_HEAD
                    let fetch_head_path = dirs::home_dir().unwrap().join(format!(
                        "{}/{}/{}/.git",
                        repo.host, repo.slug, repo.repo_name
                    ));
                    // maybe check if dir exists and delete if not a repo? idk
                    if !Path::new(&fetch_head_path).is_dir() {
                        println!("cloning {:?}", repo);
                        git.clone_repo(&repo);
                    }
                }
            }
        }
        Commands::Sync => {
            // skim with all remotes? autopick if only one?
            if args.gitlab {
                dotenv().ok(); // Load environment variables from .env file
                let config_path = dirs::home_dir().unwrap().join(".config/gits/config.toml");

                let config: SettingsConfig = toml::from_str(
                    &fs::read_to_string(config_path).expect("Failed to SettingsConfig config file"),
                )
                .expect("Failed to parse SettingsConfig file");

                let host = config.remotes.keys().next().expect("it to work"); // defaulting to one host
                                                                              //ISO 8601.
                let last_pull = &config
                    .remotes
                    .get(host)
                    .expect("it to work")
                    .last_pull
                    .parse::<DateTime<Utc>>()
                    .expect("failed to parse datetime");

                let gitlab_api_url = &config.remotes.get(host).expect("it to work").gitlab_api_url;
                let token_env_location = &config.remotes.get(host).expect("it to work").token;
                let private_token = env::var(&token_env_location)
                    .expect(&format!("can't find token {}", &token_env_location));
                let squad = config.remotes.get(host).unwrap().watch_groups.join(",");
                let projects = get_all_projects(gitlab_api_url, &private_token, last_pull)
                    .await
                    .unwrap();
                let repos = project_to_repo(projects);
                sparse_clone_projects(repos).await;
            }
        }
        Commands::List => {
            let config_path = dirs::home_dir().unwrap().join(".config/gits/config.toml");
            let config: SettingsConfig = toml::from_str(
                &fs::read_to_string(config_path).expect("Failed to SettingsConfig config file"),
            )
            .expect("Failed to parse SettingsConfig file");
            let host = config.remotes.keys().next().expect("it to work"); // defaulting to one host
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
            let (tx, rx): (SkimItemSender, SkimItemReceiver) = skim::prelude::unbounded();
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
    let groups_path = dirs::home_dir()
        .unwrap()
        .join(format!(".config/gits/{}.toml", host));
    let projects = toml::from_str::<Projects>(
        &fs::read_to_string(groups_path).expect("Failed to read projects file"),
    )
    .expect("Failed to parse projects file")
    .groups;

    let options = SkimOptionsBuilder::default()
        .prompt("Select an option > ".parse().unwrap()) // Set a custom prompt
        .height("50%".parse().unwrap()) // Restrict height (optional)
        .multi(false) // Disable multi-select
        .build()
        .unwrap();
    let (tx, rx): (SkimItemSender, SkimItemReceiver) = skim::prelude::unbounded();

    for (slug, group) in &projects {
        for project in group {
            tx.send(Arc::new(slug.clone() + "/" + project))
                .expect("TODO: panic message");
        }
    }
    drop(tx); // Close the sender to indicate no more items will be sent

    let x = Skim::run_with(&options, Some(rx));
    let binding = x.expect("should have worked");
    if binding.is_abort {
        println!("received escape code. exiting");
        std::process::exit(0);
    }
    let binding = binding
        .selected_items
        .iter()
        .map(|item| item.output())
        .collect::<Vec<_>>();
    let repo = parse_url(&format!("git@{}:{}.git", host, binding[0]));

    println!("{:?}", repo);

    let (tx, rx): (SkimItemSender, SkimItemReceiver) = skim::prelude::unbounded();
    let commands = vec![ProjectOptions::Remote, ProjectOptions::Clone];
    for command in commands {
        tx.send(Arc::new(command)).expect("TODO: panic message");
    }
    drop(tx); // Close the sender to indicate no more items will be sent

    let command = Skim::run_with(&options, Some(rx));
    let binding = command.expect("should have worked");
    if binding.is_abort {
        println!("received escape code. exiting");
        std::process::exit(0);
    }
    let selected_command = binding.selected_items.get(0).unwrap();

    print!("selectedCommand: {}", selected_command.text());

    match selected_command.text().as_ref() {
        "Readme" => {
            print!("TODO multiple readme names");
            Command::new("open")
                .arg(&format!(
                    "https://{}/{}/{}{}",
                    repo.host, repo.slug, repo.repo_name, "/blob/master/README.md"
                ))
                .output()
                .expect("TODO: panic message");
        }
        "Remote" => {
            print!(
                "opening remote {}",
                &format!("https://{}/{}/{}", repo.host, repo.slug, repo.repo_name)
            );
            Command::new("open")
                .arg(&format!(
                    "https://{}/{}/{}",
                    repo.host, repo.slug, repo.repo_name
                ))
                .output()
                .expect("TODO: panic message");
        }
        "Clone" => {
            println!("trying to CD!!");
            git.clone_repo(&repo);
            add_to_watched_projects(&repo)
            //print!("TODO: idk if its worth it because I can't cd to the location? {}", &format!("https://{}/{}/{}", repo.host, repo.slug, repo.repo_name));
        }
        _ => {}
    }
}

pub fn add_to_watched_projects(git_repo: &GitRepo) {
    let config_path = dirs::home_dir()
        .unwrap()
        .join(format!(".config/gits/{}-watched.toml", git_repo.host));
    let mut config: Value = toml::from_str(
        &fs::read_to_string(&config_path).unwrap_or_else(|_| "[groups]\nprojects = []".to_string()),
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
