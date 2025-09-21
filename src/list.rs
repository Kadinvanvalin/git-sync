use std::borrow::Cow;
use skim::options::SkimOptionsBuilder;
use skim::{Skim, SkimItem, SkimItemReceiver, SkimItemSender};
use std::sync::Arc;
use std::process::Command;
use crate::config::{GitsConfig, RealGitsConfig};
use crate::dolly::parse_url;
use crate::git::{Git, RealGit};

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


pub fn view_projects(git: &RealGit, config: &RealGitsConfig) {
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