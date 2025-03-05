use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream, UdpSocket},
    time::Instant,
};

use crate::common::BUFFER_SIZE;

/// A "merged" socket type that combines tcp and udp in a one easier to use API
#[derive(Debug)]
pub struct SocketWrapper {
    tcp: Option<TcpStream>,
    udp: Option<UdpSocket>,
    last_udp: Instant,
}
impl SocketWrapper {
    pub fn fill_in_udp(&mut self, udp: UdpSocket) {
        self.udp = Some(udp);
        self.touch();
    }
    pub fn fill_in_tcp(&mut self, tcp: TcpStream) {
        self.tcp = Some(tcp);
        self.touch();
    }

    pub fn from_udp_socket(udp: UdpSocket) -> Self {
        Self {
            tcp: None,
            udp: Some(udp),
            last_udp: Instant::now(),
        }
    }

    pub fn from_tcp_socket(tcp: TcpStream) -> Self {
        Self {
            tcp: Some(tcp),
            udp: None,
            last_udp: Instant::now(),
        }
    }

    pub fn from_tcp_and_udp_sockets(tcp: TcpStream, udp: UdpSocket) -> Self {
        Self {
            tcp: Some(tcp),
            udp: Some(udp),
            last_udp: Instant::now(),
        }
    }

    pub fn is_timed_out(&self) -> bool {
        let mut buffer = [0u8; BUFFER_SIZE];
        let tcp_can_timeout = if self.has_tcp() {
            if let Ok(size) = self.tcp.as_ref().unwrap().peek(&mut buffer) {
                if size == 0 {
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            true
        };

        const UDP_TIMEOUT_MS: u128 = 1400;
        let udp_can_timeout = if self.has_udp() {
            if Instant::now().duration_since(self.last_udp).as_millis() > UDP_TIMEOUT_MS {
                true
            } else {
                false
            }
        } else {
            true
        };

        tcp_can_timeout && udp_can_timeout
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

    pub fn has_udp(&self) -> bool {
        return self.udp.is_some();
    }

    pub fn has_tcp(&self) -> bool {
        return self.tcp.is_some();
    }

    pub fn read_udp(&self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddr)> {
        self.udp.as_ref().unwrap().recv_from(buf)
    }

    pub fn write_udp(&self, buf: &[u8], addr: String) -> std::io::Result<usize> {
        self.udp.as_ref().unwrap().send_to(buf, addr)
    }

    /// Updates udp timeouts
    pub fn touch(&mut self) {
        self.last_udp = Instant::now();
    }
}
