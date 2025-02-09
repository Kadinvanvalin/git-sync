use std::process::{Command, ExitStatus, Output};

pub fn command_success(command: &str, args: &str) -> bool {
    Command::new(command)
    .args(args.split(" "))
    .output()
    .expect("failed to call command").status.success()
}

pub fn run_command(command: &str, args: &str) -> String {
    let output = Command::new(command)
        .args(args.split(" "))
        .output()
        .expect("failed to call command");
    
    
    match output.status.success() {
        true => {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        },
        false  => {
            Err("opps")
        }
    }.expect(&format!("Failed to execute command: {} {}",
                      command,
                      args))
    
    
}