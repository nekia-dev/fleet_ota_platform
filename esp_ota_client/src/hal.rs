/*
 * ======================================================================================
 * PROYECTO: FLEET_OTA
 * MÓDULO: Hardware Abstraction Layer (HAL) - MOCK MODE
 * ARCHIVO: esp_ota_client/src/hal.rs
 * * DESCRIPCIÓN:
 * Implementación "Dummy" para romper bucles de compilación y validar la FSM.
 * No interactúa con el hardware real temporalmente.
 * ======================================================================================
 */

use esp_idf_svc::ota::EspOta;

pub trait OtaFlash {
    fn get_active_partition(&self) -> Result<String, u32>;
    fn erase_partition(&self, label: &str) -> Result<(), u32>;
    fn write_chunk(&self, label: &str, offset: u32, data: &[u8]) -> Result<(), u32>;
    fn verify_integrity(&self, label: &str, expected_hash: &[u8]) -> Result<bool, u32>;
    fn switch_to_partition(&self, label: &str) -> Result<(), u32>;
}

pub struct Esp32Flash;

impl OtaFlash for Esp32Flash {
    // ==========================================================
    // 1. RESTAURADO: Conexión Real al Hardware ESP32
    // ==========================================================
    fn get_active_partition(&self) -> Result<String, u32> {
        // Obtenemos el controlador OTA
        let ota = EspOta::new().map_err(|_| 0x101_u32)?;
        
        // CORRECCIÓN: Recibimos el objeto 'Slot' y leemos su campo 'label'
        let slot = ota.get_running_slot().map_err(|_| 0x102_u32)?;
        
        Ok(slot.label.to_string())
    }

    // ==========================================================
    // MODO SIMULADOR (En espera para los siguientes cortes)
    // ==========================================================
    fn erase_partition(&self, _label: &str) -> Result<(), u32> { Ok(()) }
    fn write_chunk(&self, _label: &str, _offset: u32, _data: &[u8]) -> Result<(), u32> { Ok(()) }
    fn verify_integrity(&self, _label: &str, _expected_hash: &[u8]) -> Result<bool, u32> { Ok(true) }
    fn switch_to_partition(&self, _label: &str) -> Result<(), u32> { Ok(()) }
}