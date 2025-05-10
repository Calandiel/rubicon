pub mod client;
pub mod commands;
pub mod common;
pub mod connections;
pub mod packet;
pub mod server;
pub mod socket;

use std::{
    io::{ErrorKind, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream, UdpSocket},
    sync::{mpsc::channel, Arc, Mutex},
    time::{Duration, Instant},
    u8,
};

use clap::Parser;
use client::ClientState;
use commands::{Args, Commands, SocketType};
use common::{
    accept_connections, handle_connections, handle_udp_traffic, BUFFER_SIZE,
    DISABLE_NAGLE_ALGORITHM, MAX_QUEUE_SIZE, MINIMUM_TICK_RATE_IN_MS,
};
use connections::{Connections, PlayerData};
use packet::{
    print_packet, process_packets, CommandPacket, ConnectionPacket, DataPacket, DataPacketLike,
    GreetingPacket, Packet,
};
use server::ServerState;

fn main() {
    let args = Args::parse();

    if let Err(e) = thread_priority::set_current_thread_priority(
        thread_priority::ThreadPriority::Crossplatform(5.try_into().unwrap()),
    ) {
        panic!("{:?}", e);
    }

    // Dispatch from cli
    match args.command {
        Commands::Host { port } => host(port),
        Commands::Connect {
            player_port: port,
            server_address,
            player_name,
            other_player_name,
            other_player_port,
        } => connect(
            port,
            server_address,
            player_name,
            other_player_name,
            other_player_port,
        ),
        Commands::Ping {
            port,
            address,
            socket,
            data_size,
        } => ping(port, address, socket, data_size),
        Commands::Listen { port, socket } => listen(port, socket),
        Commands::Command { address, command } => send_command(address, command),
        Commands::MultiConnect {
            server_address,
            player_name,
            other_player_name,
            player_ports,
        } => multi_connect(server_address, other_player_name, player_name, player_ports),
        Commands::MassConnect {
            server_address,
            player_name,
            other_player_name,
            lower_port_inclusive,
            upper_port_inclusive,
        } => mass_connect(
            server_address,
            other_player_name,
            player_name,
            lower_port_inclusive,
            upper_port_inclusive,
        ),
    }
}

fn host(port: u16) {
    println!("Hosting {}", port);
    // Listener uwu

    let server_state = ServerState::new();
    let connections = server_state.connections.clone();

    // process existing connections - we need to read the data from them and then pass it to the intended receiver

    let _cached = Default::default();
    handle_connections(server_state, move |server_state, buffer, _had_one| {
        // let peers: HashSet<u16> = server_state
        // .connections
        // .data
        // .lock()
        // .unwrap()
        // .iter()
        // .map(|(k, _)| *k)
        // .collect();
        let mut packets = vec![]; // Packets to pass
        let mut connection_packets = vec![]; // Packets to pass
        let mut disconnected = vec![]; // Disconnected peers
        let mut commands = vec![];
        let mut greetings = vec![];
        let mut rejected_packets = vec![]; // Ignored by servers

        let mut connections = server_state.connections.clone();
        process_packets(
            &mut connections,
            // &peers,
            &mut packets,
            &mut connection_packets,
            &mut disconnected,
            &mut commands,
            &mut greetings,
            buffer,
            &mut rejected_packets,
            false,
            Default::default(),
            Default::default(),
            &_cached,
        );
        relay_packets(server_state, &mut packets, &mut connection_packets);
        process_disconnection(&mut connections, &mut disconnected);

        // Process received data
        server_state.receive_greetings(greetings);
        // server_state.receive_data_packets(packets);
        server_state.receive_commands(commands);
    });

    let mut received_packets_counter = 0;
    let udp_socket = UdpSocket::bind(format!("0.0.0.0:{}", port)).unwrap();
    // udp_socket.set_nonblocking(true).unwrap();
    {
        let connections = connections.clone();
        std::thread::spawn(move || {
            if let Err(e) = thread_priority::set_current_thread_priority(
                thread_priority::ThreadPriority::Crossplatform(5.try_into().unwrap()),
            ) {
                panic!("{:?}", e);
            }

            // Receive UDP packets to relay them to clients.
            let mut buffer = [0u8; BUFFER_SIZE];
            // let mut recv_count = 0;
            loop {
                // println!("POG");
                let begin = Instant::now();
                let mut had_one = false;

                if let Ok((size, addr)) = udp_socket.recv_from(&mut buffer) {
                    had_one = true;
                    received_packets_counter += 1;
                    // recv_count += 1;
                    // println!("RECV COUNT: {recv_count}");

                    // println!("UDP: {}", addr);

                    // We need to parse the data to check if its a data packet
                    if let Ok(packet) = bincode::deserialize::<Packet>(&buffer[..size]) {
                        match packet {
                            Packet::Data(data_packet) => {
                                data_packet.print(format!("host side udp (received: {}): ", received_packets_counter).as_str());
                                // print_connections(&connections);
                                let connections = connections.data.lock().unwrap();
                                if let Some(receiver_tcp_port) = connections.get_player_tcp_port_by_name(&data_packet.receiver_name){
                                    if let Some(player_data) = connections.get(&receiver_tcp_port) {
                                        let player_local_port = player_data.last_known_udp_port;
                                        let mut final_address = player_data.address.clone();
                                        final_address.set_port(player_local_port);

                                    	// println!(
                                    	    // "RECEIVED UDP PACKET: {} @ {} FOR {}",
                                    	    // addr, size, final_address
                                        // );

										println!("final_address for udp: {}", final_address);
                                        let _ = udp_socket.send_to(&buffer[..size], final_address);

                                    	// TODO: CONTINUE HERE - we have the final adress - now we need to deliver it as a data packet with udp, then read it on the other side and relay it to the correct destination uwu
										// TODO: well, do we? isnt this basically finished? :thinkge:
                                    } else  {
										println!("Connection for the requested port was not found!");
									}
                                } else {
									println!("Player with a requested name ({}) was no found!", data_packet.receiver_name);
								}
                            },
							Packet::Heartbeat(s) => {
								if let Some(v) = connections.data.lock().unwrap().get_player_udp_port_by_name_mut(&s) {
									// println!("Reacting to a heartbeat request from player {s} on: {}", addr);
									// Ping back with a heartbeat packet!
									*v = addr.port();
									let _ = udp_socket.send_to(&bincode::serialize(&Packet::Heartbeat("".to_string())).unwrap(), addr);
								} else {
									println!("Received a heartbeat but failed to retrieve the player {s} on: {}. This could be due to it being the very first heartbeat.", addr);
								}
							}
                            _ => println!("Received a non data udp packet on the server from {addr}. This shouldn't happen, we only accept data on the udp socket!")
                        }
                    } else {
                        println!(
                            "Received unstructured udp data from {addr} on the server. Weird!"
                        );
                    }
                }

                let remaining_millis =
                    MINIMUM_TICK_RATE_IN_MS as f64 - begin.elapsed().as_millis() as f64;
                if !had_one && remaining_millis > 0. {
                    // println!("SLEEPING ON THE SERVER");
                    std::thread::sleep(Duration::from_millis(remaining_millis as u64));
                }
            }
        });
    }

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();
    accept_connections(&listener, connections, None);
}

fn relay_tcp_data(player_data: &mut PlayerData, packet: Packet) {
    // We need to construct a new packet!
    if let Err(e) = player_data
        .stream
        .write(&bincode::serialize(&packet).unwrap())
    {
        match e.kind() {
            std::io::ErrorKind::WouldBlock => {
                // println!(
                // "A stream ({:?}) would block upon writing: {:?}",
                // player_data.address.clone(),
                // e
                // )
            }
            _ => println!(
                "A stream ({:?}) returned an error upon writing: {:?}",
                player_data.address.clone(),
                e
            ),
        }
    } else {
        // No error!
        println!("Packet delivered from port to player: {}", player_data.name);
    }
}
fn relay_packets(
    server: &mut ServerState,
    packets: &mut Vec<(u16, DataPacket)>,
    connection_packets: &mut Vec<(u16, ConnectionPacket)>,
) {
    let mut locked_connections = server.connections.data.lock().unwrap();
    for (_port, packet) in packets {
        // We need to find the player to retrieve the data from.
        let receiver_name = packet.receiver_name.clone();
        // println!("Relaying a packet to {}", receiver_name);
        if let Some((_, player_data)) = locked_connections
            .iter_mut()
            .find(|element| element.1.name == receiver_name)
        {
            relay_tcp_data(player_data, Packet::Data(packet.clone()));
        } else {
            // println!("Packet delivery to player {} attempted by player at port {} but the target player was not found!", receiver_name, port);
        }
    }
    for (_port, connection_packet) in connection_packets {
        // We need to find the player to retrieve the data from.
        let receiver_name = connection_packet.receiver_name.clone();
        // println!("Relaying a packet to {}", receiver_name);
        if let Some((_, player_data)) = locked_connections
            .iter_mut()
            .find(|element| element.1.name == receiver_name)
        {
            relay_tcp_data(player_data, Packet::Connection(connection_packet.clone()));
        } else {
            // println!("Packet delivery to player {} attempted by player at port {} but the target player was not found!", receiver_name, port);
        }
    }
}

fn process_disconnection(connections: &mut Connections, disconnected: &mut Vec<u16>) {
    let mut locked_connections = connections.data.lock().unwrap();
    for disconnect in disconnected {
        println!("Disconnecting: {}", disconnect);
        locked_connections.remove(disconnect);
    }
}

fn mass_connect(
    relay_server_address: String,
    other_player_name: String,
    player_name: String,
    lower_port: u16,
    upper_port: u16,
) {
    for port in lower_port..upper_port + 1 {
        let relay_server_address = relay_server_address.clone();
        let other_player_name = other_player_name.clone();
        let player_name = player_name.clone();
        std::thread::spawn(move || {
            connect(
                port,
                relay_server_address,
                format!("{player_name}_{port}"),
                other_player_name,
                port,
            );
        });
        std::thread::sleep(Duration::from_millis(500));
    }

    println!("Multi connection established!");
    loop {
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn multi_connect(
    relay_server_address: String,
    other_player_name: String,
    player_name: String,
    player_client_port: Vec<u16>,
) {
    for port in player_client_port {
        let relay_server_address = relay_server_address.clone();
        let other_player_name = other_player_name.clone();
        let player_name = player_name.clone();
        std::thread::spawn(move || {
            connect(
                port,
                relay_server_address,
                format!("{player_name}_{port}"),
                other_player_name,
                port,
            );
        });
        std::thread::sleep(Duration::from_millis(500));
    }

    println!("Multi connection established!");
    loop {
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn connect(
    player_client_port: u16,
    relay_server_address: String,
    player_name: String,
    other_player_name: String,
    other_player_port: u16,
) {
    println!("Connecting on {}", player_client_port);
    // The stream that talks to the server
    let mut local_outgoing_stream = TcpStream::connect(relay_server_address.clone()).unwrap();
    local_outgoing_stream
        .set_nodelay(DISABLE_NAGLE_ALGORITHM)
        .unwrap();
    // ALWAYS begin by sending our name!
    local_outgoing_stream
        .write(
            &bincode::serialize(&Packet::Greeting(GreetingPacket {
                player_name: player_name.clone(),
                local_port: player_client_port,
            }))
            .unwrap()[..],
        )
        .unwrap();
    local_outgoing_stream.set_nonblocking(true).unwrap(); // set non blocking AFTER we send the packet

    let udp_packet_queue_size = Arc::new(Mutex::new(0u64));
    let (udp_packet_sender, udp_packet_receiver) = channel::<(u16, Vec<u8>)>();
    let relay_queue_size = Arc::new(Mutex::new(0u64));
    let (relay_packet_sender, relay_packet_receiver) = channel::<(String, Vec<u8>)>();
    let udp_packet_queue_size_cloned = udp_packet_queue_size.clone();
    let relay_queue_size_cloned = relay_queue_size.clone();

    // Incomming stream
    let client: ClientState = ClientState::new(
        player_name.clone(),
        player_client_port,
        other_player_name.clone(),
        other_player_port,
    );
    let connections = client.connections.clone();

    let (connection_sender, connection_receiver) = channel::<SocketAddr>();

    let mut last_heartbeat = std::time::Instant::now();
    // let relay_server_address = relay_server_address.clone();
    let relay_server_address_cloned = relay_server_address.clone();
    let player_name_cloned = player_name.clone();
    handle_connections(client, move |client, buffer, had_one| {
        // Before anything else, announce new connections to the server
        while let Ok(socket) = connection_receiver.try_recv() {
            println!("RELAYING CONNECTION FROM: {}", socket);
            local_outgoing_stream
                .write(
                    &bincode::serialize(&Packet::Connection(ConnectionPacket {
                        sender_name: client.player_name.clone(),
                        sender_port: client.player_port,
                        receiver_name: client.other_player_name.clone(),
                        receiver_port: client.other_player_port,
                        source_port: socket.port(),
                    }))
                    .unwrap(),
                )
                .unwrap();
        }

        // Every now and then, send a heartbeat packet over TCP UwU
        // While TCP itself should work, we wanna make sure no NAT shenanigans stops the game from working
        if last_heartbeat.elapsed().as_millis() > 500 {
            last_heartbeat = std::time::Instant::now();
            local_outgoing_stream
                .write(&bincode::serialize(&Packet::Heartbeat(player_name_cloned.clone())).unwrap())
                .unwrap();
        }

        /*
        let peers: HashSet<u16> = client
            .connections
            .data
            .lock()
            .unwrap()
            .iter()
            .map(|(k, _)| *k)
            .collect();
        */
        let mut packets = vec![]; // Packets to pass
        let mut connection_packets = vec![]; // Ignored by clients (?)
        let mut disconnected = vec![]; // Disconnected peers
        let mut commands = vec![]; // Ignored by clients
        let mut greetings = vec![]; // Ignored by clients
        let mut rejected_packets = vec![]; // Ignored by servers

        let mut connections = client.connections.clone();
        process_packets(
            &mut connections,
            // &peers,
            &mut packets,
            &mut connection_packets,
            &mut disconnected,
            &mut commands,
            &mut greetings,
            buffer,
            &mut rejected_packets,
            client.is_host(),
            client.other_player_name.clone(),
            client.other_player_port,
            &client.local_redirection_table,
        );
        process_disconnection(&mut connections, &mut disconnected);

        // These are the packets we received on the listener (should all always be local)
        // We will re-route them to the server.
        for (receiver_name, receiver_port, packet, source_port) in rejected_packets {
            print_packet(
                "local reject :: ",
                "self".to_string(),
                source_port,
                source_port,
                SocketType::Tcp,
                receiver_name.clone(),
                receiver_port,
                packet.len(),
            );
            // println!(
            // "self:{source_port} --(Tcp)--> {receiver_name}:{receiver_port} @ {}",
            // packet.len()
            // );
            local_outgoing_stream
                .write(
                    &bincode::serialize(&Packet::Data(DataPacket {
                        socket_type: SocketType::Tcp,
                        sender_name: client.player_name.clone(),
                        sender_port: client.player_port,
                        receiver_name,
                        receiver_port: if client.is_host() {
                            source_port
                        } else {
                            receiver_port
                        },
                        data: packet,
                        source_port,
                    }))
                    .unwrap()[..],
                )
                .unwrap(); // TODO: verify that this is OK
        }
        // Remember to also relay UDP!
        /*
        if *udp_packet_queue_size.lock().unwrap() > 0 || *relay_queue_size.lock().unwrap() > 0 {
            println!(
                "UDP PACKETS TO SEND TO A LOCAL ADDRESS: {:?}",
                udp_packet_queue_size.lock().unwrap()
            );
            println!(
                "UDP PACKETS TO SEND TO THE SERVER: {:?}",
                relay_queue_size.lock().unwrap()
            );
        }
        */
        while let Ok((udp_port, data)) = udp_packet_receiver.try_recv() {
            *udp_packet_queue_size.lock().unwrap() -= 1;

            //
            if client.is_host() {
                // We're the host!
                // We need to parse the packet to check if it's a data packet.
                // If it is, we need to ensure we have the udp socket existing
                if let Ok(packet) = bincode::deserialize::<Packet>(&data) {
                    // Data packet
                    match packet {
                        Packet::Data(data_packet) => {
                            client.ensure_udp_socket_on_redirection_table(&data_packet);

                            data_packet.print("RELATING A DATA PACKET TO A LOCAL CONNECTION: ");

                            let player_identifier = data_packet.get_original_player_identifier();
                            // println!("IDENTIFIER: {}", player_identifier);
                            // client.ensure_udp_socket_on_redirection_table(data);
                            if let Some(local_client_connection) =
                                client.local_redirection_table.get(&player_identifier)
                            {
                                // println!("RELAYING TO: 127.0.0.1:{}", data_packet.receiver_port);
                                let _ = local_client_connection
                                    .udp_socket
                                    .as_ref()
                                    .unwrap()
                                    .send_to(
                                        &data_packet.data,
                                        format!("127.0.0.1:{}", data_packet.receiver_port),
                                    );
                                // let (player_name, _player_port, source_port) = (
                                // local_client_connection.player_name.clone(),
                                // local_client_connection.port,
                                // local_client_connection.original_socket_port,
                                // );
                                // println!("redirection table contains an entry for the udp port {udp_port}");
                                // (player_name, source_port.clone(), source_port.clone());
                            } else {
                                panic!("redirection table DOESNT contain an entry for the udp port {udp_port}");
                            }
                        }
                        Packet::Heartbeat(_) => {
                            // ignore it, the server is just pinging us back
                            // println!("heartbeat: {}", udp_port);
                        }
                        _ => {
                            panic!("NOT A DATA PACKET!");
                        }
                    }
                } else {
                    // Not a data packet, instead, its unstructured information
                    panic!("UNSTRUCTURED DATA!");
                }

            /*
            let player_identifier = data.get_original_player_identifier();
            println!("IDENTIFIER: {}", player_identifier);
            // client.ensure_udp_socket_on_redirection_table(data);
            if let Some(local_client_connection) =
                client.local_redirection_table.get(&player_identifier)
            {
                let (player_name, _player_port, source_port) = (
                    local_client_connection.player_name.clone(),
                    local_client_connection.port,
                    local_client_connection.original_socket_port,
                );
                // println!("redirection table contains an entry for the udp port {udp_port}");
                (player_name, source_port.clone(), source_port.clone())
            } else {
                panic!("redirection table DOESNT contain an entry for the udp port {udp_port}");
                // TODO: should we really panic?
                // (
                // client.other_player_name.clone(),
                // client.other_player_port.clone(),
                // udp_port,
                // )
            }
            */
            } else {
                if *relay_queue_size.lock().unwrap() < MAX_QUEUE_SIZE {
                    if let Ok(packet) = bincode::deserialize::<Packet>(&data) {
                        // Data packet
                        match packet {
                            Packet::Data(data_packet) => {
                                data_packet.print("RECEIVED FOR RELAY ");

                                relay_packet_sender
                                    .send((
                                        format!("127.0.0.1:{}", data_packet.receiver_port),
                                        data_packet.data,
                                    ))
                                    .unwrap();
                                *relay_queue_size.lock().unwrap() += 1;
                            }
                            Packet::Heartbeat(_) => {
                                // ignore it, the server is just pinging us back
                                // println!("heartbeat: {}", udp_port);
                            }
                            _ => {
                                // ignore
                                println!("RECEIVED WEIRD STRUCTURED DATA!");
                            }
                        }
                    } else {
                        let data_packet = DataPacket {
                            socket_type: SocketType::Udp,
                            sender_name: client.player_name.clone(),
                            sender_port: client.player_port,
                            receiver_name: client.other_player_name.clone(),
                            receiver_port: client.other_player_port,
                            data,
                            source_port: udp_port, // TODO: fix this? If it's even an issue
                        };
                        data_packet.print("RELAYING TO SERVER ");
                        relay_packet_sender
                            .send((
                                relay_server_address.clone(),
                                bincode::serialize(&Packet::Data(data_packet)).unwrap(),
                            ))
                            .unwrap();
                        *relay_queue_size.lock().unwrap() += 1;
                    }
                } else {
                    println!("DROPPING A UDP PACKET! Queue over capacity...");
                }
            };
            // println!("UDP relay through server for {other_player_name}:{other_player_port} ({source_port}) @ {}", data.len());
            // using udp
            //*
            // */
            /*
            // using tcp
            local_outgoing_stream
                .write(
                    &bincode::serialize(&Packet::Data(DataPacket {
                        socket_type: SocketType::Udp,
                        sender_name: client.player_name.clone(),
                        sender_port: client.player_port,
                        receiver_name: other_player_name,
                        receiver_port: other_player_port,
                        data,
                        source_port, // TODO: fix this? If it's even an issue
                    }))
                    .unwrap()[..],
                )
                .unwrap(); // TODO: verify that this is OK
                           // */
        }

        // After reading packets, we also need to receive packets from the server...
        if let Ok(data) = local_outgoing_stream.peek(buffer) {
            if data == 0 {
                panic!("SERVER TIMEOUT!");
            }
        }
        // Receive packets from connected clients and send them to the server...
        for (_, local_connection) in client.local_redirection_table.iter_mut() {
            //
            // println!(
            // "TRYING TO READ FOR: {}:{} ({})",
            // local_connection.player_name,
            // local_connection.port,
            // local_connection.original_socket_port
            // );
            if local_connection.stream.is_some() {
                if let Ok(size) = local_connection.stream.as_ref().unwrap().read(buffer) {
                    *had_one = true;

                    let data = &buffer[..size];

                    // Check if the data is structured.
                    if let Ok(_r) = bincode::deserialize::<Packet>(data) {
                        // Structured data, shouldnt happen...
                        println!("Received structured data on a local client tcp socket! This should not happen!");
                    } else {
                        // Unstructured data
                        // Needs to be relayed to the server...
                        let packet = DataPacket {
                            socket_type: SocketType::Tcp,
                            sender_name: client.player_name.clone(),
                            sender_port: client.player_port,
                            receiver_name: local_connection.player_name.clone(),
                            receiver_port: local_connection.original_socket_port,
                            data: data.to_vec(),
                            source_port: local_connection
                                .stream
                                .as_ref()
                                .unwrap()
                                .peer_addr()
                                .unwrap()
                                .port(),
                        };
                        packet.print("SENDING TO THE SERVER: ");
                        let data_packet = Packet::Data(packet);
                        if let Err(e) =
                            local_outgoing_stream.write(&bincode::serialize(&data_packet).unwrap())
                        {
                            match e.kind() {
                                ErrorKind::WouldBlock => {
                                    //
                                }
                                _ => {
                                    println!(
                                        "Failed to relay unstructured data for {}:{} ({})",
                                        local_connection.player_name,
                                        local_connection.port,
                                        local_connection.original_socket_port
                                    )
                                }
                            }
                        }
                    }
                } else {
                    // Failed to read
                }
            } else {
                // No tcp stream
            }
            if local_connection.udp_socket.is_some() {
                if let Ok((size, addr)) = local_connection
                    .udp_socket
                    .as_ref()
                    .unwrap()
                    .recv_from(buffer)
                {
                    *had_one = true;

                    local_connection.received_udp_packets_counts += 1;
                    // println!(
                    // "RECEIVED UDP PACKETS: {}",
                    // local_connection.received_udp_packets_counts
                    // );

                    // If we receive data, it means we need to relay it to the server!
                    let data = &buffer[..size];

                    // Check if the data is structured.
                    if let Ok(_r) = bincode::deserialize::<Packet>(data) {
                        // Structured data, shouldnt happen...
                        println!("Received structured data on a local client udp socket! This should not happen!");
                    } else if *relay_queue_size.lock().unwrap() < MAX_QUEUE_SIZE {
                        let data_vec = bincode::serialize(&Packet::Data(DataPacket {
                            socket_type: SocketType::Udp,
                            sender_name: client.player_name.clone(),
                            sender_port: client.player_port,
                            receiver_name: local_connection.player_name.clone(),
                            receiver_port: local_connection.original_socket_port,
                            data: data.to_vec(),
                            source_port: addr.port(),
                        }))
                        .unwrap();
                        relay_packet_sender
                            .send((relay_server_address.clone(), data_vec))
                            .unwrap();
                        *relay_queue_size.lock().unwrap() += 1;
                    } else {
                        println!("Queue over capacity! Dropping the udp thingy...");
                    }
                }
            } else {
                // No udp stream
            }
        }
        // Receive data from the server and relay it to local connections
        match local_outgoing_stream.read(buffer) {
            Ok(received_data) => {
                let _socket_port = local_outgoing_stream.peer_addr().unwrap().port();

                // This is data received from the server.
                let received_data = &buffer[..received_data];
                if let Ok(value) = bincode::deserialize::<Packet>(received_data) {
                    match value {
                        Packet::Data(data) => {
                            // let socket_type = data.socket_type;
                            data.print("packet received from the server: ");
                            if client.player_name != data.receiver_name {
                                println!("Received data meant for another player! Weird!");
                            } else {
                                // let receiver_port = data.receiver_port;
                                // let address = SocketAddr::from_str(
                                // format!("127.0.0.1:{}", receiver_port).as_str(),
                                // )
                                // .unwrap();

                                // Create the socket if it doesn't exist yet
                                if client.is_host() {
                                    // Host logic
                                    client.ensure_tcp_socket_on_redirection_table(&data);

                                    // Send data to the TCP socket
                                    if let Some(local_connection) = client
                                        .local_redirection_table
                                        .get_mut(&data.get_original_player_identifier())
                                    {
                                        //
                                        if data.socket_type == SocketType::Tcp {
                                            if let Err(e) = local_connection
                                                .stream
                                                .as_ref()
                                                .unwrap()
                                                .write(&data.data)
                                            {
                                                match e.kind() {
                                                    std::io::ErrorKind::WouldBlock => {
                                                        // nothing to do...
                                                    }
                                                    _ => {
                                                        println!(
                                                            "LOCAL TCP SOCKET SEND ERROR: {}",
                                                            e
                                                        );
                                                    }
                                                }
                                            }
                                        } else {
                                            println!("TCP CONNECTIONS CAN ONLY SEND TCP DATA!");
                                        }
                                    }
                                } else {
                                    // Client logic
                                    // println!("CLIENT");
                                    // let locked_connections = connections.data.lock().unwrap();
                                    // print_connections(&connections);
                                    let locked = connections.data.lock().unwrap();
                                    if let Some(player) = locked.get(&data.receiver_port) {
                                        match player.stream.write(&data.data[..]) {
                                            Ok(_) => {
                                                // nothing to do, we sent the data!
                                            }
                                            Err(e) => {
                                                match e.kind() {
                                                    ErrorKind::WouldBlock => {
                                                        //
                                                    }
                                                    _ => {
                                                        println!("ERROR WHEN SENDING A TCP PACKET!")
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        println!("Packed received for a non existing socket!");
                                    }
                                }
                            }
                        }
                        Packet::GreetingReply => {
                            println!("Received a greeting reply from the server! TCP connection established!");
                        }
                        Packet::Connection(con) => {
                            if client.is_host() {
                                println!(
                                    "New tcp connection established: {}:{} ({}) -> {}:{}",
                                    con.sender_name,
                                    con.sender_port,
                                    con.source_port,
                                    con.receiver_name,
                                    con.receiver_port
                                );
                                // Host logic
                                client.ensure_tcp_socket_on_redirection_table(&con);
                            } else {
                                println!("Received a connection packet on non host rubicon instance. That's weird! {:?}", con);
                            }
                        }
                        _ => {
                            println!(
                                "Weird packet received from the server. Are we being hacked? {:?}",
                                value
                            );
                        }
                    }
                }

                // *had_one = true;
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock => {
                    // dont print when not debugging - itll flood the console cuz most of the time there's nothing to read...
                    // println!("A stream ({:?}) would block upon reading: {:?}", peer, e)
                }
                _ => println!("The server stream returned an error upon reading: {:?}", e),
            },
            //
        }
    });

    let listener = TcpListener::bind(format!("0.0.0.0:{}", player_client_port)).unwrap();
    handle_udp_traffic(
        (format!("0.0.0.0:{}", player_client_port), udp_packet_sender),
        relay_packet_receiver,
        udp_packet_queue_size_cloned,
        relay_queue_size_cloned,
        relay_server_address_cloned,
        player_name.clone(),
    );
    accept_connections(&listener, connections, Some(connection_sender));
}

/// Connects to an address and starts sending tcp packets to it.
fn ping(port: u16, address: String, udp: SocketType, data_size: usize) {
    println!("Pinging {} as {:?}", address, udp);
    // Outgoing stream

    let mut buf = [0u8; BUFFER_SIZE];

    let buffer_to_send = vec![0; data_size]
        .iter()
        .enumerate()
        .map(|(i, _)| (i % u8::MAX as usize) as u8)
        .collect::<Vec<u8>>();

    match udp {
        SocketType::Udp => {
            //
            println!("Binding a udp socket on 0.0.0.0:{port}");
            let udp = UdpSocket::bind(format!("0.0.0.0:{port}")).unwrap();
            udp.set_nonblocking(true).unwrap();

            // let mut o = 0;
            let mut sent_out = 0;
            let mut received = 0;
            loop {
                // std::thread::sleep(Duration::from_millis(500));
                let _ = udp.send_to(&buffer_to_send, address.clone());
                sent_out += 1;
                // o += 1;
                // println!("{}", o);

                if let Ok((data_size, addr)) = udp.recv_from(&mut buf) {
                    println!("Received back data of size: {data_size}, from {addr}");
                    received += 1;
                    println!("SENT: {}   VS   RECEIVED: {}", sent_out, received)
                }

                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        SocketType::Tcp => {
            let mut stream = TcpStream::connect(address).unwrap();
            // stream.set_nonblocking(true).unwrap();
            stream.set_nodelay(DISABLE_NAGLE_ALGORITHM).unwrap();

            // let mut o = 0;
            loop {
                std::thread::sleep(Duration::from_millis(500));
                let time = std::time::Instant::now();
                let _ = stream.write(&buffer_to_send);
                // o += 1;
                // println!("{}", o);

                if let Ok(data_size) = stream.read(&mut buf) {
                    println!("Received back data of size: {data_size}");
                }
                println!("PING: {}", time.elapsed().as_millis());
            }
        }
    }
}

/// Opens a tcp sockets and starts listening on it, while pinging back every now and then
fn listen(port: u16, udp: SocketType) {
    println!("Listening on {} as {:?}", port, udp);

    match udp {
        SocketType::Udp => {
            println!("Binding a udp socket on 0.0.0.0:{}", port);
            let socket = UdpSocket::bind(format!("0.0.0.0:{}", port).as_str()).unwrap();
            let mut buf = [0u8; BUFFER_SIZE];
            let mut counters = std::collections::HashMap::<SocketAddr, i32>::new();
            let mut total_counter = 0;
            loop {
                if let Ok((size, addr)) = socket.recv_from(&mut buf) {
                    let data = buf[..size].to_vec();
                    println!("Received data of size {} from address {}", data.len(), addr);

                    let counter = counters.entry(addr).or_default();
                    *counter += 1;
                    total_counter += 1;
                    println!("RECEIVED {} PACKETS TOTAL", total_counter);
                    if *counter % 3 == 1 {
                        println!("Pinging back on the same tcp connection...");
                        let _sent = socket.send_to(&data, addr); // TODO: handle this gracefully
                    }
                }
            }
        }
        SocketType::Tcp => {
            let connections = Connections::new();
            let mut counters = std::collections::HashMap::<SocketAddr, i32>::new();

            handle_connections(connections.clone(), move |connections, buffer, _had_one| {
                let mut connections = connections.data.lock().unwrap();
                for (port, player_data) in connections.iter_mut() {
                    let stream = &mut player_data.stream;
                    if let Ok(read) = stream.read(buffer) {
                        let data = buffer[..read].to_vec();
                        println!(
                            "Received tcp data of size {} from port {}",
                            data.len(),
                            port
                        );

                        let counter = counters.entry(stream.get_tcp_addr().unwrap()).or_default();
                        *counter += 1;
                        // if *counter % 4 == 2 {
                        println!(
                            "Pinging back on the same tcp connection -> {:?} @ {}",
                            stream.get_tcp_addr(),
                            data.len()
                        );
                        let _sent = stream.write(&data); // TODO: handle this gracefully
                                                         // }
                    }
                }
            });

            let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();
            accept_connections(&listener, connections, None);
        }
    }
}

/// Opens a tcp sockets and starts listening on it
fn send_command(address: String, command: String) {
    println!("Commanding {} to {}", address, command);
    // Outgoing stream
    let mut stream = TcpStream::connect(address).unwrap();
    stream.set_nonblocking(false).unwrap();
    stream.set_nodelay(DISABLE_NAGLE_ALGORITHM).unwrap();

    let data = bincode::serialize(&Packet::Command(CommandPacket { command })).unwrap();
    let _ = stream.write(&data[..]);
}
