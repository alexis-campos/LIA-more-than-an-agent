// lia-client/src-tauri/src/context.rs
// Módulo de contexto compartido: almacena el último estado del editor (VS Code)
// recibido a través del WebSocket local (Contrato A: context_update).

use serde::Deserialize;
use std::sync::{Arc, Mutex};

/// Representa el contexto del archivo activo en el editor.
/// Contiene la ventana de ±50 líneas alrededor del cursor.
#[derive(Debug, Deserialize, Clone)]
pub struct FileContext {
    pub file_name: String,
    pub file_path: String,
    pub language: String,
    pub cursor_line: u32,
    pub content_window: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ContextUpdate {
    pub event_type: String,
    pub timestamp: u64,
    pub workspace_name: String,
    pub file_context: FileContext,
}

/// Tipo compartido que permite acceso seguro al contexto desde múltiples hilos.
/// - `Arc`: permite clonar la referencia para pasarla entre hilos (WebSocket, Tauri, etc.)
/// - `Mutex`: garantiza exclusión mutua al leer/escribir el contexto
/// - `Option`: es `None` hasta que VS Code envíe el primer context_update
pub type SharedContext = Arc<Mutex<Option<ContextUpdate>>>;

/// Crea una nueva instancia del contexto compartido, inicialmente vacía.
pub fn create_shared_context() -> SharedContext {
    Arc::new(Mutex::new(None))
}
