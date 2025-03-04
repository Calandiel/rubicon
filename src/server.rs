use std::{
    collections::HashMap,
    net::{SocketAddr, TcpStream},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct PlayerData {
    pub address: SocketAddr,
    pub stream: TcpStream,
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
        println!("=== CONNECTIONS ===");
        for (_, player_data) in self.connections.data.lock().unwrap().iter() {
            println!("- {} @ {}", player_data.name, player_data.address);
        }
    }
}
