// lia-client/src-tauri/src/vision.rs
// Modulo de vision: captura de pantalla con soporte multi-monitor.

use std::time::Instant;
use xcap::Monitor;

/// Prueba basica de vision (Fase 1, solo diagnostico).
pub fn probar_vision() {
    println!("Iniciando prueba de vision...");
    let start = Instant::now();

    let monitores = Monitor::all().unwrap_or_else(|_| vec![]);

    if monitores.is_empty() {
        eprintln!("Lia no detecto ningun monitor.");
        return;
    }

    let monitor = &monitores[0];
    println!(
        "Monitor detectado: {} ({}x{})",
        monitor.name(),
        monitor.width(),
        monitor.height()
    );

    match monitor.capture_image() {
        Ok(imagen) => {
            let ruta_salida = std::env::temp_dir().join("ojo_de_lia_test.png");
            if let Err(e) = imagen.save(&ruta_salida) {
                eprintln!("Error al guardar la captura: {}", e);
            } else {
                let duration = start.elapsed();
                println!("Captura exitosa en {:?}: {:?}", duration, ruta_salida);
            }
        }
        Err(e) => {
            eprintln!("Error al capturar la pantalla: {}", e);
        }
    }
}

/// Captura la pantalla del monitor primario y retorna los bytes PNG.
/// Soportado multi-monitor: usa el primer monitor disponible.
pub fn capture_screen() -> Result<Vec<u8>, String> {
    let monitores = Monitor::all().map_err(|e| format!("Error al enumerar monitores: {}", e))?;

    if monitores.is_empty() {
        return Err("No se detecto ningun monitor".to_string());
    }

    // Listar monitores disponibles
    for (i, m) in monitores.iter().enumerate() {
        println!(
            "  Monitor {}: {} ({}x{})",
            i,
            m.name(),
            m.width(),
            m.height()
        );
    }

    // Capturar el monitor primario (indice 0)
    let monitor = &monitores[0];
    let imagen = monitor
        .capture_image()
        .map_err(|e| format!("Error al capturar pantalla: {}", e))?;

    // Codificar como PNG en memoria
    let mut buffer = std::io::Cursor::new(Vec::new());
    imagen
        .write_to(&mut buffer, image::ImageFormat::Png)
        .map_err(|e| format!("Error al codificar PNG: {}", e))?;

    Ok(buffer.into_inner())
}
