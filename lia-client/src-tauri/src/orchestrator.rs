// lia-client/src-tauri/src/orchestrator.rs
// Maquina de estados global que coordina el flujo completo de una interaccion:
// IDLE -> LISTENING -> THINKING -> RESPONDING -> IDLE

use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Estados posibles de Lia.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum LiaState {
    Idle,
    Listening,
    Thinking,
    Responding,
}

impl LiaState {
    /// Retorna el nombre del estado como string para el frontend.
    pub fn as_str(&self) -> &'static str {
        match self {
            LiaState::Idle => "IDLE",
            LiaState::Listening => "LISTENING",
            LiaState::Thinking => "THINKING",
            LiaState::Responding => "RESPONDING",
        }
    }
}

/// Orquestador que gestiona las transiciones de estado y emite
/// eventos al frontend React via Tauri.
pub struct Orchestrator {
    state: LiaState,
    app_handle: Option<AppHandle>,
}

impl Orchestrator {
    /// Crea un nuevo orquestador en estado IDLE.
    pub fn new() -> Self {
        Orchestrator {
            state: LiaState::Idle,
            app_handle: None,
        }
    }

    /// Asocia un AppHandle de Tauri para emitir eventos al frontend.
    pub fn set_app_handle(&mut self, handle: AppHandle) {
        self.app_handle = Some(handle);
    }

    /// Retorna el estado actual.
    pub fn state(&self) -> LiaState {
        self.state
    }

    /// Transiciona al siguiente estado y emite evento al frontend.
    pub fn transition_to(&mut self, new_state: LiaState) {
        let old = self.state;
        self.state = new_state;
        println!("Estado: {} -> {}", old.as_str(), new_state.as_str());

        // Emitir evento al frontend React
        if let Some(ref app) = self.app_handle {
            let _ = app.emit("lia://state-change", new_state.as_str());
        }
    }

    /// Transiciona a LISTENING (VAD detecto voz).
    pub fn start_listening(&mut self) {
        if self.state == LiaState::Idle {
            self.transition_to(LiaState::Listening);
        }
    }

    /// Transiciona a THINKING (silencio detectado, procesando).
    pub fn start_thinking(&mut self) {
        if self.state == LiaState::Listening {
            self.transition_to(LiaState::Thinking);
        }
    }

    /// Transiciona a RESPONDING (stream de Gemini iniciado).
    pub fn start_responding(&mut self) {
        if self.state == LiaState::Thinking {
            self.transition_to(LiaState::Responding);
        }
    }

    /// Vuelve a IDLE (stream completado).
    pub fn finish(&mut self) {
        self.transition_to(LiaState::Idle);

        // Emitir evento de fin de stream
        if let Some(ref app) = self.app_handle {
            let _ = app.emit("lia://stream-end", ());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transiciones_validas() {
        let mut orc = Orchestrator::new();
        assert_eq!(orc.state(), LiaState::Idle);

        orc.start_listening();
        assert_eq!(orc.state(), LiaState::Listening);

        orc.start_thinking();
        assert_eq!(orc.state(), LiaState::Thinking);

        orc.start_responding();
        assert_eq!(orc.state(), LiaState::Responding);

        orc.finish();
        assert_eq!(orc.state(), LiaState::Idle);
    }

    #[test]
    fn test_transicion_invalida_ignorada() {
        let mut orc = Orchestrator::new();

        // No puede pasar de IDLE a THINKING directamente
        orc.start_thinking();
        assert_eq!(orc.state(), LiaState::Idle);

        // No puede pasar de IDLE a RESPONDING
        orc.start_responding();
        assert_eq!(orc.state(), LiaState::Idle);
    }
}
