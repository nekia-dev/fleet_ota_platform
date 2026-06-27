-- Migración para crear la tabla de telemetría
CREATE TABLE IF NOT EXISTS device_telemetry (
    id SERIAL PRIMARY KEY,
    mac_address VARCHAR(17) NOT NULL UNIQUE,
    status VARCHAR(17) NOT NULL,
    version VARCHAR(8) NOT NULL,
    last_seen TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Crear un índice para búsquedas rápidas por MAC
CREATE INDEX IF NOT EXISTS idx_device_mac ON device_telemetry(mac_address);