use clap::{command, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(name = "rubicon")]
#[command(about = "A software router for network packets", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Serialize, Deserialize, ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum SocketType {
    Udp,
    Tcp,
}
impl Default for SocketType {
    fn default() -> Self {
        Self::Tcp
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Requires an open outgoing port.
    #[command(arg_required_else_help = true)]
    Host {
        /// The outgoing port for clients to connect to
        port: u16,
    },

    /// Connects to a given host and routes data it receives from other programs into it.
    /// Example use:
    /// `rubicon connect 636.35.0.24:7777 Player 7000 Host 8000`
    /// This will use a server hosted at 636.35.0.24:7777,
    /// registering ourselves as `Player` with a port 7000, and sending data to `Host:8000.
    /// Do note than the local game would then attempt to connect to 127.0.0.1:7000
    /// DO NOT use "localhost" when connecting in games.
    /// Prefer 127.0.0.1, to let udp traffic through.
    #[command(arg_required_else_help = true)]
    Connect {
        /// Adress of the host
        server_address: String,
        /// Name of the player, used as an identifier
        player_name: String,
        /// Local port to be used as a connection point for the incoming packets.
        /// rubicon may bind tcp/udp sockets on this port so make sure it's free.
        player_port: u16,
        /// Name of the other player to connect to.
        other_player_name: String,
        /// The port on their machine to route the traffic to.
        other_player_port: u16,
    },

    #[command(arg_required_else_help = true)]
    MultiConnect {
        /// Adress of the host
        server_address: String,
        /// Name of the player, used as an identifier
        player_name: String,
        /// Name of the other player to connect to.
        other_player_name: String,
        /// Local port to be used as a connection point for the incoming packets.
        /// rubicon may bind tcp/udp sockets on this port so make sure it's free.
        player_ports: Vec<u16>,
    },

    #[command(arg_required_else_help = true)]
    MassConnect {
        /// Adress of the host
        server_address: String,
        /// Name of the player, used as an identifier
        player_name: String,
        /// Name of the other player to connect to.
        other_player_name: String,
        /// Local port to be used as a connection point for the incoming packets.
        /// rubicon may bind tcp/udp sockets on this port so make sure it's free.
        lower_port_inclusive: u16,
        upper_port_inclusive: u16,
    },

    /// Pings a tcp socket at a given address from a given port.
    #[command(arg_required_else_help = true)]
    Ping {
        port: u16,
        address: String,
        socket: SocketType,
        data_size: usize,
    },

    /// Listens on a port
    #[command(arg_required_else_help = true)]
    Listen { port: u16, socket: SocketType },

    /// Sends a command to the server
    #[command(arg_required_else_help = true)]
    Command { address: String, command: String },
}
