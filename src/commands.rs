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
    #[command(arg_required_else_help = true)]
    Connect {
        /// Local port to be used as a connection point for the incoming packets.
        /// Note, each client connecting to the host must use a unique port!
        port: u16,
        /// Address of the udp socket opened on this port to relay packets.
        udp_port: u16,
        /// Adress of the host
        address: String,
        player_name: String,
        other_player_name: String,
        other_player_port: u16,
    },

    /// Pings a tcp socket at a given address from a given port.
    #[command(arg_required_else_help = true)]
    Ping { address: String, socket: SocketType },

    /// Listens on a port
    #[command(arg_required_else_help = true)]
    Listen { port: u16, socket: SocketType },

    /// Sends a command to the server
    #[command(arg_required_else_help = true)]
    Command { address: String, command: String },
}
