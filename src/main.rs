pub mod client;
pub mod commands;
pub mod common;
pub mod packet;
pub mod server;
pub mod socket;

use std::{
    collections::HashSet,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream, UdpSocket},
    str::FromStr,
    sync::mpsc::channel,
    time::Duration,
};

use clap::Parser;
use client::ClientState;
use commands::{Args, Commands, SocketType};
use common::{accept_connections, handle_connections, DISABLE_NAGLE_ALGORITHM};
use packet::{process_packets, CommandPacket, DataPacket, GreetingPacket, Packet};
use server::{Connections, PlayerData, ServerState};
use socket::SocketWrapper;

fn main() {
    let args = Args::parse();
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
        Commands::Ping { address, socket } => ping(address, socket),
        Commands::Listen { port, socket } => listen(port, socket),
        Commands::Command { address, command } => send_command(address, command),
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
        let mut disconnected = vec![]; // Disconnected peers
        let mut commands = vec![];
        let mut greetings = vec![];
        let mut rejected_packets = vec![]; // Ignored by servers

        let mut connections = server_state.connections.clone();
        process_packets(
            &mut connections,
            // &peers,
            &mut packets,
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
        relay_packets(server_state, &mut packets);
        process_disconnection(&mut connections, &mut disconnected);

        // Process received data
        server_state.receive_greetings(greetings);
        // server_state.receive_data_packets(packets);
        server_state.receive_commands(commands);
    });

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
    accept_connections(&listener, None, None, connections);
}

fn relay_packets(server: &mut ServerState, packets: &mut Vec<(u16, DataPacket)>) {
    let mut locked_connections = server.connections.data.lock().unwrap();
    for (_port, packet) in packets {
        // We need to find the player to retrieve the data from.
        let receiver_name = packet.receiver_name.clone();
        // println!("Relaying a packet to {}", receiver_name);
        if let Some((_, player_data)) = locked_connections
            .iter_mut()
            .find(|element| element.1.name == receiver_name)
        {
            // We need to construct a new packet!
            if let Err(e) = player_data
                .stream
                .write(&bincode::serialize(&Packet::Data(packet.clone())).unwrap())
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
                // println!(
                // "Packet delivered from port {} to player {}",
                // port, receiver_name
                // );
            }
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

fn connect(
    port: u16,
    relay_server_address: String,
    player_name: String,
    other_player_name: String,
    other_player_port: u16,
) {
    // The stream that accepts packets from local entities
    let mut local_outgoing_stream = TcpStream::connect(relay_server_address.clone()).unwrap();
    local_outgoing_stream
        .set_nodelay(DISABLE_NAGLE_ALGORITHM)
        .unwrap();
    // ALWAYS begin by sending our name!
    local_outgoing_stream
        .write(
            &bincode::serialize(&Packet::Greeting(GreetingPacket {
                player_name: player_name.clone(),
            }))
            .unwrap()[..],
        )
        .unwrap();
    local_outgoing_stream.set_nonblocking(true).unwrap(); // set non blocking AFTER we send the packet

    let (udp_packet_sender, udp_packet_receiver) = channel::<(u16, Vec<u8>)>();
    let (relay_packet_sender, relay_packet_receiver) = channel::<(String, Vec<u8>)>();

    // Incomming stream
    let client = ClientState::new(player_name, port, other_player_name, other_player_port);
    let connections = client.connections.clone();

    // let relay_server_address = relay_server_address.clone();
    handle_connections(client, move |client, buffer, had_one| {
        let peers: HashSet<u16> = client
            .connections
            .data
            .lock()
            .unwrap()
            .iter()
            .map(|(k, _)| *k)
            .collect();
        let mut packets = vec![]; // Packets to pass
        let mut disconnected = vec![]; // Disconnected peers
        let mut commands = vec![]; // Ignored by clients
        let mut greetings = vec![]; // Ignored by clients
        let mut rejected_packets = vec![]; // Ignored by servers

        let mut connections = client.connections.clone();
        process_packets(
            &mut connections,
            // &peers,
            &mut packets,
            &mut disconnected,
            &mut commands,
            &mut greetings,
            buffer,
            &mut rejected_packets,
            client.is_host(),
            client.other_player_name.clone(),
            client.other_player_port,
            &client.player_redirection_table,
        );
        process_disconnection(&mut connections, &mut disconnected);

        // These are the packets we received on the listener (should all always be local)
        // We will re-route them to the server.
        for (receiver_name, receiver_port, packet, source_port) in rejected_packets {
            // println!(
            // "Relaying a tcp packet of size {} for {receiver_name}:{receiver_port}, with source {source_port}, to the server",
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
        while let Ok((udp_port, data)) = udp_packet_receiver.try_recv() {
            // println!("Relaying a udp packet of size {} to the server", data.len());

            let (other_player_name, other_player_port, source_port) = if client.is_host() {
                if let Some((player_name, _player_port, source_port)) =
                    client.player_redirection_table.get(&udp_port)
                {
                    // println!("redirection table contains an entry for the udp port {udp_port}");
                    (
                        player_name.clone(),
                        source_port.clone(),
                        source_port.clone(),
                    )
                } else {
                    panic!("redirection table DOESNT contain an entry for the udp port {udp_port}");
                    // TODO: should we really panic?
                    // (
                    // client.other_player_name.clone(),
                    // client.other_player_port.clone(),
                    // udp_port,
                    // )
                }
            } else {
                (
                    client.other_player_name.clone(),
                    client.other_player_port.clone(),
                    udp_port,
                )
            };
            println!("UDP relay through server for {other_player_name}:{other_player_port} ({source_port}) @ {}", data.len());
            // using udp
            /*
            relay_packet_sender
                .send((
                    relay_server_address.clone(),
                    bincode::serialize(&Packet::Data(DataPacket {
                        socket_type: SocketType::Udp,
                        sender_name: client.player_name.clone(),
                        sender_port: client.player_port,
                        receiver_name: other_player_name,
                        receiver_port: other_player_port,
                        data,
                        source_port, // TODO: fix this? If it's even an issue, kekw
                    }))
                    .unwrap(),
                ))
                .unwrap();
            */
            //*
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
                        source_port, // TODO: fix this? If it's even an issue, kekw
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
        match local_outgoing_stream.read(buffer) {
            Ok(received_data) => {
                let _socket_port = local_outgoing_stream.peer_addr().unwrap().port();

                // This is data received from the server.
                let received_data = &buffer[..received_data];
                if let Ok(value) = bincode::deserialize::<Packet>(&received_data) {
                    match value {
                        Packet::Data(data) => {
                            let socket_type = data.socket_type;
                            if client.player_name != data.receiver_name {
                                // println!("Received data meant for another player! Weird!");
                            } else {
                                let receiver_port = data.receiver_port;
                                let address = SocketAddr::from_str(
                                    format!("127.0.0.1:{}", receiver_port).as_str(),
                                )
                                .unwrap();
                                // println!(
                                // "Received {} bytes for port {}, from {} for {} on socket {}",
                                // data.data.len(),
                                // receiver_port,
                                // data.sender_name,
                                // data.receiver_name,
                                // local_outgoing_stream.peer_addr().unwrap()
                                // );

                                // Hosts need to keep their redirection tables in sync!
                                if client.is_host() {
                                    if !client
                                        .player_redirection_table
                                        .contains_key(&data.receiver_port)
                                    {
                                        // println!(
                                        // "Adding a new redirection table entry: {} -> {}:{}, with source port {}",
                                        // data.receiver_port, data.sender_name, data.sender_port, data.source_port
                                        // );
                                        client.player_redirection_table.insert(
                                            data.receiver_port,
                                            (
                                                data.sender_name.clone(),
                                                data.sender_port,
                                                data.source_port,
                                            ),
                                        );
                                    }
                                }

                                let binary_data = &data.data;
                                // Well, now we need to send it!
                                if socket_type == SocketType::Tcp
                                    && (!peers.contains(&receiver_port)
                                        || !connections
                                            .data
                                            .lock()
                                            .unwrap()
                                            .get(&receiver_port)
                                            .unwrap()
                                            .stream
                                            .has_tcp())
                                {
                                    // println!("Failed to find the desired port. Attempting to create a new socket for this connection...");

                                    // Check if the packet is udp. If it is, a peer isn't needed in the first place!
                                    match socket_type {
                                        SocketType::Udp => {
                                            // This should never happen!
                                        }
                                        SocketType::Tcp => {
                                            let socket_result = TcpStream::connect(address);
                                            match socket_result {
                                	    	    Ok(connected_socket) => {
													connected_socket.set_nonblocking(true).unwrap();
    												connected_socket.set_nodelay(DISABLE_NAGLE_ALGORITHM).unwrap();
                                				    let mut connections = connections.data.lock().unwrap();
													if let Some(existing) = connections.get_mut(&receiver_port) {
														existing.stream.fill_in_tcp(connected_socket);
													} else {
													let stream = SocketWrapper::from_tcp_socket(connected_socket);
													connections.insert(receiver_port, PlayerData {
        											    address,
        											    stream,
        											    name: "localhost".to_string()
        											});
												}
											},
                                    	    Err(e) => println!("Failed to create the socket, dropping the packet. Error message: {}", e),
                                    		}
                                        }
                                    }
                                }

                                if socket_type == SocketType::Udp {
                                    // println!("Attempting delivery for {:?}", socket_type);
                                    let mut connections = connections.data.lock().unwrap();
                                    println!(
                                        "RECEIVED FROM SERVER (UDP) :: {}:{} -> {}:{} @ {}",
                                        data.sender_name,
                                        data.sender_port,
                                        data.receiver_name,
                                        data.receiver_port,
                                        binary_data.len()
                                    );
                                    relay_packet_sender
                                        .send((address.to_string(), binary_data.clone()))
                                        .unwrap();
                                    if let Some(target_stream) = connections.get_mut(&receiver_port)
                                    {
                                        target_stream.stream.touch();
                                    }
                                }
                                if peers.contains(&receiver_port) {
                                    let mut connections = connections.data.lock().unwrap();
                                    let target_stream =
                                        connections.get_mut(&receiver_port).unwrap();
                                    match socket_type {
                                        SocketType::Udp => {
                                            // nothing to do, handled above
                                        }
                                        SocketType::Tcp => {
                                            // println!(
                                            // "TCP Port found, attempting delivery for {:?}",
                                            // socket_type
                                            // );
                                            assert!(target_stream.stream.has_tcp(), "There must be a tcp socket in place to deliver tcp traffic!");
                                            target_stream.stream.write(&binary_data[..]).unwrap();
                                            // TODO: verify that this is fine
                                        }
                                    }
                                    // println!("Delivery succesful");
                                }
                            }
                        }
                        _ => {
                            // println!("Weird packet received from the server. Are we being hacked?");
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

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
    accept_connections(
        &listener,
        Some((format!("0.0.0.0:{}", port), udp_packet_sender)),
        Some(relay_packet_receiver),
        connections,
    );
}

/// Connects to an address and starts sending tcp packets to it.
fn ping(address: String, udp: SocketType) {
    println!("Pinging {} as {:?}", address, udp);
    // Outgoing stream

    match udp {
        SocketType::Udp => {
            //
            println!("Binding a udp socket on 0.0.0.0:45333");
            let udp = UdpSocket::bind("0.0.0.0:45333").unwrap();

            let mut o = 0;
            loop {
                std::thread::sleep(Duration::from_millis(500));
                let _ = udp.send_to(&[1, 2, 3, 5, 7, 11, 13], address.clone());
                o += 1;
                println!("{}", o);
            }
        }
        SocketType::Tcp => {
            let mut stream = TcpStream::connect(address).unwrap();
            stream.set_nonblocking(true).unwrap();
            stream.set_nodelay(DISABLE_NAGLE_ALGORITHM).unwrap();

            let mut o = 0;
            loop {
                std::thread::sleep(Duration::from_millis(500));
                let _ = stream.write(&[1, 2, 3, 5, 7, 11, 13]);
                o += 1;
                println!("{}", o);
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
            let mut buf = [0u8; 1024 * 64];
            let mut counter = 0;
            loop {
                if let Ok((size, addr)) = socket.recv_from(&mut buf) {
                    let data = buf[..size].to_vec();
                    println!("Received data of size {} from address {}", data.len(), addr);

                    counter += 1;
                    if counter % 7 == 3 {
                        println!("Pinging back on the same tcp connection...");
                        let _sent = socket.send_to(&data, addr); // TODO: handle this gracefully
                    }
                }
            }
        }
        SocketType::Tcp => {
            let connections = Connections::new();

            let mut counter = 0;
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
                        counter += 1;
                        if counter % 7 == 3 {
                            println!("Pinging back on the same tcp connection...");
                            let _sent = stream.write(&data); // TODO: handle this gracefully
                        }
                    }
                }
            });

            let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
            accept_connections(&listener, None, None, connections);
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
