
---

# Manual Técnico de Operaciones y Depuración: FLEET_OTA (V2.0)

## 1. Referencia Rápida de Infraestructura

* **Servidor Central (Dracarys / Tailscale IP):** `100.112.66.105`
* **Servidor Local (Host BD / Tailscale IP):** `100.96.254.3`
* **Nodos:** ESP32-S3 (Topic base: `flota/status/{MAC}` y `flota/cmd/{MAC}`)

# Punto de Trabajo (Servidor de Desarrollo): IP 100.96.254.3/32.

 ***Este es el nodo donde reside el entorno de compilación, el broker MQTT local y el cliente de despliegue directo. Estamos trabajando físicamente sobre esta instancia.

 ***Servidor Central (Dracarys / Backend): IP 100.107.177.78.

 ***Este es el nodo que gestiona el repositorio maestro de binarios y donde se consolidan los datos a largo plazo.

---

## 2. Operaciones de Terminal: MQTT (Mosquitto)

Esta es la caja de herramientas principal para diagnosticar si la red o el código están fallando. Todos los comandos deben ejecutarse desde la terminal del Servidor Local o de Dracarys.

### A. Monitorización de Logs del Broker (Vital para detectar bloqueos)

Para ver en tiempo real quién se conecta, quién es expulsado por *timeout* y el estado del puente (bridge):

```bash
sudo tail -f /var/log/mosquitto/mosquitto.log

```

* **Qué buscar:** `New client connected from 192.168.1.X` (Éxito).
* **Qué temer:** `Client <ID> disconnected due to timeout` (Fallo en el hilo concurrente del ESP32).

### B. Comandos de Suscripción (Escucha / Sniffing)

Para espiar todo el tráfico que pasa por el broker (Telemetría, logs, estados):

**Escuchar TODO el tráfico en Dracarys:**

```bash
mosquitto_sub -h 100.112.66.105 -t "#" -v

```

**Escuchar la telemetría de una MAC específica en el servidor local:**

```bash
mosquitto_sub -h localhost -t "flota/status/E8:3D:C1:F2:D9:C4" -v

```

### C. Comandos de Publicación (Inyección de Comandos OTA)

Para simular al servidor de producción ordenando una actualización OTA a un nodo de escritorio, sin pasar por la base de datos:

**Inyectar comando OTA localmente (que cruzará el bridge hasta Dracarys y bajará al nodo):**

```bash
mosquitto_pub -h localhost -t "flota/cmd/E8:3D:C1:F2:D9:C4" -m '{"transaction_id": "manual-01", "download_url": "http://100.112.66.105:8080/firmware.bin", "checksum": "IGNORAR_POR_AHORA"}'

```

### D. Gestión del Servicio Mosquitto

Si modificas los archivos `/etc/mosquitto/conf.d/bridge.conf`, aplica los cambios con:

```bash
sudo systemctl restart mosquitto
sudo systemctl status mosquitto

```

---

## 3. Operaciones del Nodo ESP32 (Rust)

Comandos para el ingeniero que está manipulando el hardware físicamente.

**Compilar, Flashear y Monitorizar (Todo en uno):**
Asegúrate de estar en el directorio raíz del firmware (`esp_ota_client`):

```bash
cargo espflash flash --monitor

```

*Si la terminal se llena de basura o caracteres extraños, reinicia la placa físicamente (botón RST/EN).*

---

## 4. Árbol de Decisión para Depuración (Troubleshooting)

Si un ESP32 no se actualiza, el becario/ingeniero debe seguir ESTRICTAMENTE este orden:

1. **¿El nodo tiene WiFi?** * *Acción:* Revisar la salida de `cargo espflash flash --monitor`.
* *Esperado:* `sta ip: 192.168.1.X`. Si no, fallo de DHCP o credenciales WiFi.


2. **¿El nodo se conecta al Broker?**
* *Acción:* Observar `sudo tail -f /var/log/mosquitto/mosquitto.log` local.
* *Esperado:* Conexión estable (no *timeout* a los 30s). Si hay timeout, el hilo `std::thread::spawn` de MQTT en Rust está fallando o bloqueado.


3. **¿El Puente Tailscale está vivo?**
* *Acción:* Lanzar un `mosquitto_pub` de prueba desde el local y escuchar con `mosquitto_sub -t "#"` en Dracarys (`100.112.66.105`).
* *Esperado:* El mensaje aparece instantáneamente. Si no, revisar ACLs de Tailscale o la configuración del bridge.


4. **¿Dracarys está sirviendo el binario HTTP?**
* *Acción:* Desde cualquier PC en Tailscale, ejecutar: `curl -I http://100.112.66.105:8080/firmware.bin`
* *Esperado:* Un `HTTP/1.0 200 OK`. Si da `Connection Refused`, el servidor HTTP de Python/Rust en Dracarys está caído.


---
