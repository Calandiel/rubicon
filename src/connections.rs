use std::{
    collections::{
        hash_map::{Iter, IterMut},
        HashMap,
    },
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::{common::ToConnections, socket::SocketWrapper};

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

    pub fn get_target_stream<'a>(&'a mut self, tcp_port: u16) -> Option<&'a mut SocketWrapper> {
        if let Some(v) = self.by_tcp_port.get_mut(&tcp_port) {
            return Some(&mut v.stream);
        }
        None
    }

    /// Returns whether the operation was a success. If it wasn't, it means we're dealing with a duplicate name!
    pub fn update_player_name(&mut self, tcp_port: u16, new_name: String) -> bool {
        let entry = self
            .by_tcp_port
            .iter()
            .find(|(_, player)| player.name == new_name);
        if let Some(_) = entry {
            println!("DUPLICATE PLAYER NAME: {new_name}");
            return false;
        }

        if let Some(player) = self.by_tcp_port.get_mut(&tcp_port) {
            player.name = new_name;
        }

        true
    }

    pub fn get<'a>(&'a self, k: &u16) -> Option<&'a PlayerData> {
        self.by_tcp_port.get(k)
    }

    pub fn iter<'a>(&'a self) -> Iter<'a, u16, PlayerData> {
        self.by_tcp_port.iter()
    }

    pub fn iter_mut<'a>(&'a mut self) -> IterMut<'a, u16, PlayerData> {
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
