// lia-client/src-tauri/src/sentinel.rs
// Modulo Sentinel: Data Loss Prevention (DLP) integrado en el binario.
// Escanea texto en busca de secretos conocidos y los reemplaza por
// <SECRET_REDACTED> antes de que salgan de la maquina local.

use regex::Regex;

/// Sentinel compila todas las regex una sola vez al crearse.
/// Reutiliza las expresiones compiladas en cada llamada a `sanitize()`.
pub struct Sentinel {
    rules: Vec<SanitizationRule>,
}

struct SanitizationRule {
    #[allow(dead_code)]
    name: &'static str,
    pattern: Regex,
}

const REDACTED: &str = "<SECRET_REDACTED>";

impl Sentinel {
    /// Crea una nueva instancia de Sentinel compilando todas las reglas de deteccion.
    /// Debe llamarse una sola vez al iniciar la aplicacion.
    pub fn new() -> Self {
        let rules = vec![
            // --- Tokens de Nube ---
            SanitizationRule {
                name: "AWS Access Key",
                pattern: Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
            },
            SanitizationRule {
                name: "AWS Secret Key",
                pattern: Regex::new(
                    r#"(?i)(aws_secret_access_key|aws_secret)\s*=\s*["']?[A-Za-z0-9/+=]{40}"#
                ).unwrap(),
            },
            SanitizationRule {
                name: "OpenAI API Key",
                pattern: Regex::new(r"sk-proj-[a-zA-Z0-9_\-]{20,}").unwrap(),
            },
            SanitizationRule {
                name: "Stripe Key",
                pattern: Regex::new(r"(?:sk|pk)_(?:test|live)_[a-zA-Z0-9]{20,}").unwrap(),
            },
            SanitizationRule {
                name: "Google API Key",
                pattern: Regex::new(r"AIza[0-9A-Za-z_\-]{35}").unwrap(),
            },

            // --- Cadenas de Conexion (URIs) ---
            SanitizationRule {
                name: "Database Connection URI",
                pattern: Regex::new(
                    r#"(?:mongodb|postgres|mysql|redis)://[^\s"']+:[^\s"']+@[^\s"']+"#
                ).unwrap(),
            },

            // --- Claves Genericas ---
            SanitizationRule {
                name: "Generic Key/Password",
                pattern: Regex::new(
                    r#"(?i)(password|secret|token|api_key|apikey|pwd|db_pass)\s*[=:]\s*["'][^"']{3,}["']"#
                ).unwrap(),
            },

            // --- PII (Personally Identifiable Information) ---
            SanitizationRule {
                name: "Email Address",
                pattern: Regex::new(
                    r"[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z]{2,}"
                ).unwrap(),
            },
            SanitizationRule {
                name: "Private IP Address",
                pattern: Regex::new(
                    r"(?:10|172\.(?:1[6-9]|2[0-9]|3[01])|192\.168)\.\d{1,3}\.\d{1,3}"
                ).unwrap(),
            },
        ];

        Sentinel { rules }
    }

    /// Recibe texto crudo y devuelve una copia con todos los secretos
    /// detectados reemplazados por <SECRET_REDACTED>.
    /// Opera completamente en memoria RAM, sin I/O.
    pub fn sanitize(&self, text: &str) -> String {
        let mut result = text.to_string();
        for rule in &self.rules {
            result = rule.pattern.replace_all(&result, REDACTED).to_string();
        }
        result
    }
}

// ---------------------------------------------------------------------------
// Tests unitarios
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn sentinel() -> Sentinel {
        Sentinel::new()
    }

    #[test]
    fn test_aws_access_key() {
        let s = sentinel();
        let input = r#"$aws_key = "AKIAIOSFODNN7EXAMPLE";"#;
        let output = s.sanitize(input);
        assert!(
            output.contains(REDACTED),
            "AWS key no fue redactada: {}",
            output
        );
        assert!(!output.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn test_openai_key() {
        let s = sentinel();
        let input = r#"api_key = "sk-proj-abc123def456ghi789jkl012mno345pqr678";"#;
        let output = s.sanitize(input);
        assert!(
            output.contains(REDACTED),
            "OpenAI key no fue redactada: {}",
            output
        );
    }

    #[test]
    fn test_stripe_key() {
        let s = sentinel();
        let input = r#"stripe_key = "sk_test_4eC39HqLyjWDarjtT1zdp7dc";"#;
        let output = s.sanitize(input);
        assert!(
            output.contains(REDACTED),
            "Stripe key no fue redactada: {}",
            output
        );
    }

    #[test]
    fn test_database_uri() {
        let s = sentinel();
        let input = r#"$db = new PDO('mysql://root:super_secreto_123@localhost/test');"#;
        let output = s.sanitize(input);
        assert!(
            output.contains(REDACTED),
            "DB URI no fue redactada: {}",
            output
        );
        assert!(!output.contains("super_secreto_123"));
    }

    #[test]
    fn test_generic_password() {
        let s = sentinel();
        let input = r#"$password = "mi_clave_secreta_123";"#;
        let output = s.sanitize(input);
        assert!(
            output.contains(REDACTED),
            "Password generico no fue redactado: {}",
            output
        );
    }

    #[test]
    fn test_email() {
        let s = sentinel();
        let input = "Contacto: alexis.campos@gmail.com para soporte.";
        let output = s.sanitize(input);
        assert!(
            output.contains(REDACTED),
            "Email no fue redactado: {}",
            output
        );
        assert!(!output.contains("alexis.campos@gmail.com"));
    }

    #[test]
    fn test_private_ip() {
        let s = sentinel();
        let input = "Servidor en 192.168.1.100 puerto 3306";
        let output = s.sanitize(input);
        assert!(
            output.contains(REDACTED),
            "IP privada no fue redactada: {}",
            output
        );
    }

    #[test]
    fn test_texto_normal_no_se_modifica() {
        let s = sentinel();
        let input = "function calcularTotal(precio, cantidad) {\n    return precio * cantidad;\n}";
        let output = s.sanitize(input);
        assert_eq!(input, output, "Texto normal fue modificado incorrectamente");
    }

    #[test]
    fn test_multiples_secretos() {
        let s = sentinel();
        let input = concat!(
            "$aws = \"AKIAIOSFODNN7EXAMPLE\";\n",
            "$db = new PDO('mysql://root:pass123@localhost/db');\n",
            "$email = \"user@test.com\";"
        );
        let output = s.sanitize(input);
        // Verificar que no queda ninguno de los secretos originales
        assert!(!output.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(!output.contains("root:pass123"));
        assert!(!output.contains("user@test.com"));
    }
}
