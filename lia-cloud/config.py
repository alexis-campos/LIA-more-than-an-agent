# lia-cloud/config.py
# Configuracion centralizada del servidor Lia Cloud.
# Lee variables de entorno con valores por defecto para desarrollo local.

import os

# Google Cloud / Vertex AI
GCP_PROJECT_ID = os.getenv("GCP_PROJECT_ID", "lia-ai-488723")
GCP_LOCATION = os.getenv("GCP_LOCATION", "us-central1")
GEMINI_MODEL = os.getenv("GEMINI_MODEL", "gemini-1.5-pro")

# Autenticacion: token que Rust envia en los headers para validar la conexion
LIA_CLIENT_TOKEN = os.getenv("LIA_CLIENT_TOKEN", "lia-dev-token-2024")

# Cache LRU: tiempo de vida en segundos (15 minutos por defecto)
CACHE_TTL_SECONDS = int(os.getenv("CACHE_TTL_SECONDS", "900"))
CACHE_MAX_ENTRIES = int(os.getenv("CACHE_MAX_ENTRIES", "50"))

# Servidor
HOST = os.getenv("LIA_HOST", "0.0.0.0")
PORT = int(os.getenv("LIA_PORT", "8000"))
