/* ============================================================================
 * Archivo: fleet_manager/src/ota_orchestrator.rs
 * Proyecto: FLEET_OTA
 * Descripción: Módulo encargado del disparo y orquestación de comandos OTA
 * hacia nodos específicos mediante el protocolo MQTT.
 * Autor: Sistema de Gestión de Flotas FLEET_OTA
 * ============================================================================
 */

use rumqttc::{AsyncClient, QoS};
use serde_json::json;
use chrono::Utc;

pub async fn trigger_ota_update(
    client: &AsyncClient, 
    mac: &str, 
    download_url: &str, 
    target_version: &str
) -> Result<(), Box<dyn std::error::Error>> {
    
    let transaction_id = format!("ota-{}", Utc::now().timestamp());

    let cmd = json!({
        "transaction_id": transaction_id,
        "target_version": target_version,
        "min_version_required": "0.1.0",
        "download_url": download_url,
        "checksum": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "signature": "0000000000000000000000000000000000000000000000000000000000000000",
        "size": 1500000
    });

    let topic = format!("flota/cmd/{}", mac);
    
    client.publish(topic.clone(), QoS::AtLeastOnce, false, cmd.to_string().into_bytes()).await?;
    
    println!("🚀 [OTA] Comando enviado correctamente a {}", mac);
    println!("   → Topic: {}", topic);
    println!("   → Versión: {}", target_version);
    
    Ok(())
}