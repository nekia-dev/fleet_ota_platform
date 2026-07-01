use axum::{routing::get_service, Router};
use tower_http::services::ServeDir;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // Definimos el router que sirve archivos desde la carpeta local ./firmware_storage
    let app = Router::new()
        .fallback_service(
            get_service(ServeDir::new("firmware_storage"))
        );

    // Bind a la IP local de tu servidor (192.168.1.42) en el puerto 8000
    let addr = SocketAddr::from(([192, 168, 1, 42], 8000));
    println!("🚀 Servidor OTA activo en http://{}", addr);
    println!("📦 Asegúrate de que los binarios estén en ./firmware_storage");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}