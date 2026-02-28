// lia-client/src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod audio;
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
use serde::Serialize;
use std::net::TcpListener;
use tauri::{AppHandle, Emitter};
use warp::Filter;

/// Info de contexto que se envia al frontend React.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ContextInfo {
    file_name: String,
    language: String,
    cursor_line: u32,
}

/// Encuentra un puerto disponible. Intenta el preferido primero.
fn find_available_port(preferred: u16) -> u16 {
    // Intentar el puerto preferido
    if TcpListener::bind(("127.0.0.1", preferred)).is_ok() {
        return preferred;
    }
    // Pedir al OS un puerto libre
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|addr| addr.port())
        .unwrap_or(preferred)
}

/// Escribe el puerto en ~/.lia/port para que la extension lo descubra.
fn write_port_file(port: u16) {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let lia_dir = std::path::PathBuf::from(&home).join(".lia");
    let _ = std::fs::create_dir_all(&lia_dir);
    let port_file = lia_dir.join("port");
    let _ = std::fs::write(&port_file, port.to_string());
    println!("Puerto escrito en {:?}", port_file);
}

/// Elimina el archivo de puerto al cerrar.
fn cleanup_port_file() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let port_file = std::path::PathBuf::from(&home).join(".lia").join("port");
    let _ = std::fs::remove_file(&port_file);
}

/// Maneja la conexion WebSocket de cada cliente (VS Code).
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
                                "Contexto actualizado: archivo=\"{}\", linea={}, lenguaje=\"{}\"",
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
                        Err(_) => {
                            println!("Mensaje recibido de VS Code: {}", text);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error en el WebSocket: {}", e);
                break;
            }
        }
    }
    println!("VS Code se ha desconectado.");
}

#[tokio::main]
async fn main() {
    // Probamos los sentidos de Lia al arrancar (Fase 1)
    vision::probar_vision();
    audio::probar_oido();

    // Contexto compartido (Fase 2)
    let shared_ctx = context::create_shared_context();

    // Dynamic port discovery (Fase 7)
    let port = find_available_port(3333);
    write_port_file(port);

    // Cleanup al cerrar (Ctrl+C)
    let _ = ctrlc::set_handler(move || {
        cleanup_port_file();
        std::process::exit(0);
    });

    // Iniciar Tauri con el servidor WebSocket integrado
    tauri::Builder::default()
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let ctx = shared_ctx.clone();

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
                println!(
                    "Servidor local de Lia escuchando en ws://127.0.0.1:{}/ws",
                    port
                );
                warp::serve(ws_route).run(([127, 0, 0, 1], port)).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    // Cleanup al salir normalmente
    cleanup_port_file();
}
