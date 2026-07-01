/* ============================================================================
 * Archivo: fleet_manager/src/main.rs
 * Proyecto: FLEET_OTA
 * Descripcion: Punto de entrada principal del Fleet Manager. Inicializa las
 * conexiones, levanta los workers asincronos y coordina el disparo
 * del comando de actualizacion hacia el nodo fisico activo.
 * Autor: Ivan Barra / Nekia Systems S.L.
 * Fecha: 2026-07-01
 * ----------------------------------------------------------------------------
 * Nota de configuracion (2026-06-30):
 *   La URL de descarga del firmware se lee de FIRMWARE_DISTRIBUTION_URL (.env),
 *   NO se hardcodea. Cambiar la IP/puerto del servidor de distribucion = editar
 *   una linea del .env + reiniciar; sin recompilar ni buscar literales dispersos.
 *   Hoy apunta al NkS-T del banco (ltc, 192.168.1.42:8080).
 *
 * Nota de paradigma (2026-07-01):
 *   El worker de inventario (paso 5) ya NO auto-registra dispositivos. La
 *   alta de un nodo en devices_registry es un acto manual (somnia-dev, al
 *   cargar el primer firmware). Aqui solo se VERIFICA la MAC via UPDATE; si
 *   no hay fila afectada, el anuncio se rechaza y no se persiste nada.
 *
 * Fix 2026-07-01 (proceso moria tras disparar OTA):
 *   Faltaba el bucle infinito final. main() llegaba a Ok(()) justo despues
 *   de publicar el comando OTA, el runtime de Tokio se apagaba, y mataba
 *   los workers de inventario y telemetria en segundo plano. Restaurado el
 *   loop de mantenimiento al final para que el proceso viva indefinidamente.
 * ============================================================================
 */

mod telemetry_handler;
mod ota_orchestrator;

use rumqttc::{AsyncClient, MqttOptions, Event, Incoming, QoS};
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Carga de configuracion del entorno de red
    dotenvy::dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL no definido en el entorno");
    let broker_url = env::var("MQTT_BROKER_URL")
        .expect("MQTT_BROKER_URL no definido en el entorno");
    // NUEVO: URL del servidor de distribucion, desde el .env (punto unico de verdad).
    let distribution_url = env::var("FIRMWARE_DISTRIBUTION_URL")
        .expect("FIRMWARE_DISTRIBUTION_URL no definido en el entorno");

    println!("⚙️ [FLEET_OTA] Inicializando Fleet Manager Central...");

    // 2. Conexion segura al pool de PostgreSQL
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    println!("✅ [FLEET_OTA] Conexion establecida con la base de datos.");

    // 3. Inicializacion del cliente MQTT
    let mut mqttoptions = MqttOptions::new("fleet_manager_core", broker_url.clone(), 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(10));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    // --- Suscripcion al topico de inventario ---
    client.subscribe("flota/directorio/anuncio", QoS::AtLeastOnce).await?;
    println!("📡 [FLEET_OTA] Suscrito al canal de anuncios. Esperando nodos...");

    // 4. Worker de Telemetria (escucha activa en segundo plano)
    let pool_clone_telemetry = pool.clone();
    tokio::spawn(async move {
        if let Err(e) = telemetry_handler::run_telemetry_worker(pool_clone_telemetry).await {
            eprintln!("❌ [FLEET_OTA] Error critico en Worker de Telemetria: {}", e);
        }
    });

    // 5. Orquestador de red y worker de inventario
    let pool_clone_inventory = pool.clone();
    let client_loop = client.clone();

    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    println!("📥 [DEBUG RED] Mensaje crudo detectado en topico: {}", p.topic);

                    if p.topic == "flota/directorio/anuncio" {
                        let payload = String::from_utf8_lossy(&p.payload);
                        println!("📦 [DEBUG RED] Payload recibido: {}", payload);

                        match serde_json::from_str::<shared_protocol::DeviceAnnouncement>(&payload) {
                            Ok(anuncio) => {
                                // Paradigma: identidad conocida antes que confianza.
                                // El nodo NUNCA se auto-registra. Solo se actualiza
                                // last_ip/last_seen si la MAC ya existe en
                                // devices_registry (alta manual previa en somnia-dev).
                                let res = sqlx::query!(
                                    "UPDATE devices_registry
                                     SET last_ip = $2, last_seen = NOW(), is_reachable = true
                                     WHERE mac = $1",
                                    anuncio.mac.as_str(),
                                    anuncio.ip.as_str()
                                ).execute(&pool_clone_inventory).await;

                                match res {
                                    Ok(result) if result.rows_affected() > 0 => {
                                        println!("📍 [INVENTARIO] Nodo autorizado, actualizado: {} (IP: {})", anuncio.mac, anuncio.ip)
                                    }
                                    Ok(_) => {
                                        // rows_affected == 0: MAC no existe en devices_registry.
                                        // No se persiste nada. Concepto "pending device" a futuro.
                                        eprintln!("🚫 [INVENTARIO] MAC no autorizada, rechazada: {} (IP: {})", anuncio.mac, anuncio.ip)
                                    }
                                    Err(e) => eprintln!("❌ [INVENTARIO] Error consultando BD: {}", e),
                                }
                            }
                            Err(e) => eprintln!("⚠️ [INVENTARIO] Mensaje descartado. No cumple el contrato estricto: {}", e),
                        }
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("⚠️ [FLEET_OTA] Friccion en bus MQTT: {}. Reintentando...", e);
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    });

    // 6. Ejecucion tecnica de la actualizacion
    let target_mac = "E8:3D:C1:F2:D9:C4";

    tokio::time::sleep(Duration::from_secs(8)).await;

    // Paradigma: identidad conocida antes que confianza. NO se dispara un
    // comando OTA a una MAC que no este autorizada en devices_registry, ni
    // aunque esa MAC este hardcodeada aqui (defensa en profundidad: el
    // hardcode es temporal hasta la CLI de operador, pero la comprobacion
    // debe existir igual).
    let autorizado = sqlx::query!(
        "SELECT mac FROM devices_registry WHERE mac = $1 AND is_reachable = true",
        target_mac
    )
    .fetch_optional(&pool)
    .await?;

    if autorizado.is_none() {
        eprintln!("🚫 [FLEET_OTA] Nodo {} no autorizado o no alcanzable. Abortando disparo OTA.", target_mac);
    } else {
        // La URL se construye desde el .env (NO hardcodeada).
        let download_url = format!("{}/firmware_v2.bin", distribution_url);

        println!("🎯 [FLEET_OTA] Iniciando distribucion a nodo objetivo: {}", target_mac);
        println!("   → URL descarga: {}", download_url);

        if let Err(e) = ota_orchestrator::trigger_ota_update(
            &client_loop,
            target_mac,
            &download_url,
            "1.0.1"
        ).await {
            eprintln!("❌ Error al disparar OTA: {}", e);
        } else {
            println!("✅ Comando OTA publicado correctamente hacia {}", target_mac);
        }
    }

    // Mantiene vivo el proceso indefinidamente. Sin esto, main() retorna,
    // Tokio apaga el runtime, y mueren los workers de inventario/telemetria
    // en segundo plano (paso 4 y 5) aunque el disparo OTA haya funcionado.
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}