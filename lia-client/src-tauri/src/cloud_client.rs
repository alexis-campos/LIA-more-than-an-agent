// lia-client/src-tauri/src/cloud_client.rs
// Cliente WebSocket que conecta Rust con el Cloud Python (lia-cloud).
// Envia Contrato B y recibe Contrato C (streaming texto + audio TTS).

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
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

/// Resultado del streaming desde el Cloud.
pub struct StreamResult {
    /// Audio TTS acumulado (bytes WAV/MP3 del Cloud, para reproducir despues).
    pub tts_audio: Vec<Vec<u8>>,
}

/// Envia Contrato B y retransmite texto al HUD.
/// Retorna los chunks de audio TTS para reproduccion posterior.
pub async fn send_to_cloud_and_stream(
    cloud_url: &str,
    contract_b_json: &str,
    app: &AppHandle,
) -> Result<StreamResult, String> {
    let (ws_stream, _) = connect_async(cloud_url)
        .await
        .map_err(|e| format!("Error conectando al Cloud: {}", e))?;

    println!("Conectado al Cloud");

    let (mut write, mut read) = ws_stream.split();

    write
        .send(Message::Text(contract_b_json.to_string()))
        .await
        .map_err(|e| format!("Error enviando Contrato B: {}", e))?;

    println!("Contrato B enviado");

    let mut tts_audio: Vec<Vec<u8>> = Vec::new();

    while let Some(msg_result) = read.next().await {
        match msg_result {
            Ok(Message::Text(text)) => match serde_json::from_str::<ContractCChunk>(&text) {
                Ok(chunk) => match chunk.stream_status.as_str() {
                    "in_progress" => match chunk.chunk_type.as_str() {
                        "code_suggestion" => {
                            if let Some(ref data) = chunk.data {
                                let _ = app.emit("lia://stream-chunk", data.clone());
                            }
                        }
                        "audio" => {
                            if let Some(ref data) = chunk.data {
                                match BASE64.decode(data) {
                                    Ok(audio_bytes) => {
                                        println!("TTS audio: {} bytes", audio_bytes.len());
                                        tts_audio.push(audio_bytes);
                                    }
                                    Err(e) => eprintln!("Error base64 audio: {}", e),
                                }
                            }
                        }
                        _ => {}
                    },
                    "completed" => {
                        println!("Stream completado: {}", chunk.request_id);
                        break;
                    }
                    "error" => {
                        let msg = chunk
                            .data
                            .unwrap_or_else(|| "Error desconocido".to_string());
                        eprintln!("Error Cloud: {}", msg);
                        let _ = app.emit("lia://stream-chunk", format!("[ERROR] {}", msg));
                        break;
                    }
                    _ => {}
                },
                Err(e) => eprintln!("Error parseando Contrato C: {}", e),
            },
            Ok(Message::Close(_)) => break,
            Err(e) => {
                eprintln!("Error WS Cloud: {}", e);
                break;
            }
            _ => {}
        }
    }

    let _ = write.close().await;
    Ok(StreamResult { tts_audio })
}
