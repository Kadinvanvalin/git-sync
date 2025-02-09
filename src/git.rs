use std::borrow::Cow;
use std::fmt::format;
use std::process::Command;
use crate::command;
use crate::command::{command_success, run_command};

pub(crate) async fn fetch() -> std::process::Output {
    command::run_command("git", "fetch origin");
    Command::new("git")
        .arg("fetch")
        // .current_dir()
        .output()
        .expect("TODO: panic message")
     
}

pub(crate) fn status() -> std::process::Output {
    Command::new("git")
        .arg("status")
        .arg("-s")
        // .current_dir()
        .output()
        .expect("TODO: panic message")

}
pub(crate) fn merge_base(trunk: String) -> String{
    let command = format!("merge-base {0} origin/{0}", trunk);
    run_command("git", &command)
}
pub(crate) fn commit() {
    let trunk = find_trunk();
    let last_shared_commit = merge_base(trunk.clone());
    let rev_parse = last_commit_trunk(trunk.clone());
}

pub(crate) fn last_commit_trunk(trunk: String) -> String {
    run_command("git", &format!("rev-parse {}",trunk))
}
pub(crate) fn find_trunk() -> String {
    let possible_trunks = ["main", "master"];
    for trunk in &possible_trunks {
        let remote_trunk = format!("origin/{}", trunk);
        //show-ref --verify ref/head/
        // will panic? maybe broke it
        let exists = command_success("git", &format!("show-ref --verify refs/heads/{}", trunk));
        if !exists {
            continue
        }
        return trunk.to_string();
    }
    let assumption1 = "assuming remote is origin";
    let assumption2 = "assuming we don't use master AND main";
    let assumption3 = "assuming  one is trunk";
    panic!("Something happened while looking for trunk: {:?}. Some Assumption: {}, {}, {}", [possible_trunks], assumption1, assumption2, assumption3)
}