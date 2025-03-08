use std::{
    collections::{
        hash_map::{Iter, IterMut},
        HashMap,
    },
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::{common::ToConnections, packet::GreetingPacket, socket::SocketWrapper};

#[derive(Debug)]
pub struct PlayerData {
    /// Address of the tcp socket that we bound for communicating with the player
    pub address: SocketAddr,
    pub stream: SocketWrapper,
    pub name: String,
    /// Port the player uses itself, useful for sending udp packets to it!
    pub local_port: Option<u16>,
    pub last_known_udp_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicPlayerData {
    pub name: String,
}

#[derive(Debug, Default)]
pub struct InnerConnections {
    by_tcp_port: HashMap<u16, PlayerData>,
    // pub by_own
}
impl InnerConnections {
    // Breaks encapsulation...
    // pub fn get_mut<'a>(&'a mut self, k: &u16) -> Option<&'a mut PlayerData> {
    // self.by_tcp_port.get_mut(k)
    // }

    pub fn get_player_tcp_port_by_name(&self, name: &String) -> Option<u16> {
        if let Some((port, _)) = self
            .by_tcp_port
            .iter()
            .find(|(_, player)| &player.name == name)
        {
            return Some(*port);
        }
        None
    }

    pub fn get_player_udp_port_by_name_mut(&mut self, name: &String) -> Option<&mut u16> {
        if let Some(port) = self
            .by_tcp_port
            .iter_mut()
            .find(|(_, player)| &player.name == name)
            .map(|player| &mut player.1.last_known_udp_port)
        {
            return Some(port);
        }
        None
    }

    pub fn get_target_stream<'a>(&'a mut self, tcp_port: u16) -> Option<&'a mut SocketWrapper> {
        if let Some(v) = self.by_tcp_port.get_mut(&tcp_port) {
            return Some(&mut v.stream);
        }
        None
    }

    /// Returns whether the operation was a success. If it wasn't, it means we're dealing with a duplicate name!
    pub fn update_player_from_greeting(
        &mut self,
        tcp_port: u16,
        greeting: &GreetingPacket,
    ) -> bool {
        let entry = self
            .by_tcp_port
            .iter()
            .find(|(_, player)| player.name == greeting.player_name);
        if let Some(_) = entry {
            println!("DUPLICATE PLAYER NAME: {}", greeting.player_name);
            return false;
        }

        if let Some(player) = self.by_tcp_port.get_mut(&tcp_port) {
            println!("Updating port from greeting: {}", greeting.local_port);
            player.name = greeting.player_name.clone();
            player.local_port = Some(greeting.local_port);
        }

        true
    }

    pub fn get(&self, k: &u16) -> Option<&PlayerData> {
        self.by_tcp_port.get(k)
    }

    pub fn iter(&self) -> Iter<u16, PlayerData> {
        self.by_tcp_port.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<u16, PlayerData> {
        self.by_tcp_port.iter_mut()
    }

    // If the map did not have this key present, [None] is returned.
    pub fn insert(&mut self, key: u16, value: PlayerData) -> Option<PlayerData> {
        self.by_tcp_port.insert(key, value)
    }

    // If the map did not have this key present, [None] is returned.
    pub fn remove(&mut self, key: &u16) -> Option<PlayerData> {
        self.by_tcp_port.remove(key)
    }
}

#[derive(Clone, Debug)]
pub struct Connections {
    pub data: Arc<Mutex<InnerConnections>>,
}
impl Connections {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(InnerConnections::default())),
        }
    }
}
impl ToConnections for Connections {
    fn to_connections(&mut self) -> &mut Connections {
        self
    }
}
