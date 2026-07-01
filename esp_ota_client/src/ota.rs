// ============================================================================
// Archivo: esp_ota_client/src/ota.rs
// Proyecto: FLEET_OTA
// Módulo: OTA Engine (FSM) - Versión Auditable
// Fecha: 2026-06-30
// ============================================================================

use crate::hal::OtaFlash;
use shared_protocol::OtaCommand;
use log::{info, error};

#[derive(Debug)]
pub enum OtaError {
    VersionRejected,
    PartitionReadFailed,
    EraseFailed,
    WriteFailed,
    IntegrityFailed,
    SwitchFailed,
    #[allow(dead_code)]
    DownloadFailed,
}

pub struct OtaEngine<F: OtaFlash> {
    flash: F,
    current_version: &'static str,
}

impl<F: OtaFlash> OtaEngine<F> {
    pub fn new(flash: F, current_version: &'static str) -> Self {
        Self { flash, current_version }
    }

    pub fn execute_update(&self, cmd: &OtaCommand) -> Result<(), OtaError> {
        info!("[OTA] Iniciando actualización a v{}", cmd.target_version);

        if !self.is_version_safe(&cmd.min_version_required) {
            error!("[OTA] Versión rechazada. Mínima requerida: {}", cmd.min_version_required);
            return Err(OtaError::VersionRejected);
        }

        let current_slot = self.flash.get_active_partition()
            .map_err(|_| OtaError::PartitionReadFailed)?;

        let target_slot = if current_slot == "ota_0" { "ota_1" } else { "ota_0" };

        info!("[OTA] Slot actual: {} → Destino: {}", current_slot, target_slot);

        // Preparación
        self.flash.erase_partition(target_slot)
            .map_err(|_| OtaError::EraseFailed)?;

        // TODO: Descarga real HTTP aquí

        // Simulación temporal
        info!("[OTA] Escribiendo firmware (simulado)...");
        let dummy = [0u8; 1024];
        self.flash.write_chunk(target_slot, 0, &dummy)
            .map_err(|_| OtaError::WriteFailed)?;

        // Verificación
        if !self.flash.verify_integrity(target_slot, cmd.checksum.as_bytes())
            .map_err(|_| OtaError::IntegrityFailed)? {
            return Err(OtaError::IntegrityFailed);
        }

        // Commit
        self.flash.switch_to_partition(target_slot)
            .map_err(|_| OtaError::SwitchFailed)?;

        info!("[OTA] Actualización completada. Reinicio pendiente.");
        Ok(())
    }

    fn is_version_safe(&self, min_version: &str) -> bool {
        self.current_version >= min_version
    }
}