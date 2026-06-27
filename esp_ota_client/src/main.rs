// esp_ota_client/src/main.rs
use anyhow::Result;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{AuthMethod, ClientConfiguration, Configuration, EspWifi, WifiDeviceId};
use heapless::String;
use log::info;

mod mqtt;

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // ==================== WiFi ====================
    let mut wifi = EspWifi::new(peripherals.modem, sys_loop, Some(nvs))?;

    let mut wifi_config = Configuration::Client(ClientConfiguration::default());

    if let Configuration::Client(ref mut client) = wifi_config {
        // Corrección heapless::String
        client.ssid = create_heapless_string::<32>(env!("WIFI_SSID"));
        client.password = create_heapless_string::<64>(env!("WIFI_PASS"));
        client.auth_method = AuthMethod::WPA2Personal;
    }

    wifi.set_configuration(&wifi_config)?;
    wifi.start()?;
    wifi.connect()?;

    // Esperar conexión
    while !wifi.is_connected()? {
    std::thread::sleep(std::time::Duration::from_millis(500));
    }

    // Obtener MAC
    let mac = wifi.get_mac(WifiDeviceId::Sta)?;
    let device_mac = format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", 
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);

    info!("✅ WiFi conectado. MAC: {}", device_mac);

    // Iniciar MQTT
    mqtt::mqtt_start_blocking("mqtt://192.168.1.42:1883", device_mac);
    Ok(())
}

// Helper para crear heapless::String fácilmente
fn create_heapless_string<const N: usize>(s: &str) -> String<N> {
    let mut heapless_str: String<N> = String::new();
    let _ = heapless_str.push_str(s);
    heapless_str
}