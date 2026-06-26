use sqlx::postgres::PgPoolOptions;
use dotenvy::dotenv;
use std::env;
mod telemetry_handler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> { // Cambiamos a Box<dyn Error> para manejar migraciones
    // 1. Cargamos variables de entorno
    dotenv().ok();
    
    // 2. Leemos la URL
    let database_url = env::var("DATABASE_URL")
        .expect("La variable DATABASE_URL no está definida en el .env");

    println!("📡 Conectando a la base de datos: {}", database_url);

    // 3. Creamos el Pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("✅ ¡Conexión exitosa a PostgreSQL!"); 

    // 4. Lanzamos el worker
    // Necesitamos clonar el pool para que el worker pueda usarlo de forma independiente
    let worker_pool = pool.clone(); 
    tokio::spawn(async move {
        if let Err(e) = telemetry_handler::run_telemetry_worker(worker_pool).await {
            eprintln!("Worker error: {:?}", e);
        }
    });    
    
    // Mantenemos el servidor vivo
    tokio::signal::ctrl_c().await?;
    Ok(())
}