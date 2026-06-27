// ============================================================================
// Archivo: esp_ota_client/src/mqtt.rs
// Proyecto: FLEET_OTA
// Autor: Iván Barra
// Fecha: 2026-06-27
// Descripción: Cliente MQTT con acceso directo a las variantes del evento.
// ============================================================================

use esp_idf_svc::mqtt::client::{EspMqttClient, MqttClientConfiguration};
use anyhow::Result;
use std::thread;
use std::time::Duration;
use log::{info, error};

pub fn start_mqtt_client(broker_url: &str, mac_address: &str) -> Result<EspMqttClient<'static>> {
    info!("Configurando cliente MQTT...");

    let config = MqttClientConfiguration {
        client_id: Some(mac_address),
        keep_alive_interval: Some(Duration::from_secs(15)),
        ..Default::default()
    };

    let (client, mut connection) = EspMqttClient::new(broker_url, &config)?;

    thread::spawn(move || {
        info!("Hilo de eventos MQTT iniciado.");
        
        while let Ok(event) = connection.next() {
        // Usamos el match desestructurando el struct variant correctamente
        match event.payload() {
            esp_idf_svc::mqtt::client::EventPayload::Connected(_connected_data) => {
                info!("✅ MQTT Conectado.");
            }
            esp_idf_svc::mqtt::client::EventPayload::Disconnected => {
                error!("⚠️ MQTT Desconectado.");
            }
            // Desestructuración correcta mediante campos nombrados
            esp_idf_svc::mqtt::client::EventPayload::Received { topic, data, .. } => {
                let topic_str = topic.unwrap_or("unknown");
                let data_str = std::str::from_utf8(data).unwrap_or("invalid utf8");
                info!("📥 Recibido topic [{}]: {}", topic_str, data_str);
            }
            _ => {}
        }
    }
        error!("🚨 El bucle MQTT ha finalizado.");
    });

    Ok(client)
}