// lia-client/src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod vision;
mod audio;

use warp::Filter;
use futures_util::StreamExt; // Necesario para leer los mensajes del WebSocket

// Esta función maneja la conexión de cada cliente (VS Code)
async fn handle_ws_client(websocket: warp::ws::WebSocket) {
    println!("¡VS Code se ha conectado a Lia!");
    
    // Dividimos el websocket para poder leer lo que nos envían
    let (_, mut rx) = websocket.split();

    // Escuchamos los mensajes en un bucle
    while let Some(result) = rx.next().await {
        match result {
            Ok(msg) => {
                if let Ok(text) = msg.to_str() {
                    println!("Mensaje recibido de VS Code: {}", text);
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
    // 1. Configuramos la ruta del WebSocket con Warp
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| {
            ws.on_upgrade(move |socket| handle_ws_client(socket))
        });
    // Probamos los sentidos de Lia al arrancar
    vision::probar_vision();
    audio::probar_oido();
    // 2. Levantamos el servidor en un hilo secundario para no bloquear Tauri
    tokio::spawn(async move {
        println!("Servidor local de Lia escuchando en ws://127.0.0.1:3333/ws");
        warp::serve(ws_route).run(([127, 0, 0, 1], 3333)).await;
    });

    // 3. Iniciamos la interfaz gráfica de Tauri
    tauri::Builder::default()
        .setup(|_app| {
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}