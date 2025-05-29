use std::{
    io::{Read, Write},
    net::TcpStream,
};

use crate::common::BUFFER_SIZE;

/// A "merged" socket type that combines tcp and udp in a one easier to use API.
#[derive(Debug)]
pub struct SocketWrapper {
    tcp: Option<TcpStream>,
}
impl SocketWrapper {
    pub fn from_tcp_socket(tcp: TcpStream) -> Self {
        Self { tcp: Some(tcp) }
    }

    pub fn is_timed_out(&self) -> bool {
        let mut buffer = [0u8; BUFFER_SIZE];
        if self.has_tcp() {
            if let Ok(size) = self.tcp.as_ref().unwrap().peek(&mut buffer) {
                size == 0
            } else {
                false
            }
        } else {
            true
        }
    }

    /// Peeks into the tcp stream
    pub fn peek(&self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.tcp.as_ref().unwrap().peek(buf)
    }

    /// Reads from the tcp stream
    pub fn read(&self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.tcp.as_ref().unwrap().read(buf)
    }

    pub fn get_tcp_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        self.tcp.as_ref().unwrap().peer_addr()
    }

    /// Writes the tcp stream
    pub fn write(&self, buf: &[u8]) -> std::io::Result<usize> {
        self.tcp.as_ref().unwrap().write(buf)
    }

    pub fn has_tcp(&self) -> bool {
        return self.tcp.is_some();
    }
}
