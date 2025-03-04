use std::net::TcpListener;

use crate::server::{Connections, PlayerData};

pub fn accept_connections(listener: &TcpListener, connections: Connections) -> ! {
    loop {
        let stream = listener.accept();
        if let Ok((tcp_stream, peer)) = stream {
            println!("Received connection from: {}", peer);
            tcp_stream.set_nonblocking(true).unwrap(); // TODO: remove this unwrap
            let mut connections = connections.data.lock().unwrap();
            connections.insert(
                peer.port(),
                PlayerData {
                    name: "<missing>".to_string(),
                    address: peer,
                    stream: tcp_stream,
                },
            );
        }
    }
}

pub fn handle_connections<F: FnMut(&mut Connections, &mut [u8]) -> () + Send + 'static>(
    mut connections: Connections,
    mut closure: F,
) {
    std::thread::spawn(move || {
        let mut buffer = [0u8; 1024 * 1024];
        loop {
            closure(&mut connections, &mut buffer);
        }
    });
}
