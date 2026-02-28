// lia-client/src-tauri/src/wakeword.rs
// Deteccion de actividad vocal (VAD) por energia RMS.
// Activa la grabacion cuando detecta voz y la detiene tras silencio prolongado.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Umbral de energia RMS para considerar que hay voz.
/// Valores tipicos: 0.01-0.05 (ajustar segun el microfono).
const ENERGY_THRESHOLD: f32 = 0.02;

/// Milisegundos de voz continua para activar la grabacion.
const ACTIVATION_MS: u64 = 200;

/// Milisegundos de silencio para detener la grabacion.
const SILENCE_MS: u64 = 1500;

/// Estado del detector de voz.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VadState {
    Silent,
    Speaking,
}

/// Detector de actividad vocal basado en energia RMS.
pub struct VoiceActivityDetector {
    state: VadState,
    speaking_flag: Arc<AtomicBool>,
    energy_above_count: u64,
    energy_below_count: u64,
    sample_rate: u32,
    samples_per_frame: usize,
}

impl VoiceActivityDetector {
    /// Crea un nuevo detector. `speaking_flag` se setea a true cuando hay voz.
    pub fn new(speaking_flag: Arc<AtomicBool>, sample_rate: u32) -> Self {
        // Analizar frames de ~20ms
        let samples_per_frame = (sample_rate as usize) / 50;

        VoiceActivityDetector {
            state: VadState::Silent,
            speaking_flag,
            energy_above_count: 0,
            energy_below_count: 0,
            sample_rate,
            samples_per_frame,
        }
    }

    /// Procesa un frame de audio y retorna el estado actual.
    /// Debe llamarse con bloques de muestras del stream de cpal.
    pub fn process_frame(&mut self, samples: &[f32]) -> VadState {
        let rms = compute_rms(samples);
        let frame_duration_ms = (samples.len() as u64 * 1000) / self.sample_rate as u64;

        if rms > ENERGY_THRESHOLD {
            self.energy_above_count += frame_duration_ms;
            self.energy_below_count = 0;
        } else {
            self.energy_below_count += frame_duration_ms;
            self.energy_above_count = 0;
        }

        match self.state {
            VadState::Silent => {
                if self.energy_above_count >= ACTIVATION_MS {
                    self.state = VadState::Speaking;
                    self.speaking_flag.store(true, Ordering::Relaxed);
                    println!("VAD: Voz detectada, activando grabacion");
                }
            }
            VadState::Speaking => {
                if self.energy_below_count >= SILENCE_MS {
                    self.state = VadState::Silent;
                    self.speaking_flag.store(false, Ordering::Relaxed);
                    println!("VAD: Silencio detectado, deteniendo grabacion");
                }
            }
        }

        self.state
    }

    /// Retorna el tamano recomendado de frame para este detector.
    pub fn frame_size(&self) -> usize {
        self.samples_per_frame
    }
}

/// Calcula la energia RMS (Root Mean Square) de un bloque de muestras.
fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|s| s * s).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rms_silencio() {
        let silence = vec![0.0f32; 320];
        assert!(compute_rms(&silence) < ENERGY_THRESHOLD);
    }

    #[test]
    fn test_rms_voz() {
        // Simular una onda con amplitud significativa
        let voice: Vec<f32> = (0..320).map(|i| (i as f32 * 0.1).sin() * 0.5).collect();
        assert!(compute_rms(&voice) > ENERGY_THRESHOLD);
    }

    #[test]
    fn test_vad_transicion() {
        let flag = Arc::new(AtomicBool::new(false));
        let mut vad = VoiceActivityDetector::new(flag.clone(), 16000);

        // Enviar frames de silencio: debe permanecer Silent
        let silence = vec![0.0f32; 320];
        for _ in 0..10 {
            assert_eq!(vad.process_frame(&silence), VadState::Silent);
        }

        // Enviar frames con voz suficiente para activar (>200ms)
        let voice: Vec<f32> = (0..320).map(|i| (i as f32 * 0.1).sin() * 0.5).collect();
        for _ in 0..20 {
            vad.process_frame(&voice);
        }
        assert_eq!(vad.state, VadState::Speaking);
        assert!(flag.load(Ordering::Relaxed));
    }
}
