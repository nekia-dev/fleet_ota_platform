/*
 * ======================================================================================
 * PROYECTO: Somnia III - Plataforma de Gestión de Flotas IoT
 * MÓDULO: Shared Protocol
 * ARCHIVO: shared_protocol/src/lib.rs
 * * CLASIFICACIÓN DE ERRORES:
 * - ERR_PROTO_001: Error de deserialización (JSON malformado)
 * - ERR_PROTO_002: Error de validación de integridad (SHA256 mismatch)
 * - ERR_PROTO_003: Error de capacidad de memoria (Buffer overflow en ProtocolString)
 * * DESCRIPCIÓN:
 * Contrato de comunicación unificado para el firmware ESP32 y el servidor Fleet Manager.
 * Utiliza estructuras de tamaño fijo para garantizar compatibilidad no_std.
 * ======================================================================================
 */

#![cfg_attr(not(test), no_std)]

use serde::{Deserialize, Serialize};

// Usamos heapless::String de forma universal para ESP32 y Servidor.
// Esto garantiza que el servidor NUNCA envíe un string que desborde la RAM del ESP32.
pub type ProtocolString<const N: usize> = heapless::String<N>;

// --- SECCIÓN: ESTRUCTURAS DE ESTADO Y TELEMETRÍA ---
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum DeviceState {
    Idle,
    Downloading,
    WritingFlash,
    Verifying,
    Rebooting,
    Error,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceStatus {
    pub mac: ProtocolString<17>,
    pub version: ProtocolString<8>,
    pub active_partition: ProtocolString<16>,
    pub state: DeviceState,
    pub uptime_seconds: u64,
}

// --- SECCIÓN: NUEVO - EL HANDSHAKE (ANUNCIO AL REGISTRO) ---
// Esta es la estructura que viaja por MQTT y que el servidor lee 
// para guardar en PostgreSQL (devices_registry).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceAnnouncement {
    pub mac: ProtocolString<17>,
    pub ip: ProtocolString<15>, // IPv4 ej: "192.168.1.100"
    pub status: ProtocolString<16>,
}

// --- SECCIÓN: CONTRATO DE MENSAJES OTA ---
#[derive(Debug, Serialize, Deserialize, Clone)]

pub struct OtaCommand {
    pub transaction_id: ProtocolString<32>,
    pub target_version: ProtocolString<8>,
    pub min_version_required: ProtocolString<8>, 
    pub download_url: ProtocolString<128>,
    pub checksum: ProtocolString<64>,
    pub signature: ProtocolString<128>,          
    pub size: u32, 
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Telemetria {
    pub mac_address: ProtocolString<17>,
    pub status: ProtocolString<16>,
    pub version: ProtocolString<8>,
    pub timestamp: u64,
}