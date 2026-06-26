use esp_idf_svc::mqtt::client::{EspMqttClient, MqttClientConfiguration, QoS};
use esp_idf_svc::ota::EspOta;
use shared_protocol::{DeviceStatus, DeviceState, ProtocolString};
use std::time::Duration;

pub fn mqtt_start_blocking(broker_url: &str, mac_address: String) {
    let mqtt_config = MqttClientConfiguration::default();
    let (mut client, _connection) = EspMqttClient::new(broker_url, &mqtt_config).unwrap();
    
    let topic_status = format!("flota/status/{}", mac_address);
    let mut ota = EspOta::new().unwrap();

    loop {
        // 1. Declaramos ota y slot explícitamente para que existan
        if let Ok(slot) = ota.get_running_slot() {
            let label = slot.label; // Label es un string que podemos convertir
            
            // 2. Creamos el status con String estándar de Rust primero
            let mut active_partition = ProtocolString::<2>::new();
            // push_str devuelve un Result, lo ignoramos para este caso simple
            let _ = active_partition.push_str(&label);

            // Sustituye tu bloque de creación de 'status' por esto:
            let status = DeviceStatus {
                // Usamos try_from porque &str puede fallar al convertir
                mac: ProtocolString::try_from(mac_address.as_str()).unwrap(),
                version: ProtocolString::try_from("1.0.0").unwrap(),
                active_partition,
                state: DeviceState::Idle,
                uptime_seconds: 0,
            };
                        // 3. Serializamos
            if let Ok(payload) = serde_json::to_string(&status) {
                let _ = client.publish(&topic_status, QoS::AtMostOnce, false, payload.as_bytes());
            }
        }

        std::thread::sleep(Duration::from_secs(30));
    }
}