// ============================================================================
// Archivo: ota_server/src/main.rs
// Proyecto: FLEET_OTA
// Módulo: Servidor HTTP para distribución de firmware
// Fecha: 2026-06-30
// ============================================================================

use axum::{routing::get_service, Router};
use tower_http::services::ServeDir;
use std::net::SocketAddr;
use log::{info, warn};

#[tokio::main]
async fn main() {
    env_logger::init();

    let app = Router::new()
        .fallback_service(
            get_service(ServeDir::new("firmware_storage"))
                .handle_error(|error| async move {
                    warn!("Error sirviendo archivo: {}", error);
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR
                })
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("🚀 Servidor OTA activo en http://{}", addr);
    info!("📦 Sirviendo archivos desde ./firmware_storage");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}