use std::error::Error;
use std::pin::Pin;

use futures::{AsyncRead, AsyncWrite, FutureExt, TryStreamExt};
use shiplift::{tty::TtyChunk, Container, ContainerOptions, Docker};
use std::future::Future;

use crate::uniqueptr::UniquePtr;

pub struct DockerSandbox {
  pub wait: Pin<Box<dyn Future<Output = ()> + Send>>,
  pub stdin: Pin<Box<dyn AsyncWrite + Send>>,
  pub stdout: Pin<Box<dyn AsyncRead + Send>>,
  container_ref: UniquePtr<Container<'static>>,
  container_id: UniquePtr<str>,
  docker: UniquePtr<Docker>,
}

impl DockerSandbox {
  pub async fn new() -> Result<Self, Box<dyn Error + Send + Sync>> {
    let docker = shiplift::Docker::new();
    docker.ping().await?;
    let containers = docker.containers();
    let container = containers
      .create(
        &ContainerOptions::builder(crate::config::DOCKER_IMAGE)
          .attach_stdin(true)
          .attach_stdout(true)
          .attach_stderr(true)
          .auto_remove(true)
          .tty(false)
          .cpus(0.5f64)
          .network_mode("none")
          .memory(1u64 << 26u64)
          .user("10000:10000")
          .build(),
      )
      .await?;
    if let Some(warnings) = container.warnings {
      for warning in warnings.iter() {
        eprintln!("Container creation warning: {}", warning);
      }
    }
    let docker = UniquePtr::new(docker);
    let container_id = UniquePtr::from_box(container.id.clone().into_boxed_str());
    unsafe {
      let container_ref = UniquePtr::new(docker.deref().containers().get(container_id.deref()));
      container_ref.deref().start().await?;
      let attach = container_ref.deref().attach().await?;
      let (stdout, stdin) = attach.split();
      let wait = container_ref.deref().wait();
      Ok(Self {
        docker,
        container_id,
        container_ref,
        stdin: Box::pin(stdin),
        stdout: Box::pin(
          stdout
            .map_ok(|x| match x {
              TtyChunk::StdOut(data) => data,
              _ => Vec::<u8>::new(),
            })
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            .into_async_read(),
        ),
        wait: Box::pin(wait.map(|_| ())),
      })
    }
  }

  pub async fn terminate(&mut self) {
    let _ = unsafe { self.container_ref.deref() }.kill(None).await;
  }
}
