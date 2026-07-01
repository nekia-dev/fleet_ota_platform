// ============================================================================
// Archivo: esp_ota_client/src/mqtt.rs
// Proyecto: FLEET_OTA
// Modulo: Cliente MQTT (suscripcion dirigida por Connected + sesion limpia)
// Autor: Ivan Barra <ibn963@proton.me> / Nekia Systems S.L.
// Fecha: 2026-07-01
// ----------------------------------------------------------------------------
// Proposito:
//   Cliente MQTT del NkS. Suscripcion disparada por el evento Connected.
//
// Fix 2026-06-30 (colision de client_id):
//   El broker mostraba clientes fantasma (inactive/disconnected) con el mismo
//   client_id (la MAC). Al reconectar, el ESP32 colisionaba con su propio
//   fantasma y el broker lo expulsaba antes de completar la suscripcion.
//   Solucion: client_id estable + 'fnk-' prefijo, y clean_session=true.
//
// Fix 2026-07-01 (subscribe nunca sale a red — confirmado por captura Wireshark):
//   `client` y `connection` compartian el mismo hilo. Al llamar a
//   client.subscribe() dentro del propio bucle que bombea connection.next(),
//   el hilo se bloqueaba dentro de subscribe() sin volver a drenar la cola
//   interna del cliente -> el paquete SUBSCRIBE nunca llegaba a emitirse.
//   Sintoma observado: log se detenia justo tras "Conectado. Suscribiendo...",
//   sin error ni confirmacion; Wireshark confirmo ausencia total del paquete
//   Subscribe Command tras el CONNACK (que ademas tuvo que retransmitirse).
//   Solucion: separar en dos hilos. Hilo A bombea connection.next() sin
//   interrupcion. Hilo B es dueño exclusivo de `client` y solo emite
//   subscribe() cuando el Hilo A le señaliza via canal mpsc.
//
// Stack: 8 KB (pump) + 4 KB (cmd). Dependencias: esp-idf-svc 0.52.1, serde_json, shared_protocol, anyhow, log
// ============================================================================

use anyhow::Result;
use esp_idf_svc::mqtt::client::{EspMqttClient, MqttClientConfiguration, EventPayload, QoS};
use log::{error, info, warn};
use std::thread;
use std::time::Duration;
use std::sync::mpsc::{channel, Sender};

pub fn start_mqtt_client(
    broker_url: &str,
    mac_address: &str,
    cmd_tx: Sender<shared_protocol::OtaCommand>,
) -> Result<()> {
    let client_id = format!("fnk-cmd-{}", mac_address);
    let config = MqttClientConfiguration {
        client_id: Some(client_id.as_str()),
        keep_alive_interval: Some(Duration::from_secs(60)),
        ..Default::default()
    };
    let (mut client, mut connection) = EspMqttClient::new(broker_url, &config)?;
    let cmd_topic = format!("flota/cmd/{}", mac_address);

    // Canal interno: SOLO señaliza "toca suscribirse", nunca mueve `client`.
    let (sub_tx, sub_rx) = channel::<()>();

    // Hilo A — bombea la conexion. NUNCA llama a metodos de `client` aqui.
    thread::Builder::new()
        .name("mqtt-pump".into())
        .stack_size(8 * 1024)
        .spawn(move || {
            loop {
                match connection.next() {
                    Ok(event) => match event.payload() {
                        EventPayload::Connected(_) => {
                            info!("[MQTT] Conectado. Señalizando suscripcion.");
                            if sub_tx.send(()).is_err() {
                                error!("[MQTT] Hilo de comandos no disponible.");
                            }
                        }
                        EventPayload::Subscribed(id) => {
                            info!("[MQTT] Suscripcion confirmada (id={}).", id);
                        }
                        EventPayload::Received { topic, data, .. } => {
                            info!("[MQTT] === MENSAJE RECIBIDO ===");
                            info!("[MQTT] Topic: {:?}", topic);
                            info!("[MQTT] Payload: {}", String::from_utf8_lossy(data));
                            match serde_json::from_slice::<shared_protocol::OtaCommand>(data) {
                                Ok(cmd) => {
                                    info!("[OTA] Comando recibido: v{}", cmd.target_version);
                                    if let Err(e) = cmd_tx.send(cmd) {
                                        error!("[MQTT] No se pudo encolar el comando OTA: {:?}", e);
                                    }
                                }
                                Err(e) => error!("[MQTT] Payload no es OtaCommand valido: {:?}", e),
                            }
                        }
                        EventPayload::Disconnected => warn!("[MQTT] Desconectado. Reintentara solo."),
                        EventPayload::Error(e) => error!("[MQTT] Error de evento: {:?}", e),
                        _ => {}
                    },
                    Err(e) => {
                        error!("[MQTT] connection.next() error: {:?}. Continuando.", e);
                        thread::sleep(Duration::from_millis(500));
                    }
                }
            }
        })?;

    // Hilo B — dueño exclusivo de `client`. Solo emite subscribe(), nunca bombea conexion.
    thread::Builder::new()
        .name("mqtt-cmd".into())
        .stack_size(4 * 1024)
        .spawn(move || {
            while sub_rx.recv().is_ok() {
                match client.subscribe(&cmd_topic, QoS::AtLeastOnce) {
                    Ok(_) => info!("[MQTT] Comando subscribe emitido hacia {}", cmd_topic),
                    Err(e) => warn!("[MQTT] subscribe fallo: {:?}", e),
                }
            }
            warn!("[MQTT] Canal de señalizacion cerrado. Hilo de comandos terminando.");
        })?;

    Ok(())
}