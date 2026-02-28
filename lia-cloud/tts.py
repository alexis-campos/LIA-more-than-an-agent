# lia-cloud/tts.py
# Text-to-Speech: convierte texto a audio usando Google Cloud Text-to-Speech.

import logging
from google.cloud import texttospeech

logger = logging.getLogger("lia.tts")


async def synthesize(text: str) -> bytes:
    """Convierte texto a audio WAV.

    Usa una voz femenina en espanol latinoamericano con modelo WaveNet
    para calidad natural.

    Args:
        text: Texto a convertir en voz.

    Returns:
        Bytes de audio en formato WAV (LINEAR16).
    """
    if not text or not text.strip():
        return b""

    client = texttospeech.TextToSpeechClient()

    synthesis_input = texttospeech.SynthesisInput(text=text)

    voice = texttospeech.VoiceSelectionParams(
        language_code="es-419",  # Espanol latinoamericano
        name="es-US-Wavenet-A",  # Voz femenina WaveNet
        ssml_gender=texttospeech.SsmlVoiceGender.FEMALE,
    )

    audio_config = texttospeech.AudioConfig(
        audio_encoding=texttospeech.AudioEncoding.LINEAR16,
        sample_rate_hertz=24000,
    )

    logger.info("Generando TTS (%d chars)", len(text))

    try:
        response = client.synthesize_speech(
            input=synthesis_input,
            voice=voice,
            audio_config=audio_config,
        )

        logger.info("TTS generado: %d bytes de audio", len(response.audio_content))
        return response.audio_content

    except Exception as e:
        logger.error("Error en Text-to-Speech: %s", str(e))
        return b""
