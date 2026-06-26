use chrono::Local;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use sqlx::PgPool;
use std::time::Duration;
use shared_protocol::Telemetria;

pub async fn run_telemetry_worker(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let mut mqttoptions = MqttOptions::new("fleet_manager_server", "localhost", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    client.subscribe("flota/telemetria/#", QoS::AtLeastOnce).await?;

    println!("🚀 Worker de Telemetría activo y escuchando...");

    loop {
        let notification = eventloop.poll().await?;
        if let rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish)) = notification {
            if let Ok(data) = serde_json::from_slice::<Telemetria>(&publish.payload) {
                
                // Ejecución del Upsert con last_seen integrado
                let result = sqlx::query!(
                    r#"
                    INSERT INTO device_telemetry (mac_address, status, version, last_seen) 
                    VALUES ($1, $2, $3, NOW())
                    ON CONFLICT (mac_address) 
                    DO UPDATE SET 
                        status = EXCLUDED.status,
                        version = EXCLUDED.version,
                        last_seen = NOW()
                    "#,
                    data.mac_address.as_str(),
                    data.status.as_str(),
                    data.version.as_str()
                )
                .execute(&pool)
                .await;

                // Feedback centrado en el evento
                let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                match result {
                    Ok(_) => println!("✅ [{}] Telemetría registrada: {} | v{}", now, data.mac_address, data.version),
                    Err(e) => eprintln!("❌ [{}] Error crítico en BD para {}: {}", now, data.mac_address, e),
                }
            }
        }
    }
}