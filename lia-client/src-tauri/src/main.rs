// lia-client/src-tauri/src/main.rs
// Punto de entrada principal de Lia Desktop.
//
// Pipeline completo:
// 1. VS Code → context_update → SharedContext
// 2. User clicks "Preguntar a Lia"
// 3. LISTENING → graba microfono (echo cancellation via PlayingFlag)
// 4. THINKING → captura pantalla + Sentinel + build_request (multimodal)
// 5. Envia Contrato B al Cloud Python
// 6. RESPONDING → streaming texto al HUD, TTS audio al speaker
// 7. IDLE → ciclo completado
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
#[allow(dead_code)]
mod wakeword;

use audio::PlayingFlag;
use context::{ContextUpdate, SharedContext};
use futures_util::StreamExt;
use orchestrator::{LiaState, Orchestrator};
use sentinel::Sentinel;
use serde::Serialize;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use warp::Filter;

/// URL del Cloud Python.
const CLOUD_URL: &str = "ws://127.0.0.1:8000/ws/lia?token=lia-dev-token-2024";

/// Duracion de grabacion en segundos.
const RECORD_DURATION_SECS: u64 = 4;

/// Info de contexto para el frontend React.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ContextInfo {
    file_name: String,
    file_path: String,
    language: String,
    cursor_line: u32,
    workspace: String,
}

/// Estado compartido para los Tauri commands (todo Send + Sync).
struct AppState {
    ctx: SharedContext,
    sentinel: Arc<Sentinel>,
    orchestrator: Arc<Mutex<Orchestrator>>,
    playing_flag: PlayingFlag,
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
    println!("Puerto Lia: {}", port);
}

fn cleanup_port_file() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let _ = std::fs::remove_file(std::path::PathBuf::from(&home).join(".lia").join("port"));
}

/// Pipeline completo de inferencia.
/// Todo este future es Send (AudioPlayer se crea en spawn_blocking).
async fn trigger_inference(
    ctx: &SharedContext,
    app: &AppHandle,
    sentinel: &Sentinel,
    orchestrator: &Arc<Mutex<Orchestrator>>,
    playing_flag: &PlayingFlag,
    prev_code_hash: &Mutex<Option<String>>,
    prev_image_hash: &Mutex<Option<String>>,
) {
    // ── 1. Leer contexto del editor ──
    let context_data = {
        let lock = ctx.lock().unwrap();
        lock.clone()
    };

    let context_data = match context_data {
        Some(data) => data,
        None => {
            let _ = app.emit(
                "lia://stream-chunk",
                "No hay contexto. Abre un archivo en VS Code con la extension de Lia activa."
                    .to_string(),
            );
            let _ = app.emit("lia://stream-end", ());
            return;
        }
    };

    println!(
        "Contexto: {} ({}) linea={} ws={}",
        context_data.file_context.file_name,
        context_data.file_context.language,
        context_data.file_context.cursor_line,
        context_data.workspace_name
    );

    // ── 2. LISTENING: Grabar audio del microfono ──
    if let Ok(mut orc) = orchestrator.lock() {
        orc.start_listening();
    }

    let pf = playing_flag.clone();
    let audio_data = tokio::task::spawn_blocking(move || match audio::start_recording(pf) {
        Ok(recorder) => {
            println!("Grabando {}s...", RECORD_DURATION_SECS);
            std::thread::sleep(std::time::Duration::from_secs(RECORD_DURATION_SECS));
            audio::stop_recording(recorder).unwrap_or_default()
        }
        Err(e) => {
            eprintln!("Mic error: {}", e);
            vec![]
        }
    })
    .await
    .unwrap_or_default();

    if !audio_data.is_empty() {
        println!("Audio grabado: {} bytes WAV", audio_data.len());
    }

    // ── 3. THINKING: pantalla + Sentinel + empaquetar ──
    if let Ok(mut orc) = orchestrator.lock() {
        orc.start_thinking();
    }

    let image_data = vision::capture_screen().unwrap_or_else(|e| {
        eprintln!("Vision error: {}", e);
        vec![]
    });

    let prev_c = prev_code_hash.lock().unwrap().clone();
    let prev_i = prev_image_hash.lock().unwrap().clone();

    let req = request::build_request(
        sentinel,
        &context_data.file_context.content_window,
        &context_data.file_context.language,
        &image_data,
        &audio_data,
        prev_c.as_deref(),
        prev_i.as_deref(),
    );

    *prev_code_hash.lock().unwrap() = Some(req.payload.code.hash.clone());
    *prev_image_hash.lock().unwrap() = Some(req.payload.vision.hash.clone());

    let request_json = serde_json::to_string(&req).unwrap();
    println!(
        "Contrato B: id={} code={} img={} audio={} ({}B)",
        req.request_id,
        req.payload.code.content.is_some(),
        req.payload.vision.data_b64.is_some(),
        req.payload.audio.data_b64.is_some(),
        request_json.len()
    );

    // ── 4. Limpiar HUD ──
    let _ = app.emit("lia://stream-clear", ());

    // ── 5. RESPONDING: Cloud → HUD streaming ──
    if let Ok(mut orc) = orchestrator.lock() {
        orc.start_responding();
    }

    let stream_result = cloud_client::send_to_cloud_and_stream(CLOUD_URL, &request_json, app).await;

    match stream_result {
        Ok(result) => {
            println!(
                "Stream completado, {} chunks de TTS",
                result.tts_audio.len()
            );

            // ── 6. Reproducir TTS en un thread bloqueante (AudioPlayer no es Send) ──
            if !result.tts_audio.is_empty() {
                let pf_play = playing_flag.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    match playback::AudioPlayer::new(pf_play) {
                        Ok(player) => {
                            for chunk in &result.tts_audio {
                                if let Err(e) = player.play_chunk(chunk) {
                                    eprintln!("TTS playback error: {}", e);
                                }
                            }
                            // Esperar a que termine la reproduccion
                            while player.is_playing() {
                                std::thread::sleep(std::time::Duration::from_millis(100));
                            }
                            player.stop();
                            println!("TTS playback completado");
                        }
                        Err(e) => eprintln!("AudioPlayer error: {}", e),
                    }
                })
                .await;
            }
        }
        Err(e) => {
            eprintln!("Error Cloud: {}", e);
            let _ = app.emit(
                "lia://stream-chunk",
                format!(
                    "[ERROR] {}\n\nAsegurate de que lia-cloud este corriendo.",
                    e
                ),
            );
        }
    }

    // ── 7. IDLE ──
    if let Ok(mut orc) = orchestrator.lock() {
        orc.finish();
    }
}

/// Comando Tauri: boton del HUD dispara inferencia.
#[tauri::command]
async fn ask_lia(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<String, String> {
    {
        let orc = state.orchestrator.lock().unwrap();
        if orc.state() != LiaState::Idle {
            return Ok("busy".to_string());
        }
    }

    println!("\n=== Ciclo de inferencia ===");

    trigger_inference(
        &state.ctx,
        &app,
        &state.sentinel,
        &state.orchestrator,
        &state.playing_flag,
        &state.prev_code_hash,
        &state.prev_image_hash,
    )
    .await;

    println!("=== Fin del ciclo ===\n");
    Ok("ok".to_string())
}

/// Maneja la conexion WebSocket de VS Code.
async fn handle_ws_client(websocket: warp::ws::WebSocket, ctx: SharedContext, app: AppHandle) {
    println!("VS Code conectado.");

    let (_, mut rx) = websocket.split();

    while let Some(result) = rx.next().await {
        match result {
            Ok(msg) => {
                if let Ok(text) = msg.to_str() {
                    if let Ok(update) = serde_json::from_str::<ContextUpdate>(text) {
                        println!(
                            "[{}] {} L{} ({})",
                            update.event_type,
                            update.file_context.file_name,
                            update.file_context.cursor_line,
                            update.file_context.language
                        );

                        let _ = app.emit(
                            "lia://context-update",
                            ContextInfo {
                                file_name: update.file_context.file_name.clone(),
                                file_path: update.file_context.file_path.clone(),
                                language: update.file_context.language.clone(),
                                cursor_line: update.file_context.cursor_line,
                                workspace: update.workspace_name.clone(),
                            },
                        );

                        if let Ok(mut lock) = ctx.lock() {
                            *lock = Some(update);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("WS error: {}", e);
                break;
            }
        }
    }
    println!("VS Code desconectado.");
}

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════╗");
    println!("║        Lia Desktop v0.1.0            ║");
    println!("║     More Than an Agent               ║");
    println!("╚══════════════════════════════════════╝");

    vision::probar_vision();
    audio::probar_oido();

    let sentinel = Arc::new(Sentinel::new());
    println!("Sentinel DLP activo");

    let orchestrator = Arc::new(Mutex::new(Orchestrator::new()));
    let shared_ctx = context::create_shared_context();
    let playing_flag = audio::create_playing_flag();
    println!("Echo cancellation listo");

    let port = find_available_port(3333);
    write_port_file(port);

    let _ = ctrlc::set_handler(move || {
        cleanup_port_file();
        std::process::exit(0);
    });

    let app_state = AppState {
        ctx: shared_ctx.clone(),
        sentinel: sentinel.clone(),
        orchestrator: orchestrator.clone(),
        playing_flag,
        prev_code_hash: Mutex::new(None),
        prev_image_hash: Mutex::new(None),
    };

    let ctx_for_warp = shared_ctx.clone();

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![ask_lia])
        .setup(move |app| {
            let app_handle = app.handle().clone();

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
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("  WS: ws://127.0.0.1:{}/ws", port);
                println!("  Cloud: {}", CLOUD_URL);
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                warp::serve(ws_route).run(([127, 0, 0, 1], port)).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    cleanup_port_file();
}
