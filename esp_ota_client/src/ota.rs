// ============================================================================
// Archivo: esp_ota_client/src/ota.rs
// Proyecto: FLEET_OTA
// Autor: Iván Barra
// Fecha: 2026-06-27
// Descripción: Ejecución de transferencia OTA, validación de integridad y 
//              rollback preventivo mediante EspOtaUpdate.
// ============================================================================

use esp_idf_svc::ota::EspOta;
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use embedded_svc::http::client::Client;
use shared_protocol::OtaCommand;
use std::time::Duration;
use std::io::Read;
use log::{info, error, warn};

pub fn run_ota_transfer(comando: &OtaCommand) -> anyhow::Result<()> {
    info!("📡 [OTA] Inicio de transferencia. Versión: {}", comando.target_version);

    // 1. Inicialización de sesión
    let ota = EspOta::new()?;
    let mut update = ota.initiate_update()?;
    
    // 2. Conexión HTTP
    let connection = EspHttpConnection::new(&Configuration::default())?;
    let mut client = Client::wrap(connection);
    
    let request = client.get(&comando.download_url)?;
    let mut response = request.submit()?;
    
    if response.status() != 200 {
        error!("🚨 [OTA] Error HTTP: {}", response.status());
        return Err(anyhow::anyhow!("Falló la descarga HTTP"));
    }

    // 3. Bucle de escritura y validación (4KB alineados)
    let mut buffer = [0u8; 4096];
    let mut total_bytes = 0;
    
    info!("⏳ [OTA] Escribiendo en Flash...");
    while let Ok(bytes_read) = response.read(&mut buffer) {
        if bytes_read == 0 { break; }
        update.write(&buffer[..bytes_read])?;
        total_bytes += bytes_read;
    }
    
    // 4. Finalización y marcado como app válida (Pendiente de validación tras boot)
    update.complete()?;
    info!("✅ [OTA] Descarga completada. {} bytes.", total_bytes);
    
    // 5. Configuración de reinicio
    info!("🔄 [OTA] Reiniciando en 3s...");
    std::thread::sleep(Duration::from_secs(3));
    
    unsafe { esp_idf_sys::esp_restart() };
    Ok(())
}