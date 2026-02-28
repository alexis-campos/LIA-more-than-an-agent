// lia-client/src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod audio;
mod context;
mod hasher;
mod playback;
mod request;
mod sentinel;
mod vision;

use context::{ContextUpdate, SharedContext};
use futures_util::StreamExt;
use serde::Serialize;
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

/// Maneja la conexion WebSocket de cada cliente (VS Code).
/// Parsea los mensajes entrantes como ContextUpdate (Contrato A),
/// actualiza la memoria compartida y emite eventos al frontend.
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

                            // Emitir evento al frontend React
                            let _ = app.emit(
                                "lia://context-update",
                                ContextInfo {
                                    file_name: update.file_context.file_name.clone(),
                                    language: update.file_context.language.clone(),
                                    cursor_line: update.file_context.cursor_line,
                                },
                            );

                            // Guardar en memoria compartida
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

    // 1. Creamos el contexto compartido (Fase 2)
    let shared_ctx = context::create_shared_context();

    // 4. Iniciamos Tauri con el servidor WebSocket integrado
    tauri::Builder::default()
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let ctx = shared_ctx.clone();

            // 2. Configuramos la ruta del WebSocket con Warp
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

            // 3. Levantamos el servidor en un hilo secundario
            tokio::spawn(async move {
                println!("Servidor local de Lia escuchando en ws://127.0.0.1:3333/ws");
                warp::serve(ws_route).run(([127, 0, 0, 1], 3333)).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
