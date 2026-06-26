use esp_idf_svc::ota::EspOta;
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use embedded_svc::http::client::Client;
// Importamos nuestro contrato genérico desde la librería compartida
use shared_protocol::OtaCommand;
use std::time::Duration;
use std::io::Read;
/// Ejecuta la transferencia de un binario vía HTTP basado en el comando del servidor.
/// El buffer de 4KB asegura la alineación necesaria para el driver de Flash.
pub fn run_ota_transfer(comando: &OtaCommand) -> anyhow::Result<()> {
    println!("📡 [OTA] Comando recibido. Preparando inyección de firmware...");
    println!("📦 [OTA] Versión objetivo: {}", comando.target_version);
    println!("🔗 [OTA] Descargando desde: {}", comando.download_url);

    // 1. Inicializar la sesión OTA y preparar la partición libre (ota_0 o ota_1)
    let mut ota = EspOta::new()?;
    let mut update = ota.initiate_update()?;
    
    // 2. Configurar la conexión HTTP
    let connection = EspHttpConnection::new(&Configuration::default())?;
    let mut client = Client::wrap(connection);
    
    // 3. Obtener el stream del binario
    let request = client.get(comando.download_url)?;
    let mut response = request.submit()?;
    
    println!("⏳ [OTA] Iniciando descarga. Escribiendo en memoria Flash...");
    
    // 4. Bucle de escritura en Flash
    let mut buffer = [0u8; 4096];
    let mut total_bytes = 0;
    
    // embedded_svc::io::Read o std::io::Read proveen el método read()
    while let Ok(bytes_read) = response.read(&mut buffer) {
        if bytes_read == 0 { break; } // Fin del archivo
        
        // Escribimos el fragmento en la partición inactiva de forma segura
        update.write(&buffer[..bytes_read])?;
        total_bytes += bytes_read;
    }
    
    println!("✅ [OTA] Descarga completada ({} bytes).", total_bytes);
    
    // 5. Finalización y sellado del Bootloader
    update.complete()?;
    
    println!("🔄 [OTA] ¡Actualización sellada! Reiniciando el nodo en 3 segundos...");
    
    // Pequeña pausa para asegurar que los logs se envían por puerto serie
    std::thread::sleep(Duration::from_secs(3));
    
    // Reinicio por hardware para arrancar en el nuevo banco de memoria
    unsafe { esp_idf_sys::esp_restart() };

    // Técnicamente inalcanzable, pero requerido por el tipado de Rust
    Ok(())
}