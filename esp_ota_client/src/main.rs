// ============================================================================
// Archivo: esp_ota_client/src/main.rs
// Proyecto: FLEET_OTA
// Autor: Iván Barra
// Fecha: 2026-06-27
// Descripción: Orquestador principal modularizado y clasificado por áreas 
//              de responsabilidad para facilitar su mantenimiento.
// ============================================================================

use anyhow::Result;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{AuthMethod, ClientConfiguration, Configuration, EspWifi, WifiDeviceId};
use log::{info, error};
use std::thread;
use std::time::Duration;

mod mqtt; 


// ============================================================================
// SECCIÓN 1: ORQUESTADOR PRINCIPAL
// ============================================================================
fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    info!("🚀 Iniciando nodo FLEET_OTA...");

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // 1.2 Inicializamos el hardware de red en el main (su tiempo de vida será todo el programa)
    let mut wifi = EspWifi::new(peripherals.modem, sys_loop, Some(nvs))?;
    
    // Configuramos y arrancamos la red pasando una referencia
    configure_wifi(&mut wifi)?;
    
    // 1.3 Barrera de sincronización (Garantiza que haya ruta TCP/IP)
    wait_for_ip(&mut wifi)?;

    // 1.4 Identidad del dispositivo
    let device_mac = get_device_mac(&mut wifi)?;
    info!("✅ Sistema en línea. Identidad del nodo (MAC): {}", device_mac);

    // 1.5 Lanzamiento de Servicios Concurrentes
    // Asignamos el cliente a una variable para evitar que Rust lo destruya
    let _mqtt_client = match mqtt::start_mqtt_client("mqtt://192.168.1.42:1883", &device_mac) {
        Ok(client) => {
            info!("⚙️ Servicio MQTT corriendo en hilo de fondo.");
            client // Retornamos el cliente hacia la variable _mqtt_client
        },
        Err(e) => {
            error!("❌ Fallo al iniciar servicio MQTT: {:?}", e);
            return Err(e); // Abortamos si falla la red
        }
    };

    // 1.6 Bucle de Mantenimiento
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

// ============================================================================
// SECCIÓN 2: CONFIGURACIÓN DE RED (CAPA FÍSICA Y ENLACE)
// ============================================================================
// Ahora recibe una referencia mutable en lugar de tomar propiedad del hardware
fn configure_wifi(wifi: &mut EspWifi) -> Result<()> {
    let mut wifi_config = Configuration::Client(ClientConfiguration::default());

    if let Configuration::Client(ref mut client) = wifi_config {
        client.ssid = heapless::String::<32>::try_from(env!("WIFI_SSID")).unwrap();
        client.password = heapless::String::<64>::try_from(env!("WIFI_PASS")).unwrap();
        client.auth_method = AuthMethod::WPA2Personal;
    }

    wifi.set_configuration(&wifi_config)?;
    wifi.start()?;
    wifi.connect()?;
    
    Ok(())
}

// ============================================================================
// SECCIÓN 3: SINCRONIZACIÓN DHCP (CAPA DE RED / PREVENCIÓN DE ERRORES)
// ============================================================================
fn wait_for_ip(wifi: &mut EspWifi<'static>) -> Result<()> {
    info!("📡 Esperando asociación Wi-Fi...");
    while !wifi.is_connected()? {
        thread::sleep(Duration::from_millis(500));
    }

    info!("⏳ Esperando asignación de IP del servidor DHCP...");
    loop {
        let ip_info = wifi.sta_netif().get_ip_info()?;
        // Si la IP no es 0.0.0.0, el DHCP ha terminado
        if !ip_info.ip.is_unspecified() {
            info!("✅ Red asignada correctamente. IP: {}", ip_info.ip);
            break;
        }
        thread::sleep(Duration::from_millis(500));
    }
    Ok(())
}

// ============================================================================
// SECCIÓN 4: UTILIDADES Y METADATOS DEL DISPOSITIVO
// ============================================================================
fn get_device_mac(wifi: &mut EspWifi<'static>) -> Result<String> {
    let mac = wifi.get_mac(WifiDeviceId::Sta)?;
    Ok(format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", 
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]))
}