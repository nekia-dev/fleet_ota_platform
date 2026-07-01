#!/bin/bash
# ==============================================================================
# export_firmware.sh
# Script industrial para exportar binarios de firmware (.bin) a Dracarys
# ==============================================================================

# --- CONFIGURACIÓN DE RED Y DESTINO ---
REMOTE_USER="dracarys"
REMOTE_HOST="100.112.66.105"
REMOTE_PORT="9636"
REMOTE_DIR="/opt/fleet_ota/firmware_storage/"

# --- CONFIGURACIÓN LOCAL DINÁMICA ---
# Detecta de forma absoluta dónde está guardado este script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
LOCAL_STORAGE="$SCRIPT_DIR/ota_server/firmware_storage"

echo "=========================================================="
echo "🚀 INICIANDO EXPORTACIÓN DE FLOTA (FÁBRICA -> NUBE)"
echo "=========================================================="
echo "📂 Origen Local: $LOCAL_STORAGE"
echo "🖥️  Destino Remoto: $REMOTE_USER@$REMOTE_HOST:$REMOTE_PORT$REMOTE_DIR"
echo "----------------------------------------------------------"

# 1. Validación de existencia del directorio local
if [ ! -d "$LOCAL_STORAGE" ]; then
    echo "❌ Error: El directorio local '$LOCAL_STORAGE' no existe."
    exit 1
fi

# 2. Conteo de archivos .bin presentes
BIN_COUNT=$(ls -1 "$LOCAL_STORAGE"/*.bin 2>/dev/null | wc -l)

if [ "$BIN_COUNT" -eq 0 ]; then
    echo "⚠️  Atención: No se encontraron archivos '.bin' en $LOCAL_STORAGE"
    echo "Genera el binario en tu entorno de hardware antes de exportar."
    exit 1
fi

echo "📦 Se detectaron $BIN_COUNT archivo(s) de firmware listos."

# 3. Transferencia inteligente (Sincronización)
# Intentamos usar rsync por eficiencia (solo sube lo nuevo), si no, usamos scp
if command -v rsync &> /dev/null; then
    echo "⚡ Ejecutando sincronización eficiente con rsync..."
    rsync -avz --progress -e "ssh -p $REMOTE_PORT" "$LOCAL_STORAGE/" "$REMOTE_USER@$REMOTE_HOST:$REMOTE_DIR"
else
    echo "🚚 rsync no detectado en local. Usando scp tradicional..."
    scp -P $REMOTE_PORT "$LOCAL_STORAGE"/*.bin "$REMOTE_USER@$REMOTE_HOST:$REMOTE_DIR"
fi

# 4. Verificación de código de salida
if [ $? -eq 0 ]; then
    echo "----------------------------------------------------------"
    echo "✅ ¡Exportación completada con éxito!"
    echo "🔒 Los firmwares están disponibles en el almacenamiento de Dracarys."
    echo "=========================================================="
else
    echo "----------------------------------------------------------"
    echo "❌ Error en la transferencia. Verifica la red Tailscale o el puerto $REMOTE_PORT."
    echo "=========================================================="
    exit 1
fi