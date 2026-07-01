#!/bin/bash
# ============================================================================
# Archivo: flash_ota.sh
# Proyecto: FLEET_OTA
# Descripción: Script de despliegue con validación de entorno.
# Uso:  ./flash_ota.sh           (build incremental + flash, rápido)
#       ./flash_ota.sh --clean   (limpieza profunda + flash, tras cambiar .env)
# ============================================================================

set -e

PARTITION_FILE="partitions.csv"
REQUIRED_ENV="MQTT_BROKER_URL_LOCAL_HOST"
LOG_PREFIX="[FLEET_OTA FLASH]"

echo "$LOG_PREFIX Iniciando proceso de despliegue..."

# 1. Validar variables de entorno (blindaje contra errores de compilación)
if [ -z "${!REQUIRED_ENV}" ]; then
    echo "$LOG_PREFIX ERROR: Variable de entorno '$REQUIRED_ENV' no definida."
    echo "       Verifique que direnv cargó el .env (cd al proyecto)."
    exit 1
fi

# 2. Limpieza OPCIONAL (solo con --clean; cargo clean recompila esp-idf entero)
if [ "$1" == "--clean" ]; then
    echo "$LOG_PREFIX Limpieza profunda solicitada (cargo clean)..."
    cargo clean
fi

# 3. Build y Flash
echo "$LOG_PREFIX Compilando con Broker: ${!REQUIRED_ENV}"
if cargo espflash flash --monitor --release --partition-table "$PARTITION_FILE"; then
    echo "$LOG_PREFIX Deploy concluido con éxito en $(date)."
else
    echo "$LOG_PREFIX ERROR: El proceso de flasheo falló."
    exit 1
fi