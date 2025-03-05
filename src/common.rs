use std::{
    net::{TcpListener, UdpSocket},
    time::{Duration, Instant},
};

use crate::{
    server::{Connections, PlayerData},
    socket::SocketWrapper,
};

pub const MINIMUM_TICK_RATE_IN_MS: u128 = 5;

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
            let mut buffer = [0u8; 1024 * 64];

            loop {
                let begin = Instant::now();
                let mut had_one = false;

                {
                    if let Ok((size, addr)) = udp.recv_from(&mut buffer) {
                        had_one = true;
                        // println!("Received udp traffic of size {} from {}", size, addr);
                        // Welp, gotta send it next!
                        // But... where to?
                        // This could be a server setup ;-;
                        println!("RECEIVED LOCAL UDP :: {} @ {}", addr, size);
                        server_relay
                            .send((addr.port(), buffer[..size].to_vec()))
                            .unwrap();
                    }

                    if let Ok((addr, data)) = relay_packets_receiver.as_ref().unwrap().try_recv() {
                        had_one = true;
                        println!("RELAYING DATA TO :: -> {} @ {}", addr, data.len());
                        udp.send_to(&data, addr).unwrap();
                    }
                }

                let remaining_millis =
                    MINIMUM_TICK_RATE_IN_MS as f64 - begin.elapsed().as_millis() as f64;
                if !had_one && remaining_millis > 0. {
                    std::thread::sleep(Duration::from_millis(remaining_millis as u64));
                }
            }
        }
    });

    loop {
        let begin = Instant::now();

        let mut had_one = false;
        {
            let stream = tcp_listener.accept();
            if let Ok((tcp_stream, peer)) = stream {
                had_one = true;
                // println!("Received connection from: {}", peer);
                tcp_stream.set_nonblocking(true).unwrap(); // TODO: remove this unwrap
                tcp_stream.set_nodelay(true).unwrap();
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

        let remaining_millis = MINIMUM_TICK_RATE_IN_MS as f64 - begin.elapsed().as_millis() as f64;
        if !had_one && remaining_millis > 0. {
            std::thread::sleep(Duration::from_millis(remaining_millis as u64));
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
    F: FnMut(&mut T, &mut [u8], &mut bool) -> () + Send + 'static,
>(
    mut connections: T,
    mut closure: F,
) {
    std::thread::spawn(move || {
        let mut buffer = [0u8; 1024 * 64];
        loop {
            let begin = Instant::now();

            let mut had_one = false;
            {
                closure(&mut connections, &mut buffer, &mut had_one);
            }

            let remaining_millis =
                MINIMUM_TICK_RATE_IN_MS as f64 - begin.elapsed().as_millis() as f64;
            if !had_one && remaining_millis > 0. {
                std::thread::sleep(Duration::from_millis(remaining_millis as u64));
            }
        }
    });
}
