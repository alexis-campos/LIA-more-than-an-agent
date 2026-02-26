// lia-client/src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod audio;
mod context;
mod hasher;
mod request;
mod sentinel;
mod vision;

use context::{ContextUpdate, SharedContext};
use futures_util::StreamExt;
use warp::Filter;

/// Maneja la conexión WebSocket de cada cliente (VS Code).
/// Parsea los mensajes entrantes como ContextUpdate (Contrato A) y
/// actualiza la memoria compartida para que otros hilos la lean.
async fn handle_ws_client(websocket: warp::ws::WebSocket, ctx: SharedContext) {
    println!("¡VS Code se ha conectado a Lia!");

    let (_, mut rx) = websocket.split();

    while let Some(result) = rx.next().await {
        match result {
            Ok(msg) => {
                if let Ok(text) = msg.to_str() {
                    // Intentamos parsear el JSON como un ContextUpdate (Contrato A)
                    match serde_json::from_str::<ContextUpdate>(text) {
                        Ok(update) => {
                            println!(
                                "Contexto actualizado: archivo=\"{}\", línea={}, lenguaje=\"{}\"",
                                update.file_context.file_name,
                                update.file_context.cursor_line,
                                update.file_context.language
                            );
                            // Guardamos el contexto en la memoria compartida
                            if let Ok(mut lock) = ctx.lock() {
                                *lock = Some(update);
                            }
                        }
                        Err(_) => {
                            // Si no es un ContextUpdate, lo mostramos como texto plano
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

    // 2. Configuramos la ruta del WebSocket con Warp
    //    Inyectamos el SharedContext en cada conexión nueva mediante un filtro
    let ctx_filter = {
        let ctx = shared_ctx.clone();
        warp::any().map(move || ctx.clone())
    };

    let ws_route = warp::path("ws").and(warp::ws()).and(ctx_filter).map(
        |ws: warp::ws::Ws, ctx: SharedContext| {
            ws.on_upgrade(move |socket| handle_ws_client(socket, ctx))
        },
    );

    // 3. Levantamos el servidor en un hilo secundario para no bloquear Tauri
    tokio::spawn(async move {
        println!("Servidor local de Lia escuchando en ws://127.0.0.1:3333/ws");
        warp::serve(ws_route).run(([127, 0, 0, 1], 3333)).await;
    });

    // 4. Iniciamos la interfaz gráfica de Tauri
    tauri::Builder::default()
        .setup(|_app| Ok(()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
