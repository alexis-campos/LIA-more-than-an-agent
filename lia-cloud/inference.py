# lia-cloud/inference.py
# Motor de inferencia: construye prompts multimodales y llama a Gemini 1.5 Pro
# via Vertex AI con streaming.

import base64
import logging
from typing import AsyncGenerator

from google import genai
from google.genai.types import Content, Part, GenerateContentConfig

from config import GCP_PROJECT_ID, GCP_LOCATION, GEMINI_MODEL

logger = logging.getLogger("lia.inference")

# System Prompt que instruye a Gemini sobre como interpretar los datos multimodales.
# Regla clave: el texto del codigo es la fuente de verdad, no la imagen.
SYSTEM_PROMPT = """Eres Lia, una asistente de programacion experta y amigable. 
Tu rol es actuar como un "Pair Programmer" que puede ver la pantalla del usuario, 
escuchar su voz y leer su codigo en tiempo real.

REGLAS CRITICAS:
1. Tu fuente de verdad para el codigo es el TEXTO provisto, NO la imagen. 
   La imagen es solo para entender el error visual o la GUI del navegador.
   Si hay discrepancia entre la imagen y el texto del codigo, confia en el texto.
2. Nunca reveles secretos redactados. Si ves <SECRET_REDACTED> en el codigo, 
   ignoralo y no intentes adivinar el valor original.
3. Se concisa pero precisa. El usuario esta programando y necesita respuestas 
   accionables, no ensayos.
4. Si sugieres codigo, usa el lenguaje de programacion que el usuario esta usando.
5. Responde en el mismo idioma que el usuario habla."""


def _create_client() -> genai.Client:
    """Crea el cliente de Vertex AI para Gemini."""
    return genai.Client(
        vertexai=True,
        project=GCP_PROJECT_ID,
        location=GCP_LOCATION,
    )


def build_prompt_parts(
    code: str | None,
    language: str | None,
    image_bytes: bytes | None,
    audio_transcript: str | None,
) -> list[Part]:
    """Construye las partes del prompt multimodal para Gemini.

    Combina texto del codigo, imagen de la pantalla y transcripcion
    de audio en un formato que Gemini pueda procesar.
    """
    parts: list[Part] = []

    # Contexto de codigo
    if code:
        lang_label = language or "desconocido"
        parts.append(Part.from_text(
            text=f"CODIGO ACTUAL DEL USUARIO (lenguaje: {lang_label}):\n```{lang_label}\n{code}\n```"
        ))

    # Captura de pantalla
    if image_bytes:
        parts.append(Part.from_bytes(
            data=image_bytes,
            mime_type="image/png",
        ))
        parts.append(Part.from_text(
            text="La imagen anterior es una captura de la pantalla del usuario."
        ))

    # Transcripcion de voz (lo que el usuario dijo)
    if audio_transcript:
        parts.append(Part.from_text(
            text=f"EL USUARIO DIJO: \"{audio_transcript}\""
        ))
    else:
        parts.append(Part.from_text(
            text="El usuario activo a Lia pero no dijo nada especifico. "
                 "Analiza el codigo y la pantalla para ofrecer ayuda proactiva."
        ))

    return parts


async def stream_response(
    code: str | None = None,
    language: str | None = None,
    image_bytes: bytes | None = None,
    audio_transcript: str | None = None,
) -> AsyncGenerator[str, None]:
    """Llama a Gemini y hace yield de chunks de texto a medida que llegan.

    Este es un async generator que permite enviar cada fragmento de respuesta
    al cliente Rust en tiempo real via WebSocket (Contrato C).
    """
    client = _create_client()
    parts = build_prompt_parts(code, language, image_bytes, audio_transcript)

    config = GenerateContentConfig(
        system_instruction=SYSTEM_PROMPT,
        temperature=0.7,
        max_output_tokens=2048,
    )

    logger.info("Enviando prompt multimodal a Gemini (%d partes)", len(parts))

    try:
        async for chunk in client.aio.models.generate_content_stream(
            model=GEMINI_MODEL,
            contents=[Content(role="user", parts=parts)],
            config=config,
        ):
            if chunk.text:
                yield chunk.text
    except Exception as e:
        logger.error("Error en la llamada a Vertex AI: %s", str(e))
        yield f"[ERROR] No pude conectar con Gemini: {str(e)}"
