use io::BufWriter;

use std::io::Write;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd};
use std::{
  error::Error,
  fs::File,
  mem::{ManuallyDrop, MaybeUninit},
};
use std::{io, ptr};

use crate::inputproto;

macro_rules! ret_err {
  () => {
    return Err(Box::new(std::io::Error::last_os_error()))
  };
}

pub fn parent(pid_child: libc::c_int, pty_fd: libc::c_int) -> Result<(), Box<dyn Error>> {
  struct KillOnDrop(libc::c_int);
  impl Drop for KillOnDrop {
    fn drop(&mut self) {
      unsafe {
        libc::kill(self.0, libc::SIGKILL);
        libc::waitpid(self.0, ptr::null_mut(), 0);
      }
    }
  }
  let _child_proc = KillOnDrop(pid_child);
  struct CloseOnDrop(libc::c_int);
  impl Drop for CloseOnDrop {
    fn drop(&mut self) {
      unsafe { libc::close(self.0) };
    }
  }
  let _pty = CloseOnDrop(pty_fd);

  fn make_signal_mask() -> libc::sigset_t {
    let mut sigs: MaybeUninit<libc::sigset_t> = MaybeUninit::uninit();
    unsafe {
      libc::sigemptyset(sigs.as_mut_ptr());
      libc::sigaddset(sigs.as_mut_ptr(), libc::SIGCHLD);
      libc::sigaddset(sigs.as_mut_ptr(), libc::SIGTERM);
      libc::sigaddset(sigs.as_mut_ptr(), libc::SIGINT);
      sigs.assume_init()
    }
  }

  let sigs = make_signal_mask();
  if unsafe { libc::sigprocmask(libc::SIG_BLOCK, &sigs, ptr::null_mut()) } == -1 {
    ret_err!();
  }
  let signalfd = unsafe { libc::signalfd(-1, &sigs, 0) };
  if signalfd == -1 {
    ret_err!();
  }
  let _signalfd = CloseOnDrop(signalfd);

  fn epevt_read(data: u64) -> libc::epoll_event {
    libc::epoll_event {
      events: libc::EPOLLIN as _,
      u64: data,
    }
  }

  let _rust_stdin = io::stdin();
  let mut locked_rust_stdin = _rust_stdin.lock();
  let _rust_stdout = io::stdout();
  let mut locked_rust_stdout = _rust_stdout.lock();
  let stdin_fd = locked_rust_stdin.as_raw_fd();
  let stdout_fd = locked_rust_stdout.as_raw_fd();

  unsafe {
    let epfd = libc::epoll_create(10);
    if epfd == -1 {
      ret_err!();
    }
    let _epfd = CloseOnDrop(epfd);
    if libc::epoll_ctl(
      epfd,
      libc::EPOLL_CTL_ADD,
      signalfd,
      &mut epevt_read(signalfd as _),
    ) == -1
    {
      ret_err!();
    }
    if libc::epoll_ctl(
      epfd,
      libc::EPOLL_CTL_ADD,
      stdin_fd,
      &mut epevt_read(stdin_fd as _),
    ) == -1
    {
      ret_err!();
    }
    if libc::epoll_ctl(
      epfd,
      libc::EPOLL_CTL_ADD,
      pty_fd,
      &mut epevt_read(pty_fd as _),
    ) == -1
    {
      ret_err!();
    }
    let mut event_buf: Vec<libc::epoll_event> = Vec::with_capacity(20);
    let mut transfer_buf: Vec<u8> = Vec::with_capacity(1024000);
    loop {
      let epres = libc::epoll_wait(epfd, event_buf.as_mut_ptr(), event_buf.capacity() as _, -1);
      if epres == -1 {
        ret_err!();
      }
      event_buf.set_len(epres as _);
      for event in event_buf.iter() {
        let fd: libc::c_int = event.u64 as _;
        if fd == signalfd {
          return Ok(());
        } else if fd == stdin_fd {
          let pty_f = ManuallyDrop::new(File::from_raw_fd(pty_fd));
          let mut pty_bufwriter = BufWriter::new(&*pty_f);
          inputproto::read(&mut locked_rust_stdin, &mut pty_bufwriter, |w, h| {
            let winsz = libc::winsize {
              ws_row: h,
              ws_col: w,
              ws_xpixel: 0,
              ws_ypixel: 0,
            };
            libc::ioctl(pty_fd, libc::TIOCSWINSZ, &winsz as *const libc::winsize);
          })?;
          drop(pty_bufwriter);
          ManuallyDrop::into_inner(pty_f).into_raw_fd();
        } else if fd == pty_fd {
          if libc::splice(
            pty_fd,
            ptr::null_mut(),
            stdout_fd,
            ptr::null_mut(),
            1024000,
            0,
          ) == -1
          {
            let read_size = libc::read(
              pty_fd,
              transfer_buf.as_mut_ptr() as *mut _,
              transfer_buf.capacity(),
            );
            if read_size == -1 {
              ret_err!();
            }
            transfer_buf.set_len(read_size as usize);
            locked_rust_stdout.write_all(&transfer_buf)?;
            transfer_buf.truncate(0);
            locked_rust_stdout.flush()?;
          }
        } else {
          unreachable!()
        }
      }
      event_buf.truncate(0);
    }
  }
}
