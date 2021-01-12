use std::convert::TryInto;
use std::error::Error;
use std::io::{Read, Write};
use thiserror::Error;

#[derive(Error, Debug)]
#[error("Invalid message.")]
pub struct InvalidMessage;

pub fn read<R: Read, W: Write, F: FnMut(u16, u16)>(
  mut f: R,
  mut data_into: W,
  mut resize: F,
) -> Result<(), Box<dyn Error>> {
  let mut header = [0u8; 5];
  f.read_exact(&mut header[..])?;
  match header[0] {
    0 => {
      let len = u32::from_be_bytes(header[1..5].try_into().unwrap()) as usize;
      for b in f.bytes().take(len) {
        data_into.write_all(&[b?])?;
      }
    }
    1 => {
      let w = u16::from_be_bytes(header[1..3].try_into().unwrap());
      let h = u16::from_be_bytes(header[3..5].try_into().unwrap());
      resize(w, h);
    }
    _ => {
      return Err(Box::new(InvalidMessage));
    }
  }
  Ok(())
}
