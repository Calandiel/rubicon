use std::net::{TcpListener, UdpSocket};

use crate::{
    server::{Connections, PlayerData},
    socket::SocketWrapper,
};

pub trait ToConnections {
    fn to_connections(&mut self) -> &mut Connections;
}

pub fn accept_connections(
    tcp_listener: &TcpListener,
    udp_listener_address: Option<String>,
    connections: Connections,
) -> ! {
    loop {
        let stream = tcp_listener.accept();
        if let Ok((tcp_stream, peer)) = stream {
            println!("Received connection from: {}", peer);
            tcp_stream.set_nonblocking(true).unwrap(); // TODO: remove this unwrap
            let mut connections = connections.data.lock().unwrap();
            connections.insert(
                peer.port(),
                PlayerData {
                    name: "<missing>".to_string(),
                    address: peer,
                    stream: if let Some(addr) = &udp_listener_address {
                        SocketWrapper::from_tcp_and_udp_sockets(
                            tcp_stream,
                            UdpSocket::bind(addr).unwrap(),
                        )
                    } else {
                        SocketWrapper::from_tcp_socket(tcp_stream)
                    },
                },
            );
        }
    }
}

pub fn print_connections(connections: &Connections) {
    for (_, player_data) in connections.data.lock().unwrap().iter() {
        println!("- {} @ {}", player_data.name, player_data.address);
    }
}

pub fn handle_connections<
    T: ToConnections + Send + 'static,
    F: FnMut(&mut T, &mut [u8]) -> () + Send + 'static,
>(
    mut connections: T,
    mut closure: F,
) {
    std::thread::spawn(move || {
        let mut buffer = [0u8; 1024 * 1024];
        loop {
            closure(&mut connections, &mut buffer);
        }
    });
}
