mod command;
mod git;

use clap::{Parser, Subcommand};
use crate::command::DebugCommandExecutor;
use crate::command::RealCommandExecutor;
use crate::git::{Git, RealGit};

#[derive(Subcommand)]
enum Commands {
    Status,
    Commit,
}
#[derive(Parser)]
pub struct App {
    #[clap(subcommand)]
    cmd: Commands,
    #[arg(short, long, action)]
    debug: bool,
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
            git.status();
            println!("status")
        }
        Commands::Commit => {
            git.commit("message - need to take from args");
            println!("commit")
        }
    }
}

