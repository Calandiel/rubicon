pub mod client;
pub mod commands;
pub mod common;
pub mod packet;
pub mod server;

use std::{
    collections::HashSet,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    time::Duration,
};

use clap::Parser;
use client::ClientState;
use commands::{Args, Commands};
use common::{accept_connections, handle_connections};
use packet::{process_packets, CommandPacket, DataPacket, GreetingPacket, Packet};
use server::{Connections, ServerState};

fn main() {
    let args = Args::parse();
    // Dispatch from cli
    match args.command {
        Commands::Host { port } => host(port),
        Commands::Connect {
            port,
            address,
            player_name,
            other_player_name,
            other_player_port,
        } => connect(
            port,
            address,
            player_name,
            other_player_name,
            other_player_port,
        ),
        Commands::Ping { address } => ping(address),
        Commands::Listen { port } => listen(port),
        Commands::Command { address, command } => send_command(address, command),
    }
}

fn host(port: u16) {
    println!("Hosting {}", port);
    // Listener uwu

    let server_state = ServerState::new();
    let connections = server_state.connections.clone();

    // process existing connections - we need to read the data from them and then pass it to the intended receiver

    handle_connections(server_state, |server_state, buffer| {
        let peers: HashSet<u16> = server_state
            .connections
            .data
            .lock()
            .unwrap()
            .iter()
            .map(|(k, _)| *k)
            .collect();
        let mut packets = vec![]; // Packets to pass
        let mut disconnected = vec![]; // Disconnected peers
        let mut commands = vec![];
        let mut greetings = vec![];
        let mut rejected_packets = vec![]; // Ignored by servers

        let mut connections = server_state.connections.clone();
        process_packets(
            &mut connections,
            &peers,
            &mut packets,
            &mut disconnected,
            &mut commands,
            &mut greetings,
            buffer,
            &mut rejected_packets,
        );
        relay_packets(server_state, &mut packets);
        process_disconnection(&mut connections, &mut disconnected);

        // Process received data
        server_state.receive_greetings(greetings);
        // server_state.receive_data_packets(packets);
        server_state.receive_commands(commands);
    });

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
    accept_connections(&listener, connections);
}

fn relay_packets(server: &mut ServerState, packets: &mut Vec<(u16, DataPacket)>) {
    let mut locked_connections = server.connections.data.lock().unwrap();
    for (port, packet) in packets {
        // We need to find the player to retrieve the data from.
        let receiver_name = packet.receiver_name.clone();
        println!("Relaying a packet to {}", receiver_name);
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
                        println!(
                            "A stream ({:?}) would block upon writing: {:?}",
                            player_data.address.clone(),
                            e
                        )
                    }
                    _ => println!(
                        "A stream ({:?}) returned an error upon writing: {:?}",
                        player_data.address.clone(),
                        e
                    ),
                }
            } else {
                println!(
                    "Packet delivered from port {} to player {}",
                    port, receiver_name
                );
            }
        } else {
            println!("Packet delivery to player {} attempted by player at port {} but the target player was not found!", receiver_name, port);
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
    address: String,
    player_name: String,
    other_player_name: String,
    other_player_port: u16,
) {
    // Outgoing stream
    let mut stream = TcpStream::connect(address).unwrap();
    // ALWAYS begin by sending our name!
    stream
        .write(
            &bincode::serialize(&Packet::Greeting(GreetingPacket {
                player_name: player_name.clone(),
            }))
            .unwrap()[..],
        )
        .unwrap();
    stream.set_nonblocking(true).unwrap(); // set non blocking AFTER we send the packet

    // Incomming stream
    let client = ClientState::new(player_name, other_player_name, other_player_port);
    let connections = client.connections.clone();

    handle_connections(client, move |client, buffer| {
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
            &peers,
            &mut packets,
            &mut disconnected,
            &mut commands,
            &mut greetings,
            buffer,
            &mut rejected_packets,
        );
        process_disconnection(&mut connections, &mut disconnected);

        // These are the packets we received on the listener (should all always be local)
        // We will re-route them to the server.
        for packet in rejected_packets {
            println!("Relaying the packet of size {} to the server", packet.len());
            stream
                .write(
                    &bincode::serialize(&Packet::Data(DataPacket {
                        receiver_name: client.other_player_name.clone(),
                        receiver_port: client.other_player_port,
                        data: packet,
                    }))
                    .unwrap()[..],
                )
                .unwrap(); // TODO: verify that this is OK
        }

        // After reading packets, we also need to receive packets from the server...
        if let Ok(data) = stream.peek(buffer) {
            if data == 0 {
                panic!("SERVER TIMEOUT!");
            }
        }
        match stream.read(buffer) {
            Ok(received_data) => {
                // This is data received from the server.
                let received_data = &buffer[..received_data];
                println!("here be dragons");
                if let Ok(value) = bincode::deserialize::<Packet>(&received_data) {
                    match value {
                        Packet::Data(data) => {
                            if client.player_name != data.receiver_name {
                                println!("Received data meant for another player! Weird!");
                            } else {
                                let port = data.receiver_port;
                                let data = data.data;
                                println!("Received {} bytes for port {}", data.len(), port);
                            }
                        }
                        _ => {
                            println!("Weird packet received from the server. Are we being hacked?");
                        }
                    }
                }
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
    accept_connections(&listener, connections);
}

/// Connects to an address and starts sending tcp packets to it.
fn ping(address: String) {
    println!("Pinging {}", address);
    // Outgoing stream
    let mut stream = TcpStream::connect(address).unwrap();
    stream.set_nonblocking(true).unwrap();

    let mut o = 0;
    loop {
        std::thread::sleep(Duration::from_millis(500));
        let _ = stream.write(&[1, 2, 3, 5, 7, 11, 13]);
        o += 1;
        println!("{}", o);
    }
}

/// Opens a tcp sockets and starts listening on it
fn listen(port: u16) {
    println!("Listening on {}", port);

    let connections = Connections::new();

    handle_connections(connections.clone(), |connections, buffer| {
        //
    });

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
    accept_connections(&listener, connections);
}

/// Opens a tcp sockets and starts listening on it
fn send_command(address: String, command: String) {
    println!("Commanding {} to {}", address, command);
    // Outgoing stream
    let mut stream = TcpStream::connect(address).unwrap();
    stream.set_nonblocking(false).unwrap();

    let data = bincode::serialize(&Packet::Command(CommandPacket { command })).unwrap();
    let _ = stream.write(&data[..]);
}
