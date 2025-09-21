mod command_executor;
mod config;
mod dolly;
mod git;
mod gitlab;

use crate::command_executor::DebugCommandExecutor;
use crate::command_executor::RealCommandExecutor;
use crate::config::{GitsConfig, RealGitsConfig};
use crate::dolly::{parse_url, project_to_repo};
use crate::git::{Git, RealGit};
use crate::gitlab::get_all_projects;
use clap::{Args, Parser, Subcommand};
use dotenv::dotenv;
use skim::options::SkimOptionsBuilder;
use skim::{Skim, SkimItem, SkimItemReceiver, SkimItemSender};
use std::borrow::Cow;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

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
enum ProjectOptions {
    Remote,
    Clone,
}

impl SkimItem for ProjectOptions {
    fn text(&self) -> Cow<str> {
        match self {
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

            let projects = get_all_projects(
                &*config.get_gitlab_api_url(),
                &*config.get_private_token(),
                &config.get_last_sync(),
            )
            .await
            .unwrap();
            let repos = project_to_repo(projects);
            repos.iter().for_each(|repo| config.add_to_global(repo));
        }
        Commands::List => {
            view_projects(&git, &config);
        }
    }
}

fn view_projects(git: &RealGit, config: &RealGitsConfig) {
    let options = SkimOptionsBuilder::default()
        .prompt("Select an option > ".parse().unwrap()) // Set a custom prompt
        .height("50%".parse().unwrap()) // Restrict height (optional)
        .multi(false) // Disable multi-select
        .build()
        .unwrap();
    let (tx, rx): (SkimItemSender, SkimItemReceiver) = skim::prelude::unbounded();

    for (slug, group) in &config.get_projects() {
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
    let repo = parse_url(&format!("git@{}:{}.git", config.host, binding[0]));

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
            config.add_to_global(&repo)
        }
        _ => {}
    }
}
