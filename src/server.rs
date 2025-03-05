use crate::{
    common::{print_connections, ToConnections},
    connections::Connections,
    packet::GreetingPacket,
};

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
            if let Some(_) = cons.get(&port) {
                println!("NEW PLAYER: {}:{}", greeting.player_name, port);
                if !cons.update_player_name(port, greeting.player_name) {
                    println!("Removing the impostor player...");
                    cons.remove(&port);
                }
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
