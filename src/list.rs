use crate::config::{GitsConfig, RealGitsConfig};
use crate::git::{parse_url, Git, GitRepo, RealGit};
use anyhow::{anyhow, bail, Context, Result};
use skim::options::SkimOptionsBuilder;
use skim::prelude::*;
use skim::{Skim, SkimItem, SkimItemReceiver, SkimItemSender, SkimOutput};
use std::borrow::Cow;
use std::process::Command;
use std::sync::Arc;

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
fn pick_repo(out: SkimOutput) -> anyhow::Result<GitRepo> {
    let item = out
        .selected_items
        .first()
        .ok_or_else(|| anyhow::anyhow!("No item selected"))?;
    let s = item.output();

    let (host, repo_part) = s
        .split_once(' ')
        .ok_or_else(|| anyhow::anyhow!("Invalid skim output: {s}"))?;

    let repo = parse_url(&format!("git@{}:{}.git", host, repo_part));

    Ok(repo)
}

pub fn view_projects(git: &RealGit, config: &RealGitsConfig) {
    loop {
        let inventory_map = config.get_inventory().unwrap();

        let options = SkimOptionsBuilder::default()
            .prompt("Select an option > ".parse().unwrap()) // Set a custom prompt
            .height("50%".parse().unwrap()) // Restrict height (optional)
            .multi(false) // Disable multi-select
            .build()
            .unwrap();

        // 3) Feed items
        let (tx, rx): (SkimItemSender, SkimItemReceiver) = skim::prelude::unbounded();
        for (host, groups) in &inventory_map {
            for (slug, projects) in groups {
                for project in projects {
                    // no need to clone slug/project; Display handles &String
                    let line = format!("{host} {slug}/{project}");
                    // String implements SkimItem, so Arc<String> works
                    if tx.send(Arc::new(line)).is_err() {
                        // receiver gone; stop sending
                        break;
                    }
                }
            }
        }
        drop(tx); // tell skim there’s no more input

        // 4) Run skim and read selection
        let out = Skim::run_with(&options, Some(rx));
        // let command = Skim::run_with(&options, Some(rx));
        let out = out
            .ok_or_else(|| anyhow::anyhow!("No skim output (user aborted?)"))
            .unwrap();
        if out.is_abort {
            println!("received escape code. exiting");
            std::process::exit(0);
        }
        let repo: anyhow::Result<GitRepo> = pick_repo(out);

        let repo = match repo {
            Ok(r) => r,
            Err(_) => {
                println!("received escape code. exiting");
                std::process::exit(0);
            }
        };

        let action = run_repo_actions();
        match action {
            Ok(action) => match action.as_str() {
                "Remote" => {
                    let url = format!("https://{}/{}/{}", repo.host, repo.slug, repo.repo_name);
                    open_url(&url).unwrap();
                    println!("Opened: {url}");
                }
                "Clone" => {
                    println!("trying to CD!!");
                    git.clone_repo(&repo);
                    config.add_to_inventory(&repo).unwrap();
                }
                other => panic!("unknown action: {other}"),
            },
            Err(_) => continue,
        }
    }
}

fn run_repo_actions() -> Result<String> {
    // 1) Build a small command palette
    let options = SkimOptionsBuilder::default()
        .prompt("Action > ".to_string())
        .height("40%".to_string())
        .multi(false)
        .build()?;

    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();
    for label in ["Remote", "Clone"] {
        // Arc<String> implements SkimItem
        if tx.send(Arc::new(label)).is_err() {
            bail!("failed to send skim item");
        }
    }
    drop(tx); // no more items

    // 3) Run skim
    let out = Skim::run_with(&options, Some(rx)).ok_or_else(|| anyhow!("skim failed to run"))?;

    let item = out
        .selected_items
        .first()
        .ok_or_else(|| anyhow::anyhow!("No item selected"))?;

    if out.is_abort {
        return Err(anyhow::anyhow!("No item selected"));
    }

    let s = item.output();

    Ok(s.to_string())
}

// Cross-platform URL opener `webbrowser` crate is option
fn open_url(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .status()
            .context("open failed")?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(url)
            .status()
            .context("xdg-open failed")?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", url])
            .status()
            .context("start failed")?;
    }
    Ok(())
}
