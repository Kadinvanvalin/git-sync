mod command_executor;
mod config;
mod dolly;
mod git;
mod gitlab;
mod list;
mod github;

use crate::command_executor::DebugCommandExecutor;
use crate::command_executor::RealCommandExecutor;
use crate::config::{GitsConfig, RealGitsConfig};
use crate::dolly::{project_to_repo, GitRepo};
use crate::git::{Git, HostKind, RealGit};
use crate::gitlab::get_all_gitlab_projects;
use clap::{Args, Parser, Subcommand};
use dotenv::dotenv;
use skim::SkimItem;
use std::borrow::Cow;
use std::io::Write;
use std::path::Path;
use chrono::{DateTime, Utc};
use crate::github::get_watched_github_projects;

#[derive(Args, Debug)]
struct CommitMessage {
    #[clap(trailing_var_arg = true)]
    commit_message: Vec<String>,
}

/// git-sync keeps lots of Git repos up to date with one command.
/// Point it at a directory (or read from a config), and it will discover repositories, check for uncommitted changes, and run the appropriate Git operations (pull/push/fetch) across them.
/// It favors safety (dry-run by default, dirty-tree guards) and clarity (one compact report at the end), so you can automate daily updates without surprises.
/// you can think of repos as branches - fetch gets all new repos, pull pulls everything
#[derive(Subcommand)]
#[command(version)]
enum Commands {
    #[command(about = "true status - git fetch and status")]
    Status,
    #[command(about = "you probably want to pull first? yeah, we are doing that for you")]
    Commit(CommitMessage),
    #[command(about = "opens the repo in browser")]
    Remote,
    #[command(about = "gets all new projects from gitlab and puts in a toml for faster search")]
    Sync,
    #[command(
        about = "list of all projects gits knows about - so you can remote or clone them directly"
    )]
    List,
    #[command(about = "git pull on all watched projects")]
    SyncWatched,
}

#[derive(Parser)]
pub struct App {
    #[clap(subcommand)]
    cmd: Commands,
    // Enable debug logging (repeat for more verbosity)
    #[arg(short, long, action)]
    dryrun: bool,
    // outputs to stdout instead of opening in browser.
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
    let config: RealGitsConfig = GitsConfig::build();
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
                git.get_remote_url();
            } else {
                git.remote();
            }
        }
        Commands::SyncWatched => {
            config.get_repos().iter().for_each(|repo| {
                let fetch_head_path = dirs::home_dir().unwrap().join(format!(
                    "{}/{}/{}/.git",
                    repo.host, repo.slug, repo.repo_name
                ));
                // maybe check if dir exists and delete if not a repo? idk
                if !Path::new(&fetch_head_path).is_dir() {
                    println!("cloning {:?}", repo);
                    git.clone_repo(&repo);
                }
            });
        }
        Commands::Sync => {
            dotenv().ok(); // Load environment variables from .env file
            let remotes = config.get_remotes_config().unwrap();
            let mut repos = Vec::new();
            
            for (host, remote_settings) in remotes.remotes {
                if remote_settings.host_kind == HostKind::GitLab {
                    let response = get_all_gitlab_projects(
                        &*remote_settings.api_url,
                        &*config.get_private_token(host.clone()),
                        &remote_settings
                            .last_pull
                            .parse::<DateTime<Utc>>()
                            .expect("failed to parse json created_at")
                    ).await;
                    repos.push(response.unwrap())
                }
                if remote_settings.host_kind == HostKind::GitHub {
                    let response = get_watched_github_projects(
                        &*remote_settings.api_url,
                        &*config.get_private_token(host.clone()),
                        &remote_settings
                            .last_pull
                            .parse::<DateTime<Utc>>()
                            .expect("failed to parse json created_at"),
                        "Kadinvanvalin".to_string(),
                        
                    ).await;
                    repos.push(response.unwrap())
                }
              
            }
            let repos = repos.iter().flatten().collect::<Vec<_>>();
            let repos = project_to_repo(repos);
            
            repos
                .iter()
                .for_each(|repo| config.add_to_inventory(repo).unwrap());
            
        }
        Commands::List => {
            list::view_projects(&git, &config);
        }
    }
}

