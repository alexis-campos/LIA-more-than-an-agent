// lia-client/src-tauri/src/playback.rs
// Modulo de reproduccion de audio: recibe chunks de audio (WAV/MP3)
// y los reproduce inmediatamente a traves de los altavoces usando rodio.

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::io::Cursor;

/// Reproductor de audio que mantiene un output stream abierto.
pub struct AudioPlayer {
    _stream: OutputStream,
    _handle: OutputStreamHandle,
    sink: Sink,
}

impl AudioPlayer {
    /// Crea un nuevo reproductor de audio.
    /// Abre el dispositivo de salida por defecto del sistema.
    pub fn new() -> Result<Self, String> {
        let (stream, handle) = OutputStream::try_default()
            .map_err(|e| format!("No se pudo abrir el dispositivo de audio: {}", e))?;

        let sink =
            Sink::try_new(&handle).map_err(|e| format!("No se pudo crear el sink: {}", e))?;

        println!("Reproductor de audio inicializado");

        Ok(AudioPlayer {
            _stream: stream,
            _handle: handle,
            sink,
        })
    }

    /// Reproduce un fragmento de audio (bytes WAV).
    /// Los chunks se encolaran y reproduciran secuencialmente.
    pub fn play_chunk(&self, audio_bytes: &[u8]) -> Result<(), String> {
        let cursor = Cursor::new(audio_bytes.to_vec());
        let source =
            Decoder::new(cursor).map_err(|e| format!("Error al decodificar audio: {}", e))?;

        self.sink.append(source);
        Ok(())
    }

    /// Detiene toda la reproduccion inmediatamente.
    pub fn stop(&self) {
        self.sink.stop();
    }

    /// Verifica si el reproductor esta actualmente reproduciendo audio.
    pub fn is_playing(&self) -> bool {
        !self.sink.empty()
    }
}
