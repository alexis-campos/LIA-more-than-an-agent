// lia-client/src-tauri/src/playback.rs
// Modulo de reproduccion de audio con soporte para echo cancellation.
// Setea el flag de reproduccion para que el microfono descarte muestras.

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::io::Cursor;
use std::sync::atomic::Ordering;

use crate::audio::PlayingFlag;

/// Reproductor de audio que mantiene un output stream abierto.
pub struct AudioPlayer {
    _stream: OutputStream,
    _handle: OutputStreamHandle,
    sink: Sink,
    playing_flag: PlayingFlag,
}

impl AudioPlayer {
    /// Crea un nuevo reproductor con echo cancellation integrado.
    pub fn new(playing_flag: PlayingFlag) -> Result<Self, String> {
        let (stream, handle) = OutputStream::try_default()
            .map_err(|e| format!("No se pudo abrir el dispositivo de audio: {}", e))?;

        let sink =
            Sink::try_new(&handle).map_err(|e| format!("No se pudo crear el sink: {}", e))?;

        println!("Reproductor de audio inicializado (echo cancel enlazado)");

        Ok(AudioPlayer {
            _stream: stream,
            _handle: handle,
            sink,
            playing_flag,
        })
    }

    /// Reproduce un fragmento de audio (bytes WAV).
    pub fn play_chunk(&self, audio_bytes: &[u8]) -> Result<(), String> {
        // Activar echo cancellation
        self.playing_flag.store(true, Ordering::Relaxed);

        let cursor = Cursor::new(audio_bytes.to_vec());
        let source =
            Decoder::new(cursor).map_err(|e| format!("Error al decodificar audio: {}", e))?;

        self.sink.append(source);
        Ok(())
    }

    /// Detiene toda la reproduccion inmediatamente.
    pub fn stop(&self) {
        self.sink.stop();
        self.playing_flag.store(false, Ordering::Relaxed);
    }

    /// Verifica si el reproductor esta actualmente reproduciendo audio.
    pub fn is_playing(&self) -> bool {
        let playing = !self.sink.empty();
        // Actualizar el flag en tiempo real
        self.playing_flag.store(playing, Ordering::Relaxed);
        playing
    }
}
