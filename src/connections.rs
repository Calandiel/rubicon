use std::{
    collections::HashMap,
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
