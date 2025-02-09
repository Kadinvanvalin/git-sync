use std::fmt::format;
use std::process::Command;

pub(crate) async fn fetch() -> std::process::Output {
    run_command("git", ["fetch"]);
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

pub(crate) fn x() -> String {
    let possible_trunks = ["main", "master"];
    for trunk in &possible_trunks {
        // TODO assuming remote is origin
        // TODO assuming we don't use master AND main and one is trunk
        let remote_trunk = format!("origin/{}", trunk);
        
        let trunk_exists = Command::new("git")
            .arg("show-ref")
            .arg("--verify")
            .arg(format!("ref/head/{}", trunk))
            .output()
            .expect("Failed to check if trunk exists")
            .status
            .success();
        
        if !trunk_exists {
            continue
        }
        return trunk.to_string();
    }
    let assumption1 = "assuming remote is origin";
    let assumption2 = "assuming we don't use master AND main";
    let assumption3 = "assuming  one is trunk";
    panic!("Something happened while looking for trunk: {:?}. Some Assumption: {}, {}, {}", [possible_trunks], assumption1, assumption2, assumption3)
}