#!/bin/bash
# ============================================================================
# Archivo: flash_ota.sh
# Proyecto: FLEET_OTA
# Descripción: Script para compilar y flashear el firmware en nodos ESP32-S3.
# ============================================================================

# Define o diretório do projeto e arquivos críticos
PARTITION_FILE="partitions.csv"
LOG_PREFIX="[FLEET_OTA FLASH]"

echo "$LOG_PREFIX Iniciando processo de deployment..."

# 1. Verifica se a tabela de partição existe
if [ ! -f "$PARTITION_FILE" ]; then
    echo "$LOG_PREFIX ERRO: $PARTITION_FILE não encontrado."
    exit 1
fi

# 2. Executa o build e flash
# Usamos --release para garantir a otimização de performance (-Os)
echo "$LOG_PREFIX Executando build e flash com $PARTITION_FILE..."

cargo espflash flash --monitor --release --partition-table "$PARTITION_FILE"

# 3. Verifica o código de saída
if [ $? -eq 0 ]; then
    echo "$LOG_PREFIX Deploy concluído com sucesso."
else
    echo "$LOG_PREFIX ERRO: Falha no processo de flash."
    exit 1
fi