use std::{
    collections::{HashMap, HashSet},
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{Arc, Mutex},
    time::Duration,
};

use clap::{command, Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(name = "rubicon")]
#[command(about = "A software router for network packets", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
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
    },

    /// Pings a tcp socket at a given address from a given port.
    #[command(arg_required_else_help = true)]
    Ping { address: String },
}

#[derive(Serialize, Deserialize)]
struct Packet {
    receiver_port: u16,
    data: Vec<u8>,
}

#[derive(Clone, Debug)]
struct Connections {
    pub data: Arc<Mutex<HashMap<u16, (SocketAddr, TcpStream)>>>,
}
impl Connections {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(std::collections::HashMap::<
                u16,
                (SocketAddr, TcpStream),
            >::default())),
        }
    }
}

fn main() {
    let args = Args::parse();

    // println!("Hello, world!");
    // let udp_socket = UdpSocket::bind("127.0.0.1:34254").unwrap();

    match args.command {
        Commands::Host { port } => host(port),
        Commands::Connect { port, address } => connect(port, address),
        Commands::Ping { address } => ping(address),
    }
}

fn host(port: u16) {
    // Listener uwu
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();

    let connections = Connections::new();

    // process existing connections - we need to read the data from them and then pass it to the intended receiver
    let cons = connections.clone();
    std::thread::spawn(move || {
        let connections = cons;
        let mut buffer = [0u8; 1024 * 1024];
        loop {
            let mut connections = connections.data.lock().unwrap();
            let peers: HashSet<u16> = connections.iter().map(|(k, _)| *k).collect();
            let mut packets = vec![]; // Packets to pass
            let mut disconnected = vec![]; // Disconnected peers

            for (port, (peer, stream)) in connections.iter_mut() {
                // Check for disconnects
                if let Ok(size) = stream.peek(&mut buffer) {
                    if size == 0 {
                        // Potential timeout?
                        disconnected.push(port);
                        continue;
                    }
                }

                match stream.read(&mut buffer) {
                    Ok(value) => {
                        println!("Received data of size: {}", value);
                        let sliced_data = &buffer[..value];
                        if let Ok(packet) = bincode::deserialize::<Packet>(sliced_data) {
                            // Check if the received port exists!
                            if peers.contains(&packet.receiver_port) {
                                packets.push(packet);
                            }
                        } else {
                            println!("Failed to decode the packet.");
                        }
                    }
                    Err(e) => match e.kind() {
                        std::io::ErrorKind::WouldBlock => {
                            // dont print when not debugging - itll flood the console cuz most of the time there's nothing to read...
                            // println!("A stream ({:?}) would block upon reading: {:?}", peer, e)
                        }
                        _ => println!(
                            "A stream ({:?}) returned an error upon reading: {:?}",
                            peer, e
                        ),
                    },
                }
            }

            for packet in packets {
                if let Some((peer, stream)) = connections.get_mut(&packet.receiver_port) {
                    if let Err(e) = stream.write(&packet.data[..]) {
                        match e.kind() {
                            std::io::ErrorKind::WouldBlock => {
                                println!("A stream ({:?}) would block upon writing: {:?}", peer, e)
                            }
                            _ => println!(
                                "A stream ({:?}) returned an error upon writing: {:?}",
                                peer, e
                            ),
                        }
                    }
                }
            }

            // for disconnect in disconnected {
            // connections.remove(disconnect);
            // }
        }
    });

    accept_connections(&listener, connections);
}

fn connect(port: u16, address: String) {
    // Outgoing stream
    let mut stream = TcpStream::connect(address).unwrap();

    // Incomming stream
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
    let connections = Connections::new();

    let cons = connections.clone();
    std::thread::spawn(move || {
        let connections = cons;
        let mut buffer = [0u8; 1024 * 1024];
        loop {
            //
        }
    });

    accept_connections(&listener, connections);
}

fn accept_connections(listener: &TcpListener, connections: Connections) -> ! {
    loop {
        let stream = listener.accept();
        if let Ok((tcp_stream, peer)) = stream {
            tcp_stream.set_nonblocking(true).unwrap(); // TODO: remove this unwrap
            let mut connections = connections.data.lock().unwrap();
            connections.insert(peer.port(), (peer, tcp_stream));
        }
    }
}

fn ping(address: String) {
    // Outgoing stream
    let mut stream = TcpStream::connect(address).unwrap();
    stream.set_nonblocking(true).unwrap();

    loop {
        std::thread::sleep(Duration::from_millis(16));
        let _ = stream.write(&[1, 2, 3, 5, 7, 11, 13]);
    }
}
