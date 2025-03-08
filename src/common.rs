use std::{
    net::{TcpListener, UdpSocket},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::{
    connections::{Connections, PlayerData},
    packet::Packet,
    socket::SocketWrapper,
};

pub const DISABLE_NAGLE_ALGORITHM: bool = true;
pub const MINIMUM_TICK_RATE_IN_MS: u128 = 1;
pub const BUFFER_SIZE: usize = 1024 * 64;
pub const MAX_QUEUE_SIZE: u64 = 1024;
pub const HEARTBEATS_PER_SECOND: f64 = 4.;

pub trait ToConnections {
    fn to_connections(&mut self) -> &mut Connections;
}

pub fn handle_udp_traffic(
    mut udp_address_and_server_relay: Option<(String, std::sync::mpsc::Sender<(u16, Vec<u8>)>)>,
    relay_packets_receiver: Option<std::sync::mpsc::Receiver<(String, Vec<u8>)>>,
    udp_queue_size: Option<Arc<Mutex<u64>>>,
    relay_queue_size: Option<Arc<Mutex<u64>>>,
    server_address: String,
) {
    std::thread::spawn(move || {
        if let Err(e) = thread_priority::set_current_thread_priority(
            thread_priority::ThreadPriority::Crossplatform(5.try_into().unwrap()),
        ) {
            panic!("{:?}", e);
        }

        // let connections = connections_cloned;
        if let Some((addr, server_relay)) = &mut udp_address_and_server_relay {
            println!(
                "Binding a udp socket on {} while accepting connections",
                addr
            );
            let udp = UdpSocket::bind(addr.clone()).unwrap();
            udp.set_nonblocking(true).unwrap();
            let mut buffer = [0u8; BUFFER_SIZE];

            // We must send a packet to the server. This is VERY important as it'll let us avoid issues with the NAT.

            let udp_queue_size = udp_queue_size.unwrap().clone();
            let relay_queue_size = relay_queue_size.unwrap().clone();

            // Send the initial heartbeat. Important for communication!
            udp.send_to(
                &bincode::serialize(&Packet::Heartbeat).unwrap(),
                &server_address,
            )
            .unwrap();

            let mut last_heartbeat = Instant::now();
            loop {
                let begin = Instant::now();
                let mut had_one = false;

                // Keep the connection going and send the heartbeat again
                if last_heartbeat.elapsed().as_secs_f64() > 1. / HEARTBEATS_PER_SECOND {
                    last_heartbeat = Instant::now();
                    udp.send_to(
                        &bincode::serialize(&Packet::Heartbeat).unwrap(),
                        &server_address,
                    )
                    .unwrap();
                }
                {
                    // Loop until we empty the queue
                    // loop {
                    if let Ok((size, addr)) = udp.recv_from(&mut buffer) {
                        had_one = true;

                        if *udp_queue_size.lock().unwrap() < MAX_QUEUE_SIZE {
                            // println!("Received udp traffic of size {} from {}", size, addr);
                            // Welp, gotta send it next!
                            // But... where to?
                            // This could be a server setup ;-;
                            // println!("RECEIVED UDP :: {} @ {}", addr, size);
                            server_relay
                                .send((addr.port(), buffer[..size].to_vec()))
                                .unwrap();
                            *udp_queue_size.lock().unwrap() += 1;

                            // println!("RECEIVED UDP PACKETS: {}", received_packets);
                        } else {
                            println!(
                                "QUEUE OVER CAPACITY {}! DROPPING A UDP PACKET...",
                                *udp_queue_size.lock().unwrap()
                            );
                        }
                    } else {
                        // break;
                    }
                    // }
                    // Loop until we empty the queue
                    // loop {
                    if let Ok((addr, data)) = relay_packets_receiver.as_ref().unwrap().try_recv() {
                        *relay_queue_size.lock().unwrap() -= 1;

                        had_one = true;
                        // println!("RELAYING UDP DATA TO :: -> {} @ {}", addr, data.len());
                        udp.send_to(&data, addr).unwrap();
                        // println!("SENT UDP PACKETS: {}", sent_packets);
                    } else {
                        // break;
                    }
                    // }
                }

                let remaining_millis =
                    MINIMUM_TICK_RATE_IN_MS as f64 - begin.elapsed().as_millis() as f64;
                if !had_one && remaining_millis > 0. {
                    std::thread::sleep(Duration::from_millis(remaining_millis as u64));
                }
            }
        }
    });
}

pub fn accept_connections(tcp_listener: &TcpListener, connections: Connections) -> ! {
    loop {
        let begin = Instant::now();

        let mut had_one = false;
        {
            let stream = tcp_listener.accept();
            if let Ok((tcp_stream, peer)) = stream {
                had_one = true;
                // println!("Received connection from: {}", peer);
                tcp_stream.set_nonblocking(true).unwrap(); // TODO: remove this unwrap
                tcp_stream.set_nodelay(DISABLE_NAGLE_ALGORITHM).unwrap();
                let mut connections = connections.data.lock().unwrap();
                // Here is where we add new connections!
                // We detect them by receiving tcp packets.
                connections.insert(
                    peer.port(),
                    PlayerData {
                        name: "<missing>".to_string(),
                        address: peer,
                        stream: SocketWrapper::from_tcp_socket(tcp_stream),
                        local_port: None,
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
        if let Err(e) = thread_priority::set_current_thread_priority(
            thread_priority::ThreadPriority::Crossplatform(5.try_into().unwrap()),
        ) {
            panic!("{:?}", e);
        }

        let mut buffer = [0u8; BUFFER_SIZE];
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
