use serde::{Deserialize, Serialize};

use crate::server::PublicPlayerData;

#[derive(Serialize, Deserialize)]
pub enum Packet {
    Data(DataPacket),
    NetworkTopology(NetworkTopologyPacket),
}

#[derive(Serialize, Deserialize)]
pub struct DataPacket {
    pub receiver_port: u16,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct NetworkTopologyPacket {
    pub players: Vec<PublicPlayerData>,
}
