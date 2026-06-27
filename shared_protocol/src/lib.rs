// shared_protocol/src/lib.rs
use serde::{Deserialize, Serialize};

#[cfg(not(target_os = "espidf"))]
pub type ProtocolString<const N: usize> = String;

#[cfg(target_os = "espidf")]
pub type ProtocolString<const N: usize> = heapless::String<N>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum DeviceState {
    Idle,
    Downloading,
    WritingFlash,
    Verifying,
    Rebooting,
    Error, // Simplificado para ahorrar memoria en el enum
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceStatus {
    pub mac: ProtocolString<17>,
    pub version: ProtocolString<8>,
   pub active_partition: ProtocolString<16>,// "A" o "B"
    pub state: DeviceState,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OtaCommand {
    pub transaction_id: ProtocolString<32>,
    pub target_version: ProtocolString<8>,
    pub download_url: ProtocolString<128>,
    pub checksum: ProtocolString<64>, // Hash SHA-256 para validación industrial
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Telemetria {
    pub mac_address: ProtocolString<17>,
    pub status: ProtocolString<16>,
    pub version: ProtocolString<8>,
    pub timestamp: u64,
}