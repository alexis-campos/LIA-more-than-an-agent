use xcap::Monitor;
use std::time::Instant;

pub fn probar_vision() {
    println!("Iniciando prueba de visión...");
    let start = Instant::now();

    let monitores = Monitor::all().unwrap_or_else(|_| vec![]);

    if monitores.is_empty() {
        eprintln!("Lia no detectó ningún monitor.");
        return;
    }

    // Tomamos el primer monitor (el principal)
    let monitor_principal = &monitores[0];
    println!(
        "Monitor detectado: {} (Resolución: {}x{})",
        monitor_principal.name(),
        monitor_principal.width(),
        monitor_principal.height()
    );

    // Capturamos la pantalla
    match monitor_principal.capture_image() {
        Ok(imagen) => {
            // Usamos la carpeta temporal del sistema (en Linux es /tmp/)
            let ruta_salida = std::env::temp_dir().join("ojo_de_lia_test.png");
            
            if let Err(e) = imagen.save(&ruta_salida) {
                eprintln!("Error al guardar la captura: {}", e);
            } else {
                let duration = start.elapsed();
                println!("¡Captura exitosa! Guardada en {:?} en {:?}", ruta_salida, duration);
            }
        }
        Err(e) => {
            eprintln!("Error al capturar la pantalla: {}", e);
        }
    }
}