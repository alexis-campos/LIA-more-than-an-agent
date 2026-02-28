// lia-client/src-tauri/src/main.rs
// Punto de entrada principal de Lia Desktop.
// Levanta el servidor WebSocket para VS Code, conecta al Cloud Python,
// y retransmite la respuesta de Gemini al HUD React.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod audio;
mod cloud_client;
mod context;
mod hasher;
mod orchestrator;
mod playback;
mod request;
mod sentinel;
mod vision;
mod wakeword;

use context::{ContextUpdate, SharedContext};
use futures_util::StreamExt;
use orchestrator::Orchestrator;
use sentinel::Sentinel;
use serde::Serialize;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use warp::Filter;

/// URL del Cloud Python.
const CLOUD_URL: &str = "ws://127.0.0.1:8000/ws/lia?token=lia-dev-token-2024";

/// Info de contexto para el frontend React.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ContextInfo {
    file_name: String,
    language: String,
    cursor_line: u32,
}

/// Estado compartido para los Tauri commands.
struct AppState {
    ctx: SharedContext,
    sentinel: Arc<Sentinel>,
    orchestrator: Arc<Mutex<Orchestrator>>,
    prev_code_hash: Mutex<Option<String>>,
    prev_image_hash: Mutex<Option<String>>,
}

fn find_available_port(preferred: u16) -> u16 {
    if TcpListener::bind(("127.0.0.1", preferred)).is_ok() {
        return preferred;
    }
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|addr| addr.port())
        .unwrap_or(preferred)
}

fn write_port_file(port: u16) {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let lia_dir = std::path::PathBuf::from(&home).join(".lia");
    let _ = std::fs::create_dir_all(&lia_dir);
    let _ = std::fs::write(lia_dir.join("port"), port.to_string());
}

fn cleanup_port_file() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let _ = std::fs::remove_file(std::path::PathBuf::from(&home).join(".lia").join("port"));
}

/// Pipeline completo: contexto → captura → Sentinel → Cloud → HUD streaming.
async fn trigger_inference(
    ctx: &SharedContext,
    app: &AppHandle,
    sentinel: &Sentinel,
    orchestrator: &Arc<Mutex<Orchestrator>>,
    prev_code_hash: &Mutex<Option<String>>,
    prev_image_hash: &Mutex<Option<String>>,
) {
    // 1. Leer contexto actual
    let context_data = {
        let lock = ctx.lock().unwrap();
        lock.clone()
    };

    let context_data = match context_data {
        Some(data) => data,
        None => {
            let _ = app.emit("lia://stream-chunk",
                "No hay contexto del editor. Abre un archivo en VS Code con la extension de Lia activa.".to_string());
            let _ = app.emit("lia://stream-end", ());
            return;
        }
    };

    // 2. Transicion THINKING
    if let Ok(mut orc) = orchestrator.lock() {
        orc.transition_to(orchestrator::LiaState::Thinking);
    }

    // 3. Capturar pantalla
    let image_data = vision::capture_screen().unwrap_or_else(|e| {
        eprintln!("Error capturando pantalla: {}", e);
        vec![]
    });

    // 4. Smart Caching hashes
    let prev_c = prev_code_hash.lock().unwrap().clone();
    let prev_i = prev_image_hash.lock().unwrap().clone();

    // 5. Construir Contrato B
    let req = request::build_request(
        sentinel,
        &context_data.file_context.content_window,
        &context_data.file_context.language,
        &image_data,
        &[],
        prev_c.as_deref(),
        prev_i.as_deref(),
    );

    // Actualizar hashes
    *prev_code_hash.lock().unwrap() = Some(req.payload.code.hash.clone());
    *prev_image_hash.lock().unwrap() = Some(req.payload.vision.hash.clone());

    let request_json = serde_json::to_string(&req).unwrap();
    println!(
        "Contrato B: id={}, size={} bytes",
        req.request_id,
        request_json.len()
    );

    // 6. Limpiar texto previo en HUD
    let _ = app.emit("lia://stream-clear", ());

    // 7. Transicion RESPONDING
    if let Ok(mut orc) = orchestrator.lock() {
        orc.transition_to(orchestrator::LiaState::Responding);
    }

    // 8. Enviar al Cloud y retransmitir al HUD
    match cloud_client::send_to_cloud_and_stream(CLOUD_URL, &request_json, app).await {
        Ok(()) => println!("Inferencia completada"),
        Err(e) => {
            eprintln!("Error en inferencia: {}", e);
            let _ = app.emit(
                "lia://stream-chunk",
                format!(
                    "[ERROR] {}\n\nAsegurate de que lia-cloud este corriendo (python main.py).",
                    e
                ),
            );
            let _ = app.emit("lia://stream-end", ());
        }
    }

    // 9. Transicion IDLE
    if let Ok(mut orc) = orchestrator.lock() {
        orc.finish();
    }
}

/// Comando Tauri: el boton del HUD llama a esto para disparar inferencia.
#[tauri::command]
async fn ask_lia(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<String, String> {
    println!("Boton 'Preguntar a Lia' presionado");

    trigger_inference(
        &state.ctx,
        &app,
        &state.sentinel,
        &state.orchestrator,
        &state.prev_code_hash,
        &state.prev_image_hash,
    )
    .await;

    Ok("ok".to_string())
}

/// Maneja la conexion WebSocket de VS Code (solo guarda contexto, NO dispara inferencia).
async fn handle_ws_client(websocket: warp::ws::WebSocket, ctx: SharedContext, app: AppHandle) {
    println!("VS Code se ha conectado a Lia.");

    let (_, mut rx) = websocket.split();

    while let Some(result) = rx.next().await {
        match result {
            Ok(msg) => {
                if let Ok(text) = msg.to_str() {
                    match serde_json::from_str::<ContextUpdate>(text) {
                        Ok(update) => {
                            println!(
                                "Contexto: \"{}\" linea={} ({})",
                                update.file_context.file_name,
                                update.file_context.cursor_line,
                                update.file_context.language
                            );

                            let _ = app.emit(
                                "lia://context-update",
                                ContextInfo {
                                    file_name: update.file_context.file_name.clone(),
                                    language: update.file_context.language.clone(),
                                    cursor_line: update.file_context.cursor_line,
                                },
                            );

                            if let Ok(mut lock) = ctx.lock() {
                                *lock = Some(update);
                            }
                        }
                        Err(_) => {}
                    }
                }
            }
            Err(e) => {
                eprintln!("Error WS: {}", e);
                break;
            }
        }
    }
    println!("VS Code desconectado.");
}

#[tokio::main]
async fn main() {
    vision::probar_vision();
    audio::probar_oido();

    let sentinel = Arc::new(Sentinel::new());
    let orchestrator = Arc::new(Mutex::new(Orchestrator::new()));
    let shared_ctx = context::create_shared_context();

    let port = find_available_port(3333);
    write_port_file(port);

    let _ = ctrlc::set_handler(move || {
        cleanup_port_file();
        std::process::exit(0);
    });

    // Estado compartido para Tauri commands
    let app_state = AppState {
        ctx: shared_ctx.clone(),
        sentinel: sentinel.clone(),
        orchestrator: orchestrator.clone(),
        prev_code_hash: Mutex::new(None),
        prev_image_hash: Mutex::new(None),
    };

    let ctx_for_warp = shared_ctx.clone();

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![ask_lia])
        .setup(move |app| {
            let app_handle = app.handle().clone();

            // Conectar Orchestrator al AppHandle
            if let Ok(mut orc) = orchestrator.lock() {
                orc.set_app_handle(app_handle.clone());
            }

            let ctx = ctx_for_warp.clone();

            let ctx_filter = {
                let ctx = ctx.clone();
                warp::any().map(move || ctx.clone())
            };
            let app_filter = {
                let app = app_handle.clone();
                warp::any().map(move || app.clone())
            };

            let ws_route = warp::path("ws")
                .and(warp::ws())
                .and(ctx_filter)
                .and(app_filter)
                .map(|ws: warp::ws::Ws, ctx: SharedContext, app: AppHandle| {
                    ws.on_upgrade(move |socket| handle_ws_client(socket, ctx, app))
                });

            tokio::spawn(async move {
                println!("Servidor Lia en ws://127.0.0.1:{}/ws", port);
                println!("Cloud target: {}", CLOUD_URL);
                warp::serve(ws_route).run(([127, 0, 0, 1], port)).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    cleanup_port_file();
}
