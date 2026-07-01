/*
 * ======================================================================================
 * PROYECTO: FLEET_OTA
 * MÓDULO: Telemetry Manager
 * ARCHIVO: fleet_manager/src/telemetry_manager.rs
 * DESCRIPCIÓN: Worker asíncrono para la ingesta de telemetría MQTT y persistencia
 * en base de datos PostgreSQL mediante persistencia relacional.
 *
 * Fix 2026-07-01 (paradigma):
 *   El INSERT ... ON CONFLICT anterior creaba una fila nueva en device_telemetry
 *   para CUALQUIER MAC que mandara un mensaje de telemetria, sin comprobar si
 *   esa MAC estaba autorizada en devices_registry. Esto era auto-registro
 *   colandose por otra tabla, contradiciendo el paradigma ya aplicado en el
 *   worker de inventario ("identidad conocida antes que confianza").
 *   Fix: se consulta devices_registry ANTES de persistir. Si la MAC no existe
 *   ahi, la telemetria se descarta sin escribir nada.
 * ======================================================================================
 */

use chrono::Local;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use sqlx::PgPool;
use std::time::Duration;
use shared_protocol::Telemetria;
use std::env;

pub async fn run_telemetry_worker(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Carga de configuración desde .env
    let broker_url = env::var("MQTT_BROKER_URL")
        .expect("MQTT_BROKER_URL no definido en .env");

    let mut mqttoptions = MqttOptions::new("fleet_manager_server", broker_url, 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(10));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    client.subscribe("flota/telemetria/#", QoS::AtLeastOnce).await?;

    println!("🚀 [FLEET_OTA] Worker de Telemetría activo...");

    // 2. Bucle de procesamiento con manejo de fallos
    loop {
        match eventloop.poll().await {
            Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish))) => {
                if let Ok(data) = serde_json::from_slice::<Telemetria>(&publish.payload) {

                    let mac_val = data.mac_address.as_str();
                    let status_val = data.status.as_str();
                    let version_val = data.version.as_str();

                    // Paradigma: identidad conocida antes que confianza.
                    // Solo se persiste telemetria de MACs ya autorizadas en
                    // devices_registry. Sin auto-registro en ninguna tabla.
                    let autorizado = sqlx::query!(
                        "SELECT 1 as ok FROM devices_registry WHERE mac = $1",
                        mac_val
                    )
                    .fetch_optional(&pool)
                    .await;

                    match autorizado {
                        Ok(Some(_)) => {
                            // 3. Persistencia con integridad relacional
                            let result = sqlx::query!(
                                r#"
                                INSERT INTO device_telemetry (mac_address, status, version, last_seen) 
                                VALUES ($1::TEXT, $2::TEXT, $3::TEXT, NOW())
                                ON CONFLICT (mac_address) 
                                DO UPDATE SET 
                                    status = EXCLUDED.status,
                                    version = EXCLUDED.version,
                                    last_seen = NOW()
                                "#,
                                mac_val,
                                status_val,
                                version_val
                            )
                            .execute(&pool)
                            .await;

                            if let Err(e) = result {
                                error_log(&data.mac_address, &e.to_string());
                            } else {
                                success_log(&data.mac_address, &data.version);
                            }
                        }
                        Ok(None) => {
                            rechazo_log(mac_val);
                        }
                        Err(e) => {
                            error_log(&data.mac_address, &e.to_string());
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("⚠️ [FLEET_OTA] Error MQTT: {}. Reconectando en 5s...", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            _ => {}
        }
    }
}

fn success_log(mac: &str, ver: &str) {
    println!("✅ [{}] Telemetría registrada: {} | v{}", Local::now().format("%H:%M:%S"), mac, ver);
}

fn error_log(mac: &str, err: &str) {
    eprintln!("❌ [{}] Error BD para {}: {}", Local::now().format("%H:%M:%S"), mac, err);
}

fn rechazo_log(mac: &str) {
    eprintln!("🚫 [{}] MAC no autorizada, telemetría descartada: {}", Local::now().format("%H:%M:%S"), mac);
}