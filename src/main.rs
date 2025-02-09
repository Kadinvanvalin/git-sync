mod command;
mod git;
mod gitlab;
mod dolly;

use std::{env, fs};

use clap::{Args, Parser, Subcommand};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use crate::command::DebugCommandExecutor;
use crate::command::RealCommandExecutor;
use crate::git::{Git, RealGit, SettingsConfig};
#[derive(Args, Debug)]
struct CommitMessage {
    #[clap(trailing_var_arg=true)]
    commit_message: Vec<String>,
    // #[arg(short, long)]
    // option_for_one: Option<String>,
}
#[derive(Subcommand)]
enum Commands {
    Status,
    Commit(CommitMessage),
    Remote,
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
    if args.gitlab {
        dotenv().ok(); // Load environment variables from .env file
        let config_path = dirs::home_dir().unwrap().join(".config/gits/config.toml");

        let config: SettingsConfig = toml::from_str(&fs::read_to_string(config_path).expect("Failed to read config file")).expect("Failed to parse config file");
        // let gitlab_api_url = config.remotes.get("gitlab").expect("it to work").gitlab_api_url;
        let private_token = env::var("PRIVATE_TOKEN").expect("PRIVATE_TOKEN not set");
        let squad = config.remotes.get("test").unwrap().watch_groups.join(",");

    }
    let git = if args.debug {
        println!("running in debug mode");
        RealGit::new(&DebugCommandExecutor)
    } else {
        RealGit::new(&RealCommandExecutor)
    };

    match args.cmd {
        Commands::Status => {
            git.status();
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
    }
}

