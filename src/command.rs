use std::process::{Command};


pub trait CommandExecutor {
    fn run_command(&self, command: &str, args: &str) -> String;
    fn command_success(&self, command: &str, args: &str) -> bool;
    fn run_explicit_command(&self, command: &str, args: Vec<&str>) -> String;
}
pub struct RealCommandExecutor;

impl CommandExecutor for RealCommandExecutor {
 


    fn command_success(&self, command: &str, args: &str) -> bool {
        Command::new(command)
            .args(args.split(" "))
            .output()
            .expect("failed to call command").status.success()
    }

    fn run_command(&self, command: &str, args: &str) -> String {
        let output = Command::new(command)
            .args(args.split(" "))
            .output()
            .expect("failed to call command");


        match output.status.success() {
            true => {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            },
            false => {
                Err(&output.stderr)
            }
        }.expect(&format!("Failed to execute command: {} {}",
                          command,
                          args))


    }

    fn run_explicit_command(&self, command: &str, args: Vec<&str>) -> String {
        let output = Command::new(command)
            .args(&args)
            .output()
            .expect("failed to call command");


        match output.status.success() {
            true => {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            },
            false => {
                Err(&output.stderr)
            }
        }.expect(&format!("Failed to execute command: {} {:?}",
                          command,
                          args))


    }
}



pub struct DebugCommandExecutor;

impl CommandExecutor for DebugCommandExecutor {
    fn run_command(&self, command: &str, args: &str) -> String {
        println!("DEBUG: Simulating execution of `{} {}`", command, args);
        "mocked output".to_string()
    }

    fn command_success(&self, command: &str, args: &str) -> bool {
        println!("DEBUG: Pretending `{}` with args `{}` succeeded", command, args);
        true // Always return success during debug mode
    }

    fn run_explicit_command(&self, command: &str, args:Vec<&str> ) -> String {
        todo!()
    }
}
