// esp_ota_client/src/mqtt.rs
use esp_idf_svc::mqtt::client::{EspMqttClient, MqttClientConfiguration, QoS, EventPayload};
use esp_idf_svc::ota::EspOta;
use shared_protocol::{DeviceStatus, DeviceState, ProtocolString, OtaCommand};
use std::time::{Duration, Instant};
use log::{error, info};

pub fn mqtt_start_blocking(broker_url: &str, mac_address: String) {
    let mqtt_config = MqttClientConfiguration::default();
    
    let (mut client, mut connection) = match EspMqttClient::new(broker_url, &mqtt_config) {
        Ok(c) => c,
        Err(e) => {
            error!("Error inicializando MQTT: {:?}", e);
            return;
        }
    };

    let topic_status = format!("flota/status/{}", mac_address);
    let topic_cmd = format!("flota/cmd/{}", mac_address);

    loop {
        // 1. PUBLICACIÓN
        if let Ok(slot) = EspOta::new().unwrap().get_running_slot() {
            let label = slot.label;
            let mut active_partition = ProtocolString::<16>::new();
            let _ = active_partition.push_str(&label);
            
            let status = DeviceStatus {
                mac: ProtocolString::try_from(mac_address.as_str()).unwrap_or_default(),
                version: ProtocolString::try_from("1.0.0").unwrap_or_default(),
                active_partition,
                state: DeviceState::Idle,
                uptime_seconds: 0,
            };

            if let Ok(payload) = serde_json::to_string(&status) {
                let _ = client.publish(&topic_status, QoS::AtMostOnce, false, payload.as_bytes());
            }
        }

        // 2. VENTANA DE ESCUCHA (5 segundos)
        let _ = client.subscribe(&topic_cmd, QoS::AtLeastOnce);
        let start_time = Instant::now();
        
        while start_time.elapsed() < Duration::from_secs(5) {
            if let Ok(event) = connection.next() {
                // Desestructuramos la variante struct directamente como nos indicó el compilador
                if let EventPayload::Received { topic, data, .. } = event.payload() {
                    // topic es un Option<&str>, data es un &[u8]
                    if topic == Some(topic_cmd.as_str()) {
                        info!("📥 Comando OTA detectado");
                        // Usamos directamente 'data' que extrajimos arriba
                        if let Ok(cmd) = serde_json::from_slice::<OtaCommand>(data) {
                            info!("🚀 Ejecutando OTA: URL={}", cmd.download_url.as_str());
                            // AQUÍ SE INYECTARÁ LA DESCARGA
                        }
                    }
                }
            }
        }
        
        std::thread::sleep(Duration::from_secs(30)); 
    }
}