
---

# Guía de Onboarding y Arquitectura: Proyecto FLEET_OTA

## 1. Visión General del Proyecto

**FLEET_OTA** es un sistema integral de gestión y actualización remota de firmware (Over-The-Air) para flotas de dispositivos IoT. El objetivo del sistema es permitir que miles de placas distribuidas en diferentes redes locales se actualicen de forma autónoma, segura y sin colapsar el ancho de banda, utilizando un modelo de comunicación asíncrona.

## 2. Infraestructura y Ecosistema de Red

El sistema no expone servicios a la Internet pública. Todo el tráfico de control y despliegue está encapsulado dentro de una red privada virtual cifrada (VPN) gestionada por **Tailscale**, utilizando Listas de Control de Acceso (ACLs) para segmentar el tráfico.

### Nodos y Servidores Críticos:

* **Servidor de Base de Datos (Host):**
* **IP Tailscale:** `100.96.254.3`
* **Función:** Ejecuta PostgreSQL. Es la "fuente de verdad". Almacena el inventario de placas (MAC address), perfiles de hardware y versiones de firmware requeridas.


* **Servidor Central "Dracarys" (Backend / Servidor de Descargas):**
* **IP Tailscale:** `100.112.66.105`
* **Función A (Control):** Aloja el broker Mosquitto maestro donde convergen todas las comunicaciones.
* **Función B (Datos):** Aloja el servidor HTTP (Puerto 8080) desde el cual los nodos descargan los binarios (`.bin`).


* **Servidores de Producción (Puentes / Gateways Locales):**
* **Función:** Servidores intermedios en las redes locales. Ejecutan un broker Mosquitto configurado como *Bridge* que retransmite el tráfico local hacia Dracarys, actuando como orquestadores locales antes de enviar las placas al campo.


## 3. Arquitectura del Hardware (El Nodo)

* **Microcontrolador:** ESP32-S3.
* **Memoria:** 16MB de memoria Flash.
* **Mapa de Particiones:** Arquitectura de "Doble Banco" (Dual Bank). Se han asignado 4MB a cada banco OTA (`ota_0` y `ota_1`) para garantizar espacio suficiente para el firmware remoto y futuros modelos de machine learning.
* **Firmware:** Escrito en **Rust** (entorno `no_std` parcial / `esp-idf-svc`), optimizado para tamaño de flash (`opt-level = 's'`).

## 4. El Flujo de Actualización (Modelo Push/Pull)

Para evitar el colapso del tráfico (polling constante), el sistema utiliza un enfoque híbrido:

1. **Evaluación (PostgreSQL):** La base de datos determina qué MACs necesitan actualizarse en función de su hardware.
2. **Notificación PUSH (MQTT):** El servidor publica un mensaje JSON con la orden de actualización en el topic específico del dispositivo: `flota/cmd/{MAC_ADDRESS}`.
3. **Ejecución PULL (HTTP):** La placa ESP32, que está suscrita a su topic de comandos de forma persistente, recibe la alerta, extrae la URL de descarga (ej. `http://100.112.66.105:8080/firmware.bin`) y ejecuta una petición GET para descargar el binario en el banco OTA inactivo. Al finalizar, cambia la partición de arranque y se reinicia.

## 5. Lecciones Aprendidas (Historial de Estabilización)

*Nota para el desarrollador/becario: Si encuentras fallos de conectividad, revisa estos hitos que ya han sido depurados.*

* **Bloqueo MQTT en ESP32 (Timeout):**
* *El Problema:* El ESP32 se conectaba al WiFi pero era expulsado del broker a los 30 segundos (`disconnect by timeout`).
* *La Solución:* En `esp-idf-svc`, la gestión de la conexión MQTT bloquea el hilo si no se procesan los eventos. Es **estrictamente obligatorio** utilizar una arquitectura de hilos concurrentes. El cliente MQTT de publicación se ejecuta en el hilo principal, pero el mantenimiento del socket (`connection.next()`) debe ejecutarse en un hilo de fondo (`std::thread::spawn`) para responder automáticamente a los Pings de *Keep-Alive* del servidor.


* **Validación del Bridge Mosquitto:**
* Para comprobar si un nodo está aislado o si el puente hacia Dracarys funciona, se publica un mensaje manual en el servidor local (`mosquitto_pub -h localhost -t "flota/status/TEST" -m '{"test": "ok"}'`). Si el mensaje aparece en el servidor maestro (Dracarys), la capa de red está sana y el problema reside en el código Rust del nodo.



## 6. Configuración del Entorno de Desarrollo Local

Para trabajar en este proyecto, tu máquina debe contar con:

1. **Tailscale instalado y autenticado:** Debes ser asignado al grupo de ACL correspondiente para tener visibilidad de la IP `100.112.66.105`.
2. **Rust Toolchain para ESP32:** Instalación de utilidades como `cargo-espflash` para compilar y flashear el código en las placas de prueba.
3. **Herramientas CLI MQTT:** `mosquitto-clients` instalado localmente (`mosquitto_pub` y `mosquitto_sub`) para monitorizar y falsear telemetría y comandos durante el desarrollo.

---