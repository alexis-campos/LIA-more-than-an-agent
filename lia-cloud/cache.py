# lia-cloud/cache.py
# Cache LRU volatil en memoria RAM con TTL.
# Nunca escribe a disco (regla del modelo de privacidad).
# Almacena codigo e imagenes indexados por su SHA-256.

import time
from collections import OrderedDict
from config import CACHE_TTL_SECONDS, CACHE_MAX_ENTRIES


class LRUCache:
    """Cache LRU con expiracion por tiempo (TTL).

    Reglas de seguridad:
    - Vive estrictamente en la memoria RAM del proceso Python.
    - Entradas expiran automaticamente despues de CACHE_TTL_SECONDS.
    - Nunca se serializa ni se escribe a disco.
    """

    def __init__(self, max_entries: int = CACHE_MAX_ENTRIES, ttl: int = CACHE_TTL_SECONDS):
        self._store: OrderedDict[str, tuple[float, any]] = OrderedDict()
        self._max_entries = max_entries
        self._ttl = ttl

    def get(self, hash_key: str):
        """Busca contenido por hash. Retorna None si no existe o expiro."""
        if hash_key not in self._store:
            return None

        timestamp, content = self._store[hash_key]

        # Verificar TTL
        if time.time() - timestamp > self._ttl:
            del self._store[hash_key]
            return None

        # Mover al final (mas reciente) para mantener orden LRU
        self._store.move_to_end(hash_key)
        return content

    def put(self, hash_key: str, content) -> None:
        """Almacena contenido asociado a un hash SHA-256."""
        # Si ya existe, actualizar
        if hash_key in self._store:
            self._store.move_to_end(hash_key)
            self._store[hash_key] = (time.time(), content)
            return

        # Si estamos al limite, eliminar el mas antiguo (LRU)
        if len(self._store) >= self._max_entries:
            self._store.popitem(last=False)

        self._store[hash_key] = (time.time(), content)

    def cleanup_expired(self) -> int:
        """Elimina entradas expiradas. Retorna cantidad eliminada."""
        now = time.time()
        expired_keys = [
            k for k, (ts, _) in self._store.items()
            if now - ts > self._ttl
        ]
        for key in expired_keys:
            del self._store[key]
        return len(expired_keys)

    @property
    def size(self) -> int:
        return len(self._store)
