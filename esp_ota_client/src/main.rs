// ============================================================================
// Archivo: esp_ota_client/src/main.rs
// Proyecto: FLEET_OTA
// Autor: Ivan Barra <ibn963@proton.me> / Nekia Systems S.L.
// Fecha: 2026-06-30
// ----------------------------------------------------------------------------
// Proposito:
//   Orquestador principal del nodo NkS. Secuencia: WiFi -> IP -> anuncio de
//   registro (handshake) -> cliente MQTT de comandos -> motor OTA -> bucle.
//
// Cambios 2026-06-30:
//   - Anade publicar_anuncio(): publica UNA vez un DeviceAnnouncement a
//     flota/directorio/anuncio. El fleet_manager lo inserta en devices_registry
//     (PostgreSQL). Es el alta automatica del nodo en la flota.
//   - El cliente de anuncio usa client_id propio ("fnk-ann-{MAC}") para no
//     colisionar con el cliente de comandos de mqtt.rs ("fnk-cmd-{MAC}").
//   - Se usa ip_str (antes se calculaba y descartaba).
//
// Dependencias: esp-idf-svc/hal, shared_protocol, serde_json, anyhow, log, heapless
// ============================================================================

use anyhow::Result;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{AuthMethod, ClientConfiguration, Configuration, EspWifi, WifiDeviceId};
use esp_idf_svc::mqtt::client::{EspMqttClient, MqttClientConfiguration, QoS};
use log::{info, error};
use std::thread;
use std::time::Duration;

mod hal;
mod ota;
mod mqtt;

use crate::hal::Esp32Flash;
use crate::ota::OtaEngine;
use shared_protocol::{OtaCommand, DeviceAnnouncement, ProtocolString};

const MQTT_BROKER: &str = env!("MQTT_BROKER_URL_LOCAL_HOST");

// ============================================================================
// PUNTO DE ENTRADA
// ============================================================================
fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let handle = thread::Builder::new()
        .stack_size(32768)
        .spawn(move || {
            if let Err(e) = run_app() {
                error!("🚨 Error critico en la tarea de aplicacion: {:?}", e);
            }
        })?;

    handle.join().unwrap();
    Ok(())
}

// ============================================================================
// LOGICA PRINCIPAL
// ============================================================================
fn run_app() -> Result<()> {
    info!("🚀 Iniciando nodo FLEET_OTA (Tarea aislada)...");

    // 1. WiFi
    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = EspWifi::new(peripherals.modem, sys_loop, Some(nvs))?;
    configure_wifi(&mut wifi)?;
    wait_for_ip(&mut wifi)?;

    let device_mac = get_device_mac(&mut wifi)?;
    let ip_info = wifi.sta_netif().get_ip_info()?;
    let ip_str = format!("{}", ip_info.ip);

    info!("✅ Sistema en linea. MAC: {} | IP: {}", device_mac, ip_str);

    let broker_url = format!("mqtt://{}:1883", MQTT_BROKER);

    // 2. ANUNCIO DE REGISTRO (llena devices_registry en PostgreSQL)
    if let Err(e) = publicar_anuncio(&broker_url, &device_mac, &ip_str) {
        error!("⚠️ [ANUNCIO] No se pudo publicar el anuncio: {:?}", e);
    }

    // 3. Canal MQTT -> OTA + cliente de comandos
    //    El hilo MQTT recibe comandos y los envia por el canal; este hilo
    //    (run_app) los recibe y ejecuta el motor OTA. Patron mpsc, como el
    //    encoder de SOMNIA.
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<OtaCommand>();
    info!("🔌 Intentando conectar MQTT a: {}", broker_url);
    mqtt::start_mqtt_client(&broker_url, &device_mac, cmd_tx)?;
    info!("⚙️ Servicio MQTT corriendo.");

    // 4. Motor OTA
    const FIRMWARE_VERSION: &str = env!("CARGO_PKG_VERSION");
    let ota_engine = OtaEngine::new(Esp32Flash, FIRMWARE_VERSION);
    info!("✅ Motor OTA listo. Version: {}", FIRMWARE_VERSION);

    // 5. Bucle principal: espera comandos OTA y ejecuta el motor.
    info!("🔄 Sistema operativo. Esperando comandos OTA...");
    loop {
        match cmd_rx.recv() {
            Ok(cmd) => {
                info!("🎯 [OTA] Ejecutando actualizacion a v{}", cmd.target_version);
                match ota_engine.execute_update(&cmd) {
                    Ok(_)  => info!("✅ [OTA] Ciclo completado (simulado)."),
                    Err(e) => error!("🚨 [OTA] Fallo: {:?}", e),
                }
            }
            Err(e) => {
                error!("[OTA] Canal de comandos cerrado: {:?}. Saliendo.", e);
                break;
            }
        }
    }
    Ok(())
}

// ============================================================================
// FUNCION: publicar_anuncio
// Publica UNA vez el DeviceAnnouncement al topic flota/directorio/anuncio.
// Cliente MQTT efimero con client_id propio ("fnk-ann-{MAC}") para no
// colisionar con el cliente de comandos. ip es ProtocolString<15>: se valida
// la longitud en el try_from (una IPv4 cabe: max "255.255.255.255" = 15).
// ============================================================================
fn publicar_anuncio(broker_url: &str, mac: &str, ip: &str) -> Result<()> {
    let client_id = format!("fnk-ann-{}", mac);

    let config = MqttClientConfiguration {
        client_id: Some(client_id.as_str()),
        ..Default::default()
    };

    let (mut client, mut connection) = EspMqttClient::new(broker_url, &config)?;

    // Bombear la conexion en un hilo breve para procesar el CONNACK.
    thread::Builder::new()
        .name("ann".into())
        .stack_size(4096)
        .spawn(move || {
            for _ in 0..10 {
                if connection.next().is_err() {
                    break;
                }
            }
        })?;

    // Construir el anuncio respetando los limites de tamano del protocolo.
    let anuncio = DeviceAnnouncement {
        mac: ProtocolString::<17>::try_from(mac)
            .map_err(|_| anyhow::anyhow!("MAC excede 17 caracteres"))?,
        ip: ProtocolString::<15>::try_from(ip)
            .map_err(|_| anyhow::anyhow!("IP excede 15 caracteres"))?,
        status: ProtocolString::<16>::try_from("ONLINE")
            .map_err(|_| anyhow::anyhow!("status excede 16 caracteres"))?,
    };

    let payload = serde_json::to_string(&anuncio)?;

    // Espera breve para asegurar conexion antes de publicar.
    thread::sleep(Duration::from_millis(1500));

    client.publish(
        "flota/directorio/anuncio",
        QoS::AtLeastOnce,
        true,   // retain=true: el broker guarda este ultimo anuncio y lo
                // entrega automaticamente a cualquier suscriptor que se
                // conecte despues (p.ej. fleet_manager arrancando tarde).
                // Antes en false, causaba perdida silenciosa del anuncio
                // si fleet_manager no estaba ya suscrito en ese instante.
        payload.as_bytes(),
    )?;

    info!("📍 [ANUNCIO] Publicado: {}", payload);
    Ok(())
}

// ============================================================================
// FUNCIONES DE SOPORTE
// ============================================================================

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

fn wait_for_ip(wifi: &mut EspWifi<'static>) -> Result<()> {
    info!("📡 Esperando Wi-Fi...");
    while !wifi.is_connected()? {
        thread::sleep(Duration::from_millis(500));
    }

    info!("⏳ Esperando IP...");
    loop {
        let ip_info = wifi.sta_netif().get_ip_info()?;
        if !ip_info.ip.is_unspecified() {
            info!("✅ IP: {}", ip_info.ip);
            break;
        }
        thread::sleep(Duration::from_millis(500));
    }
    Ok(())
}

fn get_device_mac(wifi: &mut EspWifi<'static>) -> Result<String> {
    let mac = wifi.get_mac(WifiDeviceId::Sta)?;
    Ok(format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]))
}