pub mod client;
pub mod server;
mod tests;

use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io::{Read, Write};
use std::mem::size_of;
use std::net::TcpStream;



pub struct Transport<T: Read + Write> {
    c: T,
}

impl<T> Transport<T>
    where
        T: Read + Write,
{
    pub fn new(c: T) -> Self {
        Self { c }
    }

    pub fn send<V>(&mut self, value: &V) -> Result<()>
        where
            V: Serialize,
    {
        self.c.write_all(&bincode::serialize(value)?)?;
        Ok(())
    }

    pub fn send_with_varsize<V>(&mut self, value: &V) -> Result<()>
        where
            V: Serialize,
    {
        let data = bincode::serialize(value)?;
        let len = bincode::serialize(&data.len())?;
        self.c.write_all(&len)?;
        self.c.write_all(&data)?;
        Ok(())
    }

    pub fn receive_varsize<R: DeserializeOwned>(&mut self) -> Result<R> {
        let msg_size: usize = self.receive(size_of::<usize>())?;
        self.receive::<R>(msg_size)
    }

    pub fn receive<R: DeserializeOwned>(&mut self, size: usize) -> Result<R> {
        let mut buf: Vec<u8> = vec![0; size];
        self.c.read_exact(&mut buf)?;
        let result: R = bincode::deserialize(&buf)?;
        Ok(result)
    }
}

enum ClientState {
    Initial,
    PuzzleSent,
}

struct Connection {
    stream: TcpStream,
    state: ClientState,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            state: ClientState::Initial,
        }
    }
}

