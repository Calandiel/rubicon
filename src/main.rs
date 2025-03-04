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
use commands::{Args, Commands};
use common::{accept_connections, handle_connections};
use packet::{DataPacket, Packet};
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
        } => connect(port, address, player_name),
        Commands::Ping { address } => ping(address),
        Commands::Listen { port } => listen(port),
    }
}

fn host(port: u16) {
    println!("Hosting {}", port);
    // Listener uwu

    let server_state = ServerState::new();

    // process existing connections - we need to read the data from them and then pass it to the intended receiver

    handle_connections(
        server_state.connections.clone(),
        |mut connections, buffer| {
            let peers: HashSet<u16> = connections
                .data
                .lock()
                .unwrap()
                .iter()
                .map(|(k, _)| *k)
                .collect();
            let mut packets = vec![]; // Packets to pass
            let mut disconnected = vec![]; // Disconnected peers

            process_packets(
                &mut connections,
                &peers,
                &mut packets,
                &mut disconnected,
                buffer,
            );
            relay_packets(&mut connections, &mut packets);
            process_disconnection(&mut connections, &mut disconnected);
        },
    );

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
    accept_connections(&listener, server_state.connections);
}

/// Processes incomming packets
fn process_packets(
    connections: &mut Connections,
    peers: &HashSet<u16>,
    packets: &mut Vec<DataPacket>,
    disconnected: &mut Vec<u16>,
    buffer: &mut [u8],
) {
    let mut locked_connections = connections.data.lock().unwrap();
    for (port, player_data) in locked_connections.iter_mut() {
        // println!("Checking");
        // Check for disconnects
        if let Ok(size) = player_data.stream.peek(buffer) {
            if size == 0 {
                // Potential timeout?
                disconnected.push(*port);
                continue;
            }
        }

        match player_data.stream.read(buffer) {
            Ok(value) => {
                println!(
                    "Received data of size {} from {}",
                    value, player_data.address
                );
                let sliced_data = &buffer[..value];
                let deserialize = bincode::deserialize::<Packet>(sliced_data);
                if let Ok(packet) = deserialize {
                    // Check if the received port exists!
                    match packet {
                        Packet::Data(data) => {
                            if peers.contains(&data.receiver_port) {
                                packets.push(data);
                            }
                        }
                        Packet::NetworkTopology(_) => todo!(),
                    }
                } else {
                    println!("Failed to decode the packet.");
                }
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock => {
                    // dont print when not debugging - itll flood the console cuz most of the time there's nothing to read...
                    // println!("A stream ({:?}) would block upon reading: {:?}", peer, e)
                }
                _ => println!(
                    "A stream ({:?}) returned an error upon reading: {:?}",
                    player_data.address, e
                ),
            },
        }
    }
}

fn relay_packets(connections: &mut Connections, packets: &mut Vec<DataPacket>) {
    let mut locked_connections = connections.data.lock().unwrap();
    for packet in packets {
        if let Some(player_data) = locked_connections.get_mut(&packet.receiver_port) {
            if let Err(e) = player_data.stream.write(&packet.data[..]) {
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
            }
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

fn connect(port: u16, address: String, player_name: String) {
    // Outgoing stream
    let mut stream = TcpStream::connect(address).unwrap();

    // Incomming stream
    let connections = Connections::new();

    handle_connections(connections.clone(), |connections, buffer| {
        //
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
