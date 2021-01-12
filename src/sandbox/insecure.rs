use std::pin::Pin;
use std::{error::Error, process::Stdio};

use futures::FutureExt;
use std::future::Future;
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use crate::uniqueptr::UniquePtr;
pub struct InsecureSandbox {
  pub wait: Pin<Box<dyn Future<Output = ()> + Send>>,
  pub stdin: ChildStdin,
  pub stdout: ChildStdout,
  _proc: UniquePtr<Child>,
}

impl InsecureSandbox {
  pub async fn new() -> Result<Self, Box<dyn Error + Send + Sync>> {
    let mut proc = Command::new(crate::config::INNERBIN_PATH)
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::null())
      .kill_on_drop(true)
      .spawn()?;
    let stdin = proc.stdin.take().unwrap();
    let stdout = proc.stdout.take().unwrap();
    unsafe {
      let proc = UniquePtr::new(proc);
      let fut = proc.deref_mut().wait().map(|_| ());
      Ok(Self {
        _proc: proc,
        wait: Box::pin(fut),
        stdin,
        stdout,
      })
    }
  }

  pub async fn terminate(&mut self) {
    unsafe {
      std::ptr::drop_in_place(&mut self.wait);
      let proc = self._proc.deref_mut();
      let _ = proc.kill().await;
      std::ptr::write(&mut self.wait, Box::pin(proc.wait().map(|_| ())));
    }
  }
}
