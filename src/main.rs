mod command;
mod git;

use clap::{Parser, Subcommand};

#[derive(Subcommand)]
enum Commands {
    Status,
    Commit,
}
#[derive(Parser)]
pub struct App {
    #[clap(subcommand)]
    cmd: Commands,
}


#[tokio::main]
async fn main() {
    let args = App::parse();
    
    match args.cmd { 
        Commands::Status => {
            println!("status")
        }
        Commands::Commit => {
            git::commit();
            println!("commit")
        }
    }
}

