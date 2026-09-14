//! PKCE helpers. See docs/05-sincronizacion.md section 1.2 step 1.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};

const VERIFIER_ALPHABET: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
const VERIFIER_LEN: usize = 64;

/// 64 characters from `[A-Za-z0-9-._~]`.
pub fn generate_verifier() -> String {
    let mut bytes = [0u8; VERIFIER_LEN];
    rand::rng().fill_bytes(&mut bytes);
    bytes
        .iter()
        .map(|b| VERIFIER_ALPHABET[(*b as usize) % VERIFIER_ALPHABET.len()] as char)
        .collect()
}

/// `base64url(sha256(verifier))` without padding (S256).
pub fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// 32 random bytes in base64url.
pub fn random_state() -> String {
    random_token(32)
}

/// `n` random bytes in base64url without padding (also used for webhook channel tokens).
pub fn random_token(n: usize) -> String {
    let mut bytes = vec![0u8; n];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_shape() {
        let v = generate_verifier();
        assert_eq!(v.len(), 64);
        assert!(v.bytes().all(|b| VERIFIER_ALPHABET.contains(&b)));
        assert_ne!(v, generate_verifier());
    }

    #[test]
    fn s256_matches_rfc7636_example() {
        // RFC 7636 appendix B.
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn state_is_43_chars() {
        assert_eq!(random_state().len(), 43);
    }
}
