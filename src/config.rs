use std::process::Command;

pub fn get_command() -> Command {
  Command::new("/usr/bin/bash")
}

pub const INNERBIN_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/innerbin");
