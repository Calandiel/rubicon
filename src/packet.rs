use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    commands::SocketType,
    server::{Connections, PublicPlayerData},
};

#[derive(Serialize, Deserialize)]
pub enum Packet {
    Command(CommandPacket),
    Data(DataPacket),
    Greeting(GreetingPacket),
    NetworkTopology(NetworkTopologyPacket),
}

#[derive(Serialize, Deserialize)]
pub struct CommandPacket {
    pub command: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DataPacket {
    pub socket_type: SocketType,
    pub sender_name: String,
    pub sender_port: u16,
    pub receiver_name: String,
    pub receiver_port: u16,
    pub data: Vec<u8>,
    pub source_port: u16,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GreetingPacket {
    pub player_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct NetworkTopologyPacket {
    pub players: Vec<PublicPlayerData>,
}

/// Processes incomming packets
/// TODO: replace these args with a single structure
pub fn process_packets(
    connections: &mut Connections,
    // peers: &HashSet<u16>,
    packets: &mut Vec<(u16, DataPacket)>,
    disconnected: &mut Vec<u16>,
    commands: &mut Vec<String>,
    greetings: &mut Vec<(u16, GreetingPacket)>,
    buffer: &mut [u8],
    rejected_packets_buffers: &mut Vec<(String, u16, Vec<u8>, u16)>, // ignored by servers
    is_host: bool,                                                   // ignored by servers
    default_receiver_name: String,
    default_receiver_port: u16,
    redirection_table: &HashMap<u16, (String, u16, u16)>,
) {
    let mut locked_connections = connections.data.lock().unwrap();
    for (port, player_data) in locked_connections.iter_mut() {
        // Check for disconnects
        if player_data.stream.is_timed_out() {
            println!("player timeout");
            disconnected.push(*port);
            continue;
        }

        if player_data.stream.has_tcp() {
            let tcp_address = player_data.stream.get_tcp_addr().unwrap();
            match player_data.stream.read(buffer) {
                Ok(value) => {
                    // println!(
                    // "Received data of size {} from {} ({}) while processing packets",
                    // value, player_data.address, player_data.name
                    // );
                    // Address of the socket we're receiving data from.

                    if value == 0 {
                        continue; // skip size 0 packets, lol
                    }
                    let sliced_data = &buffer[..value];
                    let deserialize = bincode::deserialize::<Packet>(sliced_data);
                    if let Ok(packet) = deserialize {
                        // Check if the received port exists!
                        match packet {
                            Packet::Data(data) => {
                                // println!("Recognized a data packet from {}:{} for {}:{}", data.sender_name, data.sender_port, data.receiver_name, data.receiver_port);
                                packets.push((*port, data));

                                // Hosts need to store the data
                                // if is_host && redirection_table.contains_key(&port) {
                                // redirection_table.insert(k, v)
                                // }
                            }
                            Packet::NetworkTopology(_) => todo!(),
                            Packet::Command(command) => {
                                commands.push(command.command);
                            }
                            Packet::Greeting(greeting) => greetings.push((*port, greeting.clone())),
                        }
                    } else {
                        // println!(
                        // "Failed to decode the packet. Data size: {}. Port: {}. Potentially a packed received locally, intended for transmission.",
                        // sliced_data.len(), player_data.address.port()
                        // );
                        if is_host {
                            // If we're a host, we need to resolve the address ourselves
                            // rejected_packets_buffers.push((default_receiver_name.clone(), default_receiver_port ,sliced_data.to_vec()));
                            if let Some((receiver_name, receiver_port, tcp_port)) =
                                redirection_table.get(port)
                            {
                                rejected_packets_buffers.push((
                                    receiver_name.clone(),
                                    *receiver_port,
                                    sliced_data.to_vec(),
                                    *tcp_port,
                                ));
                                // println!("Retrieved receiver name and port. Message scheduled for transmission.");
                            } else {
                                // println!("Retrieval of receivers name failed on port: {}", *port)
                            }
                        } else {
                            // If we're not a host, just target the default receiver.
                            // Also, use the tcp adress as the port
                            rejected_packets_buffers.push((
                                default_receiver_name.clone(),
                                default_receiver_port,
                                sliced_data.to_vec(),
                                tcp_address.port(),
                            ));
                        }
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
}
