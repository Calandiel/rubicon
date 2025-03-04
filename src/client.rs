use crate::{
    common::{print_connections, ToConnections},
    server::Connections,
};

pub struct ClientState {
    pub connections: Connections,
    pub player_name: String,
    pub player_port: u16,
    pub other_player_name: String,
    pub other_player_port: u16,
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
