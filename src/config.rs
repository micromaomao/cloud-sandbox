use std::process::Command;

pub fn get_command() -> Command {
  let mut cmd = Command::new("/usr/local/openjdk-11/bin/java");
  cmd.arg("-jar");
  cmd.arg("/tmp/jsh.jar");
  cmd
}

pub const INNERBIN_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/innerbin");

pub const DOCKER_IMAGE: &str = "jsh-sandbox";
