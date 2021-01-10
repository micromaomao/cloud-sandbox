use std::{error::Error, process::Stdio};

use tokio::process::{Child, ChildStdin, ChildStdout, Command};

pub struct InsecureSandbox {
  pub proc: Child,
  pub stdin: ChildStdin,
  pub stdout: ChildStdout,
}

impl InsecureSandbox {
  pub fn new() -> Result<Self, Box<dyn Error + Send + Sync>> {
    let mut proc = Command::new(crate::config::INNERBIN_PATH)
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::null())
      .kill_on_drop(true)
      .spawn()?;
    let stdin = proc.stdin.take().unwrap();
    let stdout = proc.stdout.take().unwrap();
    Ok(Self {
      proc,
      stdin,
      stdout,
    })
  }

  pub async fn terminate(&mut self) {
    let _ = self.proc.kill().await;
  }
}
