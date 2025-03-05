use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::{
    common::{print_connections, ToConnections},
    packet::GreetingPacket,
    socket::SocketWrapper,
};

#[derive(Debug)]
pub struct PlayerData {
    pub address: SocketAddr,
    pub stream: SocketWrapper,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicPlayerData {
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct Connections {
    pub data: Arc<Mutex<HashMap<u16, PlayerData>>>,
}
impl Connections {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(
                std::collections::HashMap::<u16, PlayerData>::default(),
            )),
        }
    }
}
impl ToConnections for Connections {
    fn to_connections(&mut self) -> &mut Connections {
        self
    }
}

/// The server is responsible for the following operations:
/// - accepting new TCP connections from clients
/// - receiving packets from clients, along the lines of "connected a socket" / "closed a socket" / "transferred data"
/// - relaying all the relevant information to clients as it comes in
pub struct ServerState {
    pub connections: Connections,
}
impl ServerState {
    pub fn new() -> Self {
        Self {
            connections: Connections::new(),
        }
    }

    pub fn print_connections(&self) {
        print_connections(&self.connections);
    }

    /// Receives a vector of commands (strings) and executes them
    pub fn receive_commands(&mut self, commands: Vec<String>) {
        for command in commands {
            match command.as_str() {
                "show_connections" => {
                    self.print_connections();
                }
                _ => println!("Unknown command: {}", command),
            }
        }
    }

    pub fn receive_greetings(&mut self, greetings: Vec<(u16, GreetingPacket)>) {
        let mut cons = self.connections.data.lock().unwrap();
        for (port, greeting) in greetings {
            if let Some(player) = cons.get_mut(&port) {
                println!("NEW PLAYER: {}:{}", greeting.player_name, port);
                player.name = greeting.player_name;
            }
        }
    }
    // pub fn receive_data_packets(&mut self, packets: Vec<(u16, DataPacket)>) {

    // }
}
impl ToConnections for ServerState {
    fn to_connections(&mut self) -> &mut Connections {
        &mut self.connections
    }
}
