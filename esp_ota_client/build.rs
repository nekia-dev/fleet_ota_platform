use std::fs;
use std::path::Path;


fn main() {
    // 1. Configuración del Linker (Obligatorio)
    embuild::espidf::sysenv::output();

    // 2. Inyección de variables desde el .env en la raíz del workspace
    let env_path = Path::new("../.env"); 
    
    if let Ok(content) = fs::read_to_string(env_path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let clean_key = key.trim();
                let clean_value = value.trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .trim_matches('\r');
                
                // INYECCIÓN DIRECTA PARA env!()
                println!("cargo:rustc-env={}={}", clean_key, clean_value);
            }
        }
        println!("cargo:rerun-if-changed=../.env");
    } else {
        println!("cargo:warning=⚠️ No se pudo leer el archivo .env en {:?}", env_path);
    }
    
    println!("cargo:rerun-if-changed=build.rs");
}