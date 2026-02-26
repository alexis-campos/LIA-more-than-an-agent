// lia-client/src-tauri/src/hasher.rs
// Modulo de hashing: calcula SHA-256 para el sistema de Smart Caching.
// Permite comparar si el codigo o la imagen han cambiado desde el ultimo envio,
// evitando retransmitir datos identicos a la nube.

use sha2::{Digest, Sha256};

/// Calcula el SHA-256 de un contenido y lo retorna como string hexadecimal.
/// Se usa para generar los campos `hash` del Contrato B.
pub fn compute_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

/// Calcula el SHA-256 de datos binarios (imagenes, audio).
pub fn compute_sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{:x}", result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_determinista() {
        let hash1 = compute_sha256("SELECT * FROM rooms");
        let hash2 = compute_sha256("SELECT * FROM rooms");
        assert_eq!(
            hash1, hash2,
            "El mismo contenido debe producir el mismo hash"
        );
    }

    #[test]
    fn test_hash_cambia_con_contenido_diferente() {
        let hash1 = compute_sha256("SELECT * FROM rooms");
        let hash2 = compute_sha256("SELECT * FROM users");
        assert_ne!(
            hash1, hash2,
            "Contenido diferente debe producir hash diferente"
        );
    }

    #[test]
    fn test_hash_formato_hex() {
        let hash = compute_sha256("test");
        // SHA-256 siempre produce 64 caracteres hexadecimales
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_hash_bytes() {
        let data = vec![0u8, 1, 2, 3, 4, 5];
        let hash = compute_sha256_bytes(&data);
        assert_eq!(hash.len(), 64);
    }
}
