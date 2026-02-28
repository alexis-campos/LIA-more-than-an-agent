# lia-cloud/stt.py
# Speech-to-Text: transcribe audio WAV a texto usando Google Cloud Speech-to-Text.

import logging
from google.cloud import speech

from config import GCP_PROJECT_ID

logger = logging.getLogger("lia.stt")


async def transcribe(audio_bytes: bytes, sample_rate: int = 16000) -> str:
    """Transcribe audio WAV a texto.

    Usa Google Cloud Speech-to-Text con deteccion automatica de idioma
    (espanol e ingles).

    Args:
        audio_bytes: Audio en formato WAV (PCM 16-bit, mono)
        sample_rate: Frecuencia de muestreo en Hz

    Returns:
        Texto transcrito del audio, o string vacio si no se detecto habla.
    """
    client = speech.SpeechClient()

    audio = speech.RecognitionAudio(content=audio_bytes)

    config = speech.RecognitionConfig(
        encoding=speech.RecognitionConfig.AudioEncoding.LINEAR16,
        sample_rate_hertz=sample_rate,
        language_code="es-419",  # Espanol latinoamericano
        alternative_language_codes=["en-US"],  # Fallback a ingles
        model="latest_long",
        enable_automatic_punctuation=True,
    )

    logger.info("Enviando audio a Speech-to-Text (%d bytes)", len(audio_bytes))

    try:
        response = client.recognize(config=config, audio=audio)

        if not response.results:
            logger.info("STT no detecto habla en el audio")
            return ""

        # Concatenar todas las transcripciones
        transcript = " ".join(
            result.alternatives[0].transcript
            for result in response.results
            if result.alternatives
        )

        logger.info("STT transcripcion: '%s'", transcript[:100])
        return transcript

    except Exception as e:
        logger.error("Error en Speech-to-Text: %s", str(e))
        return ""
