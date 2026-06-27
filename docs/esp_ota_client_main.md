### Responsabilidades críticas de main.rs:
## Inicialización de Bajo Nivel:

- Gestiona los Peripherals del ESP32-S3 (GPIO, Modem).

- Configura el EspSystemEventLoop para manejar eventos del sistema (Wi-Fi, OTA, etc.) de forma no bloqueante.

- Inicializa la partición NVS (Non-Volatile Storage), donde se guardan configuraciones críticas como las credenciales Wi-Fi o el estado de la última actualización.

### Gestión de la Capa de Conectividad (Wi-Fi):

- Configura y levanta el stack Wi-Fi.

- Implementa el bucle de "espera activa" hasta que la IP es asignada por DHCP.

### Identificación Única: Extrae la dirección MAC real del chip y la formatea como una cadena hexadecimal para que el Broker MQTT pueda identificar el nodo de forma unívoca (E8:3D:C1:F2:D9:C4).

- Lanzamiento de Servicios (Concurrencia):

- Invoca al servicio MQTT (vía mqtt.rs), asegurándose de que este no bloquee la ejecución.

Esta es la clave para la estabilidad: main.rs arranca el servicio y libera el hilo principal, permitiendo que el sistema sea capaz de realizar otras tareas mientras escucha comandos OTA.

Mantenimiento del Nodo (Watchdog/Supervisor):

El loop final que ves al final del archivo es una "cárcel" o supervisor de seguridad. Evita que el programa termine y, por tanto, evita que el Watchdog del hardware interprete que el firmware ha crasheado y fuerce un reinicio innecesario.