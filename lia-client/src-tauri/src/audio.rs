use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::time::Duration;

pub fn probar_oido() {
    println!("Iniciando prueba de audición...");

    // Obtener el motor de audio del sistema (ALSA en tu caso, por ser Linux)
    let host = cpal::default_host();

    // Buscar el micrófono principal
    let device = match host.default_input_device() {
        Some(d) => d,
        None => {
            eprintln!("Lia no detectó ningún micrófono.");
            return;
        }
    };

    println!("Micrófono detectado: {}", device.name().unwrap_or_else(|_| "Desconocido".to_string()));

    let config = match device.default_input_config() {
        Ok(c) => c.into(),
        Err(e) => {
            eprintln!("Error al obtener configuración del micrófono: {}", e);
            return;
        }
    };

    // Crear el "stream" (el túnel de datos de audio)
    let err_fn = move |err| {
        eprintln!("Error en el stream de audio: {}", err);
    };

    // Esta función se ejecuta MUCHAS veces por segundo mientras hablas
    let stream = device.build_input_stream(
        &config,
        move |_data: &[f32], _: &cpal::InputCallbackInfo| {
            // Aquí es donde en la Fase 2 acumularemos los fragmentos de audio 
            // para enviarlos a la nube. Por ahora lo dejamos vacío para no 
            // saturar tu terminal de texto.
        },
        err_fn,
        None, // Timeout opcional
    ).expect("No se pudo construir el stream de audio");

    stream.play().expect("No se pudo iniciar el stream");
    println!("Lia está escuchando tu micrófono por 3 segundos...");

    std::thread::sleep(Duration::from_secs(3));
    
    println!("Prueba de audición terminada exitosamente.");
}