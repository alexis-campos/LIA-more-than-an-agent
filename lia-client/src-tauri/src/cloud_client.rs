// lia-client/src-tauri/src/cloud_client.rs
// Cliente WebSocket que conecta Rust con el Cloud Python (lia-cloud).
// Envia Contrato B (peticion multimodal) y recibe Contrato C (streaming).

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tauri::{AppHandle, Emitter};
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Chunk del Contrato C recibido del Cloud.
#[derive(Debug, Deserialize)]
pub struct ContractCChunk {
    pub request_id: String,
    pub stream_status: String,
    pub chunk_type: String,
    pub data: Option<String>,
}

/// Envia un Contrato B al Cloud y retransmite los chunks de Contrato C al HUD.
///
/// Flujo:
/// 1. Conectar al WebSocket del Cloud Python
/// 2. Enviar el JSON del Contrato B
/// 3. Recibir chunks de Contrato C en streaming
/// 4. Emitir cada chunk como evento Tauri al frontend React
pub async fn send_to_cloud_and_stream(
    cloud_url: &str,
    contract_b_json: &str,
    app: &AppHandle,
) -> Result<(), String> {
    // 1. Conectar al Cloud
    let (ws_stream, _) = connect_async(cloud_url)
        .await
        .map_err(|e| format!("Error conectando al Cloud: {}", e))?;

    println!("Conectado al Cloud en {}", cloud_url);

    let (mut write, mut read) = ws_stream.split();

    // 2. Enviar Contrato B
    write
        .send(Message::Text(contract_b_json.to_string()))
        .await
        .map_err(|e| format!("Error enviando Contrato B: {}", e))?;

    println!("Contrato B enviado al Cloud");

    // 3. Recibir chunks de Contrato C
    while let Some(msg_result) = read.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<ContractCChunk>(&text) {
                    Ok(chunk) => {
                        match chunk.stream_status.as_str() {
                            "in_progress" => {
                                if chunk.chunk_type == "code_suggestion" {
                                    if let Some(ref data) = chunk.data {
                                        // Emitir texto al HUD
                                        let _ = app.emit("lia://stream-chunk", data.clone());
                                    }
                                }
                                // chunk_type "audio" se podria reproducir aqui
                                // pero para la demo omitimos TTS
                            }
                            "completed" => {
                                println!("Stream completado para {}", chunk.request_id);
                                let _ = app.emit("lia://stream-end", ());
                                break;
                            }
                            "error" => {
                                let error_msg = chunk
                                    .data
                                    .unwrap_or_else(|| "Error desconocido".to_string());
                                eprintln!("Error del Cloud: {}", error_msg);
                                let _ = app
                                    .emit("lia://stream-chunk", format!("[ERROR] {}", error_msg));
                                let _ = app.emit("lia://stream-end", ());
                                break;
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        eprintln!("Error parseando Contrato C: {}", e);
                    }
                }
            }
            Ok(Message::Close(_)) => {
                println!("Cloud cerro la conexion");
                break;
            }
            Err(e) => {
                eprintln!("Error en WebSocket del Cloud: {}", e);
                break;
            }
            _ => {}
        }
    }

    // Cerrar la conexion
    let _ = write.close().await;
    Ok(())
}
