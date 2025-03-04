use clap::{command, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "rubicon")]
#[command(about = "A software router for network packets", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
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
        /// Adress of the host
        address: String,
        player_name: String,
    },

    /// Pings a tcp socket at a given address from a given port.
    #[command(arg_required_else_help = true)]
    Ping { address: String },

    /// Listens on a port
    #[command(arg_required_else_help = true)]
    Listen { port: u16 },
}
