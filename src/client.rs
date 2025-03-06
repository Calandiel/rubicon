use std::{collections::HashMap, net::TcpStream};

use crate::{
    common::{print_connections, ToConnections},
    connections::Connections,
};

pub struct ClientLocalConnection {
    pub player_name: String,
    pub port: u16,
    pub original_socket_port: u16,
    pub stream: TcpStream,
}

pub struct ClientState {
    pub connections: Connections,
    pub player_name: String,
    pub player_port: u16,
    pub other_player_name: String,
    pub other_player_port: u16,

    /// A hashset mapping played identifiers to their local connections
    pub player_redirection_table: HashMap<String, ClientLocalConnection>,
}
impl ClientState {
    pub fn new(
        player_name: String,
        player_port: u16,
        other_player_name: String,
        other_player_port: u16,
    ) -> Self {
        Self {
            connections: Connections::new(),
            player_name,
            player_port,
            other_player_name,
            other_player_port,
            player_redirection_table: Default::default(),
        }
    }

    pub fn print_connections(&self) {
        print_connections(&self.connections);
    }

    /// Check if the client is a game host (if so, player name == other player name)
    pub fn is_host(&self) -> bool {
        self.player_name == self.other_player_name
    }
}
impl ToConnections for ClientState {
    fn to_connections(&mut self) -> &mut Connections {
        &mut self.connections
    }
}
