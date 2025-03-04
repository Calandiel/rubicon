use std::{
    io::Read,
    net::{TcpStream, UdpSocket},
};

/// A "merged" socket type that combines tcp and udp in a one easier to use API
#[derive(Debug)]
pub struct SocketWrapper {
    tcp: Option<TcpStream>,
    udp: Option<UdpSocket>,
}
impl SocketWrapper {
    pub fn connect(tcp_address: String, udp_address: String) -> Self {
        Self {
            tcp: Some(TcpStream::connect(tcp_address).unwrap()),
            udp: Some(UdpSocket::bind(udp_address).unwrap()),
        }
    }

    pub fn from_tcp_socket(tcp: TcpStream) -> Self {
        Self {
            tcp: Some(tcp),
            udp: None,
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
}
