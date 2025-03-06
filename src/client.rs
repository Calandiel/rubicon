use std::{
    collections::HashMap,
    net::{TcpStream, UdpSocket},
};

use crate::{
    common::{print_connections, ToConnections, DISABLE_NAGLE_ALGORITHM},
    connections::Connections,
    packet::DataPacket,
};

pub struct ClientLocalConnection {
    pub player_name: String,
    pub port: u16,
    pub original_socket_port: u16,
    pub stream: Option<TcpStream>,
    pub udp_socket: Option<UdpSocket>,
}

pub struct ClientState {
    pub connections: Connections,
    pub player_name: String,
    pub player_port: u16,
    pub other_player_name: String,
    pub other_player_port: u16,

    /// A hashset mapping identifiers to their local connections
    pub local_redirection_table: HashMap<String, ClientLocalConnection>,
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
            local_redirection_table: Default::default(),
        }
    }

    pub fn print_connections(&self) {
        print_connections(&self.connections);
    }

    /// Check if the client is a game host (if so, player name == other player name)
    pub fn is_host(&self) -> bool {
        self.player_name == self.other_player_name
    }

    /// Given a data packet, creates the necessary local client connection with a tcp port present.
    pub fn ensure_tcp_socket_on_redirection_table(&mut self, data: &DataPacket) {
        if let Some(connection) = self
            .local_redirection_table
            .get_mut(&data.get_original_player_identifier())
        {
            if connection.stream.is_none() {
                // There's no tcp stream but the local connection exists
                // Communication probably started with udp?
            }
        } else if let Some(local_connection) =
            Self::get_local_tcp_socket_for_redirection_table(data)
        {
            // There's no  local connection exists
            self.local_redirection_table.insert(
                data.get_original_player_identifier(),
                self.get_local_connection_for_redirection_table(data, local_connection),
            );
        }
    }

    fn get_local_tcp_socket_for_redirection_table(data: &DataPacket) -> Option<TcpStream> {
        let tcp_socket_addr = format!("127.0.0.1:{}", data.receiver_port);
        if let Ok(tcp_socket) = TcpStream::connect(tcp_socket_addr.clone()) {
            tcp_socket.set_nodelay(DISABLE_NAGLE_ALGORITHM).unwrap();
            tcp_socket.set_nonblocking(true).unwrap();
            return Some(tcp_socket);
        }
        None
    }

    fn get_local_connection_for_redirection_table(
        &self,
        data: &DataPacket,
        tcp_socket: TcpStream,
    ) -> ClientLocalConnection {
        ClientLocalConnection {
            player_name: data.sender_name.clone(),
            port: data.sender_port,
            original_socket_port: data.source_port,
            stream: Some(tcp_socket),
            udp_socket: None,
        }
    }
}
impl ToConnections for ClientState {
    fn to_connections(&mut self) -> &mut Connections {
        &mut self.connections
    }
}
