use std::{collections::HashSet, io::Read};

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
    pub receiver_name: String,
    pub receiver_port: u16,
    pub data: Vec<u8>,
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
    peers: &HashSet<u16>,
    packets: &mut Vec<(u16, DataPacket)>,
    disconnected: &mut Vec<u16>,
    commands: &mut Vec<String>,
    greetings: &mut Vec<(u16, GreetingPacket)>,
    buffer: &mut [u8],
    rejected_packets_buffers: &mut Vec<Vec<u8>>,
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
                            println!("Recognized a data packet");
                            packets.push((*port, data));
                        }
                        Packet::NetworkTopology(_) => todo!(),
                        Packet::Command(command) => {
                            commands.push(command.command);
                        }
                        Packet::Greeting(greeting) => greetings.push((*port, greeting.clone())),
                    }
                } else {
                    println!("Failed to decode the packet.");
                    rejected_packets_buffers.push(sliced_data.to_vec());
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
