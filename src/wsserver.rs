use futures::sink::SinkExt;
use futures::stream::StreamExt;

use std::{error::Error, net::SocketAddr};
use tokio::net::TcpListener;
use tokio::{
  io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
};
use tokio_tungstenite::{accept_async, WebSocketStream};
use tungstenite::Message;
use thiserror::Error;

mod config;
mod sandbox;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let listener = TcpListener::bind("127.0.0.1:5000").await?;
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

async fn handle_ws_connection<S: AsyncRead + AsyncWrite + Unpin>(stream: S, src: SocketAddr) {
  async fn inner<S: AsyncRead + AsyncWrite + Unpin>(
    stream: S,
    _src: SocketAddr,
  ) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut ws = accept_async(stream).await?;
    ws.feed(Message::Text(
      "Starting container, please wait...\n".to_owned(),
    ))
    .await?;
    type Sandbox = sandbox::insecure::InsecureSandbox;
    let mut run = Sandbox::new()?;
    let mut read_buf = vec![0u8; 2048];
    async fn proc_loop<S: AsyncRead + AsyncWrite + Unpin>(
      run: &mut Sandbox,
      ws: &mut WebSocketStream<S>,
      read_buf: &mut Vec<u8>,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
      tokio::select! {
        read_res = run.stdout.read(&mut read_buf[..]) => {
          let read_size = read_res?;
          ws.feed(Message::Text(String::from_utf8_lossy(&read_buf[0..read_size]).into_owned())).await?;
        },
        wsmsg = ws.next() => {
          if wsmsg.is_none() {
            return Ok(false);
          }
          let wsmsg = wsmsg.unwrap()?;
          match wsmsg {
            Message::Binary(stuff) => run.stdin.write_all(&stuff).await?,
            Message::Close(_) => {
              return Ok(false);
            },
            Message::Ping(payload) => {
              ws.feed(Message::Pong(payload)).await?;
            },
            Message::Pong(_) => {},
            Message::Text(_) => {
              return Err(Box::new(ExpectedBinaryData));
            }
          }
        },
        _ = run.proc.wait() => {
          return Ok(false);
        }
      }
      Ok(true)
    }
    loop {
      match proc_loop(&mut run, &mut ws, &mut read_buf).await {
        Ok(true) => {}
        Ok(false) => break,
        Err(e) => {
          let _ = ws
            .feed(Message::Text(format!("Error: {}\nWill now exit.\n", e)))
            .await;
          run.terminate().await;
          return Err(e);
        }
      }
    }
    run.terminate().await;
    Ok(())
  }
  if let Err(e) = inner(stream, src).await {
    eprintln!("{}: {}", src, e);
  }
}
