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
    mut udp_address_and_server_relay: Option<(String, std::sync::mpsc::Sender<(u16, Vec<u8>)>)>,
    relay_packets_receiver: Option<std::sync::mpsc::Receiver<(String, Vec<u8>)>>,
    connections: Connections,
) -> ! {
    // let connections_cloned = connections.clone();
    std::thread::spawn(move || {
        // let connections = connections_cloned;
        if let Some((addr, server_relay)) = &mut udp_address_and_server_relay {
            // println!(
            // "Binding a udp socket on {} while accepting connections",
            // addr
            // );
            let udp = UdpSocket::bind(addr.clone()).unwrap();
            udp.set_nonblocking(true).unwrap();
            let mut buffer = [0u8; 1024 * 8];

            loop {
                if let Ok((size, addr)) = udp.recv_from(&mut buffer) {
                    // println!("Received udp traffic of size {} from {}", size, addr);
                    // Welp, gotta send it next!
                    // But... where to?
                    // This could be a server setup ;-;
                    server_relay
                        .send((addr.port(), buffer[..size].to_vec()))
                        .unwrap();
                }

                if let Ok(received) = relay_packets_receiver.as_ref().unwrap().try_recv() {
                    udp.send_to(&received.1, received.0).unwrap();
                }
            }
        }
    });

    loop {
        let stream = tcp_listener.accept();
        if let Ok((tcp_stream, peer)) = stream {
            // println!("Received connection from: {}", peer);
            tcp_stream.set_nonblocking(true).unwrap(); // TODO: remove this unwrap
            let mut connections = connections.data.lock().unwrap();
            // Here is where we add new connections!
            // We detect them by receiving tcp packets.
            connections.insert(
                peer.port(),
                PlayerData {
                    name: "<missing>".to_string(),
                    address: peer,
                    stream: SocketWrapper::from_tcp_socket(tcp_stream),
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
        let mut buffer = [0u8; 1024 * 8];
        loop {
            closure(&mut connections, &mut buffer);
        }
    });
}
