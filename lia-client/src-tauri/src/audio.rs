// lia-client/src-tauri/src/audio.rs
// Modulo de audio: grabacion real del microfono con echo cancellation.
// El audio se codifica como WAV (PCM 16-bit, mono, 16kHz) para enviar al backend.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const SAMPLE_RATE: u32 = 16000;

/// Flag compartido para echo cancellation.
/// Cuando `is_playing` es true, la grabacion descarta muestras.
pub type PlayingFlag = Arc<AtomicBool>;

pub fn create_playing_flag() -> PlayingFlag {
    Arc::new(AtomicBool::new(false))
}

/// Mantiene el estado de una grabacion en curso.
pub struct AudioRecorder {
    stream: cpal::Stream,
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
}

/// Prueba basica del microfono (Fase 1, solo diagnostico).
pub fn probar_oido() {
    println!("Iniciando prueba de audicion...");

    let host = cpal::default_host();
    let device = match host.default_input_device() {
        Some(d) => d,
        None => {
            eprintln!("Lia no detecto ningun microfono.");
            return;
        }
    };

    println!(
        "Microfono detectado: {}",
        device.name().unwrap_or_else(|_| "Desconocido".to_string())
    );
    println!("Prueba de audicion terminada exitosamente.");
}

/// Inicia la grabacion del microfono con echo cancellation.
/// Si `playing_flag` es true, las muestras se descartan (Lia esta hablando).
pub fn start_recording(playing_flag: PlayingFlag) -> Result<AudioRecorder, String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "No se detecto ningun microfono".to_string())?;

    let config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(SAMPLE_RATE),
        buffer_size: cpal::BufferSize::Default,
    };

    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let buffer_clone = buffer.clone();

    let err_fn = |err| {
        eprintln!("Error en el stream de audio: {}", err);
    };

    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // Echo cancellation: descartar muestras si Lia esta hablando
                if playing_flag.load(Ordering::Relaxed) {
                    return;
                }
                if let Ok(mut buf) = buffer_clone.lock() {
                    buf.extend_from_slice(data);
                }
            },
            err_fn,
            None,
        )
        .map_err(|e| format!("No se pudo construir el stream: {}", e))?;

    stream
        .play()
        .map_err(|e| format!("No se pudo iniciar la grabacion: {}", e))?;

    println!(
        "Grabacion de audio iniciada ({}Hz, mono, echo cancel activo)",
        SAMPLE_RATE
    );

    Ok(AudioRecorder {
        stream,
        buffer,
        sample_rate: SAMPLE_RATE,
    })
}

/// Detiene la grabacion y retorna los datos como bytes WAV.
pub fn stop_recording(recorder: AudioRecorder) -> Result<Vec<u8>, String> {
    drop(recorder.stream);

    let samples = recorder
        .buffer
        .lock()
        .map_err(|e| format!("Error al acceder al buffer: {}", e))?
        .clone();

    if samples.is_empty() {
        return Err("No se grabo ningun audio".to_string());
    }

    println!(
        "Grabacion finalizada: {} muestras ({:.1}s)",
        samples.len(),
        samples.len() as f64 / recorder.sample_rate as f64
    );

    encode_wav(&samples, recorder.sample_rate)
}

/// Codifica muestras f32 a bytes WAV (PCM 16-bit, mono).
fn encode_wav(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>, String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec)
            .map_err(|e| format!("Error al crear WavWriter: {}", e))?;

        for &sample in samples {
            let clamped = sample.max(-1.0).min(1.0);
            let value = (clamped * 32767.0) as i16;
            writer
                .write_sample(value)
                .map_err(|e| format!("Error al escribir muestra: {}", e))?;
        }

        writer
            .finalize()
            .map_err(|e| format!("Error al finalizar WAV: {}", e))?;
    }

    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_wav_vacio() {
        let result = encode_wav(&[], 16000);
        assert!(result.is_ok());
        let wav = result.unwrap();
        assert!(wav.len() >= 44);
    }

    #[test]
    fn test_encode_wav_con_datos() {
        let samples: Vec<f32> = (0..1600).map(|i| (i as f32 / 1600.0).sin()).collect();
        let result = encode_wav(&samples, 16000);
        assert!(result.is_ok());
        let wav = result.unwrap();
        assert!(wav.len() > 44);
    }
}
