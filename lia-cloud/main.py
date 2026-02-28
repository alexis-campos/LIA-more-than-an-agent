# lia-cloud/main.py
# Servidor principal de Lia Cloud.
# Recibe peticiones multimodales de Rust (Contrato B),
# las procesa con Gemini via Vertex AI, y devuelve
# respuestas en streaming (Contrato C).

import json
import base64
import logging
import uvicorn
from fastapi import FastAPI, WebSocket, WebSocketDisconnect, Query

from config import HOST, PORT, LIA_CLIENT_TOKEN
from cache import LRUCache
from inference import stream_response

# Logging seguro: solo metadatos, nunca contenido de codigo
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(name)s] %(levelname)s: %(message)s",
)
logger = logging.getLogger("lia.server")

app = FastAPI(title="Lia Cloud", version="0.1.0")
cache = LRUCache()


@app.get("/health")
async def health_check():
    """Endpoint para verificar que el servidor esta arriba."""
    return {"status": "ok", "cache_size": cache.size}


@app.websocket("/ws/lia")
async def websocket_endpoint(
    websocket: WebSocket,
    token: str = Query(default=None),
):
    """WebSocket bidireccional para comunicacion con Lia Client (Rust).

    Flujo:
    1. Validar token de autenticacion (Bearer)
    2. Recibir Contrato B (peticion multimodal)
    3. Resolver hashes contra cache LRU
    4. Llamar a Gemini via Vertex AI
    5. Enviar respuesta en streaming (Contrato C)
    """
    # 1. Autenticacion Bearer
    if token != LIA_CLIENT_TOKEN:
        await websocket.close(code=4001, reason="Token invalido")
        logger.warning("Conexion rechazada: token invalido")
        return

    await websocket.accept()
    logger.info("Cliente Rust conectado")

    try:
        while True:
            # 2. Recibir Contrato B
            raw_message = await websocket.receive_text()
            request_data = json.loads(raw_message)

            request_id = request_data.get("request_id", "unknown")
            payload = request_data.get("payload", {})

            logger.info("Procesando peticion %s", request_id)

            # 3. Resolver cache para codigo
            code_data = payload.get("code", {})
            code_hash = code_data.get("hash", "")
            code_content = code_data.get("content")
            code_language = code_data.get("language", "")

            if code_content is not None:
                # Codigo nuevo: guardar en cache
                cache.put(code_hash, code_content)
            else:
                # Smart Caching: recuperar del cache
                code_content = cache.get(code_hash)
                if code_content is None:
                    logger.warning("Cache miss para hash %s", code_hash[:12])

            # 4. Resolver cache para imagen
            vision_data = payload.get("vision", {})
            image_hash = vision_data.get("hash", "")
            image_b64 = vision_data.get("data_b64")
            image_bytes = None

            if image_b64 is not None:
                image_bytes = base64.b64decode(image_b64)
                cache.put(image_hash, image_bytes)
            else:
                image_bytes = cache.get(image_hash)

            # 5. Audio (por ahora solo decodificamos, STT viene en Fase 5)
            audio_data = payload.get("audio", {})
            audio_b64 = audio_data.get("data_b64")
            audio_transcript = None  # Placeholder hasta Fase 5 (STT)

            if audio_b64:
                logger.info("Audio recibido (%d chars b64), STT pendiente Fase 5",
                            len(audio_b64))

            # 6. Llamar a Gemini y enviar streaming (Contrato C)
            try:
                async for text_chunk in stream_response(
                    code=code_content,
                    language=code_language,
                    image_bytes=image_bytes,
                    audio_transcript=audio_transcript,
                ):
                    # Enviar chunk de tipo code_suggestion
                    await websocket.send_json({
                        "request_id": request_id,
                        "stream_status": "in_progress",
                        "chunk_type": "code_suggestion",
                        "data": text_chunk,
                    })

                # Mensaje de cierre
                await websocket.send_json({
                    "request_id": request_id,
                    "stream_status": "completed",
                    "chunk_type": "system",
                    "data": None,
                })
                logger.info("Peticion %s completada", request_id)

            except Exception as e:
                logger.error("Error procesando peticion %s: %s", request_id, str(e))
                await websocket.send_json({
                    "request_id": request_id,
                    "stream_status": "error",
                    "chunk_type": "system",
                    "data": str(e),
                })

    except WebSocketDisconnect:
        logger.info("Cliente Rust desconectado")
    except Exception as e:
        logger.error("Error inesperado en WebSocket: %s", str(e))


if __name__ == "__main__":
    logger.info("Iniciando Lia Cloud en %s:%d", HOST, PORT)
    uvicorn.run(app, host=HOST, port=PORT)
