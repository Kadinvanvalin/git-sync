use std::process::{Command, Output};
use std::io;

fn run_command(command: &str, args: &[&str]) -> io::Result<Output> {
    let output = Command::new(command)
        .args(args)
        .output();

    match output {
        Ok(output) => Ok(output),
        Err(e) => {
            eprintln!(
                "Failed to execute command: {} {}",
                command,
                args.join(" ")
            );
            Err(e)
        }
    }
}