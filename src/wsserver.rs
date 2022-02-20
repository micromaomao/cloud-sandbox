use futures::sink::SinkExt;
use futures::stream::StreamExt;

use std::time::Duration;
use std::{error::Error, net::SocketAddr};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, WebSocketStream};
use tungstenite::Message;

mod config;
mod sandbox;
mod uniqueptr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let listener = TcpListener::bind("0.0.0.0:5000").await?;
  loop {
    let (stream, src) = listener.accept().await?;
    tokio::spawn(async move {
      handle_ws_connection(stream, src).await;
    });
  }
}

#[derive(Error, Debug)]
#[error("Expected binary ws data, got text.")]
struct ExpectedBinaryData;

async fn handle_ws_connection<S: AsyncRead + AsyncWrite + Unpin>(mut stream: S, src: SocketAddr) {
  let mut ws = match accept_async(stream).await {
    Ok(ws) => ws,
    Err(e) => {
      eprintln!("{}: {}", src, e);
      return;
    }
  };
  async fn inner<S: AsyncRead + AsyncWrite + Unpin>(
    ws: &mut WebSocketStream<S>,
    _src: SocketAddr,
  ) -> Result<(), Box<dyn Error + Send + Sync>> {
    ws.feed(Message::Text(
      "Starting container, please wait...\n\r".to_owned(),
    ))
    .await?;
    type Sandbox = sandbox::docker::DockerSandbox;
    // type Sandbox = sandbox::insecure::InsecureSandbox;
    let mut run = Sandbox::new().await?;
    let mut read_buf = vec![0u8; 2048];
    async fn proc_loop<S: AsyncRead + AsyncWrite + Unpin>(
      run: &mut Sandbox,
      ws: &mut WebSocketStream<S>,
      read_buf: &mut Vec<u8>,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
      let timeout = tokio::time::sleep(Duration::from_secs(600));
      tokio::select! {
        read_res = futures::AsyncReadExt::read(&mut run.stdout, &mut read_buf[..]) => {
          let read_size = read_res?;
          ws.feed(Message::Text(String::from_utf8_lossy(&read_buf[0..read_size]).into_owned())).await?;
        },
        wsmsg = ws.next() => {
          if wsmsg.is_none() {
            return Ok(false);
          }
          let wsmsg = wsmsg.unwrap()?;
          match wsmsg {
            Message::Binary(stuff) => futures::AsyncWriteExt::write_all(&mut run.stdin, &stuff).await?,
            Message::Text(stuff) => {
              let mut stuff = stuff.into_bytes();
              let len = stuff.len();
              stuff.push(0u8);
              stuff.extend_from_slice(&u32::to_be_bytes(len as u32));
              stuff.rotate_right(5);
              futures::AsyncWriteExt::write_all(&mut run.stdin, &stuff).await?;
            },
            Message::Close(_) => {
              return Ok(false);
            },
            Message::Ping(payload) => {
              ws.feed(Message::Pong(payload)).await?;
            },
            Message::Pong(_) => {}
          }
        },
        _ = run.wait.as_mut() => {
          let _ = ws.feed(Message::Text("\n\rContainer exited.\n".to_owned())).await;
          return Ok(false);
        }
        _ = timeout => {
          let _ = ws.feed(Message::Text("\n\rSession terminated due to timeout.\n".to_owned())).await;
          return Ok(false);
        }
      }
      Ok(true)
    }
    loop {
      match proc_loop(&mut run, ws, &mut read_buf).await {
        Ok(true) => {}
        Ok(false) => break,
        Err(e) => {
          let _ = ws
            .feed(Message::Text(format!("Error: {}\n\rWill now exit.\n\r", e)))
            .await;
          run.terminate().await;
          return Err(e);
        }
      }
    }
    run.terminate().await;
    Ok(())
  }
  if let Err(e) = inner(&mut ws, src).await {
    eprintln!("{}: {}", src, e);
    let _ = ws.feed(Message::Text(format!("\n\rError: {}\n", e))).await;
  }
}
