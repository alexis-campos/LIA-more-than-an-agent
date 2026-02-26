// lia-client/src-tauri/src/request.rs
// Modulo de empaquetado: construye la peticion multimodal (Contrato B)
// que viaja de Rust a Python cuando el usuario pide ayuda a Lia.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::Serialize;
use uuid::Uuid;

use crate::hasher;
use crate::sentinel::Sentinel;

// ---------------------------------------------------------------------------
// Estructuras del Contrato B
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct AudioPayload {
    pub format: String,
    /// Audio codificado en base64. Puede ser null si no hay audio.
    pub data_b64: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VisionPayload {
    pub hash: String,
    /// Imagen codificada en base64. Es null si el hash no cambio (Smart Caching).
    pub data_b64: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CodePayload {
    pub hash: String,
    pub language: String,
    /// Codigo ya sanitizado por Sentinel. Es null si el hash no cambio.
    pub content: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MultimodalPayload {
    pub audio: AudioPayload,
    pub vision: VisionPayload,
    pub code: CodePayload,
}

#[derive(Debug, Serialize)]
pub struct MultimodalRequest {
    pub request_id: String,
    pub action: String,
    pub payload: MultimodalPayload,
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Construye una peticion multimodal completa (Contrato B).
///
/// Aplica Sentinel al codigo, calcula hashes para Smart Caching,
/// y codifica los datos binarios en base64.
///
/// Parametros:
/// - `sentinel`: Instancia de Sentinel para sanitizar el codigo
/// - `code_content`: Texto crudo del editor (content_window del Contrato A)
/// - `language`: Lenguaje de programacion detectado por VS Code
/// - `image_data`: Bytes de la captura de pantalla (puede estar vacio)
/// - `audio_data`: Bytes del audio grabado (puede estar vacio)
/// - `prev_code_hash`: Hash del codigo enviado anteriormente (para Smart Caching)
/// - `prev_image_hash`: Hash de la imagen enviada anteriormente (para Smart Caching)
pub fn build_request(
    sentinel: &Sentinel,
    code_content: &str,
    language: &str,
    image_data: &[u8],
    audio_data: &[u8],
    prev_code_hash: Option<&str>,
    prev_image_hash: Option<&str>,
) -> MultimodalRequest {
    // 1. Sanitizar el codigo con Sentinel
    let sanitized_code = sentinel.sanitize(code_content);

    // 2. Calcular hashes
    let code_hash = hasher::compute_sha256(&sanitized_code);
    let image_hash = hasher::compute_sha256_bytes(image_data);

    // 3. Smart Caching: solo enviamos el contenido si el hash cambio
    let code_payload = CodePayload {
        hash: code_hash.clone(),
        language: language.to_string(),
        content: if prev_code_hash == Some(code_hash.as_str()) {
            None // El codigo no cambio, Python usara su cache
        } else {
            Some(sanitized_code)
        },
    };

    let vision_payload = VisionPayload {
        hash: image_hash.clone(),
        data_b64: if prev_image_hash == Some(image_hash.as_str()) {
            None // La imagen no cambio, Python usara su cache
        } else if image_data.is_empty() {
            None
        } else {
            Some(BASE64.encode(image_data))
        },
    };

    let audio_payload = AudioPayload {
        format: "wav".to_string(),
        data_b64: if audio_data.is_empty() {
            None
        } else {
            Some(BASE64.encode(audio_data))
        },
    };

    // 4. Armar la peticion completa
    MultimodalRequest {
        request_id: format!("req-{}", &Uuid::new_v4().to_string()[..8]),
        action: "multimodal_inference".to_string(),
        payload: MultimodalPayload {
            audio: audio_payload,
            vision: vision_payload,
            code: code_payload,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_request_basico() {
        let sentinel = Sentinel::new();
        let req = build_request(
            &sentinel,
            "SELECT * FROM rooms",
            "sql",
            &[],
            &[],
            None,
            None,
        );

        assert!(req.request_id.starts_with("req-"));
        assert_eq!(req.action, "multimodal_inference");
        assert_eq!(req.payload.code.language, "sql");
        assert!(req.payload.code.content.is_some());
        assert!(req.payload.vision.data_b64.is_none());
        assert!(req.payload.audio.data_b64.is_none());
    }

    #[test]
    fn test_smart_caching_codigo() {
        let sentinel = Sentinel::new();
        let code = "function hello() { return 1; }";

        // Primera peticion: contenido incluido
        let req1 = build_request(&sentinel, code, "js", &[], &[], None, None);
        assert!(req1.payload.code.content.is_some());

        // Segunda peticion con el mismo hash: contenido omitido (cache)
        let hash = &req1.payload.code.hash;
        let req2 = build_request(&sentinel, code, "js", &[], &[], Some(hash), None);
        assert!(
            req2.payload.code.content.is_none(),
            "Deberia ser None por Smart Caching"
        );
    }

    #[test]
    fn test_sanitizacion_integrada() {
        let sentinel = Sentinel::new();
        let code = r#"$key = "AKIAIOSFODNN7EXAMPLE"; $query = "SELECT 1";"#;

        let req = build_request(&sentinel, code, "php", &[], &[], None, None);
        let content = req.payload.code.content.unwrap();
        assert!(
            !content.contains("AKIAIOSFODNN7EXAMPLE"),
            "La AWS key no fue sanitizada"
        );
        assert!(content.contains("<SECRET_REDACTED>"));
    }
}
