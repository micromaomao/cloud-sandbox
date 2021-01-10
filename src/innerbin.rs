
use std::{error::Error, process::Stdio};
use std::{io, ptr};

mod config;
mod parent;
mod inputproto;

fn main() -> Result<(), Box<dyn Error>> {
  let mut pty_master: libc::c_int = 0;
  let fork_ret = unsafe {
    libc::forkpty(
      &mut pty_master,
      ptr::null_mut(),
      ptr::null_mut(),
      ptr::null_mut(),
    )
  };
  if fork_ret == -1 {
    Err(Box::new(io::Error::last_os_error()))
  } else if fork_ret == 0 {
    child()
  } else {
    let pid_child = fork_ret;
    parent::parent(pid_child, pty_master)
  }
}

fn child() -> ! {
  unsafe {
    libc::setsid();
    libc::ioctl(0, libc::TIOCSCTTY, 0_i32);
  }

  use std::os::unix::process::CommandExt;
  let mut cmd = config::get_command();
  cmd
    .stdin(Stdio::inherit())
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit());
  let err = cmd.exec();
  eprintln!("Error execing command: {}", err);
  std::process::exit(1)
}
