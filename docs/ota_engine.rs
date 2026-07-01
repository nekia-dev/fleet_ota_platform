// ============================================================================
// Archivo: esp_ota_client/src/ota_engine.rs
// Proyecto: FLEET_OTA
// Módulo: OTA Engine - Capa de Decisión y Seguridad
// Fecha: 2026-06-30
// ============================================================================

use crate::hal::OtaFlash;
use shared_protocol::OtaCommand;
use log::{info, error};

pub struct OtaEngine<F: OtaFlash> {
    flash: F,
    current_version: &'static str,
}

impl<F: OtaFlash> OtaEngine<F> {
    pub fn new(flash: F, current_version: &'static str) -> Self {
        Self { flash, current_version }
    }

    pub fn process_update(&self, cmd: &OtaCommand) -> Result<(), u32> {
        // 1. Validación de seguridad
        if !self.is_version_safe(&cmd.min_version_required) {
            error!("[OTA] Rechazo de versión. Actual: {}, Mínima: {}", 
                   self.current_version, cmd.min_version_required);
            return Err(99_u32);
        }

        info!("[OTA] Versión segura. Iniciando actualización a v{}", cmd.target_version);

        // 2. Delegación a lógica de flash
        // Aquí puedes llamar a métodos de self.flash directamente o a un motor interno

        // Por ahora simulamos
        info!("[OTA] Descarga y escritura simulada completada.");
        info!("[OTA] Actualización lista para reinicio.");

        Ok(())
    }

    fn is_version_safe(&self, min_required: &str) -> bool {
        self.current_version >= min_required
    }
}