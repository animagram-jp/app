use alloc::{
    fmt::{self, Display, Formatter},
    string::{String, ToString},
    vec::Vec,
};
use core::{
    primitive::{str, u8, u16, u32, usize},
    result::Result::{self, Err, Ok},
};

#[cfg(feature = "vault")]
use argon2::{Algorithm, Argon2, Params, Version};
use hkdf::Hkdf;
use hmac::{Hmac, KeyInit, Mac};
use sha2::{Digest, Sha256};

use crate::WireError;

pub const PROTOCOL: u8 = 1;
pub const KEY_LENGTH: usize = 32;
pub const SALT_LENGTH: usize = 16;
pub const EMAIL_MAX: usize = 254;
pub const PASSWORD_MIN: usize = 8;
pub const PASSWORD_MAX: usize = 128;
pub const STRETCH_MEMORY: u32 = 19 * 1024;
pub const STRETCH_TIME: u32 = 2;
pub const STRETCH_LANES: u32 = 1;

const SALT_INFO: &[u8] = b"animagram/credential/1/salt";
const AUTH_INFO: &[u8] = b"animagram/credential/1/auth";
const WRAP_INFO: &[u8] = b"animagram/credential/1/wrap";

#[derive(Debug, PartialEq)]
pub enum CredentialError {
    Email,
    Password,
    Stretch,
}

impl Display for CredentialError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl WireError for CredentialError {
    fn identifiers(&self, path: &mut Vec<u16>) {
        path.push(match self {
            CredentialError::Email => 1,
            CredentialError::Password => 2,
            CredentialError::Stretch => 3,
        });
    }

    fn detail(&self) -> String {
        self.to_string()
    }
}

pub type Key = [u8; KEY_LENGTH];

pub fn normalize(email: &str) -> Result<String, CredentialError> {
    let trimmed = email.trim();
    if trimmed.is_empty() || trimmed.len() > EMAIL_MAX || !trimmed.is_ascii() {
        return Err(CredentialError::Email);
    }
    let (local, domain) = trimmed.split_once('@').ok_or(CredentialError::Email)?;
    if local.is_empty() || domain.is_empty() || domain.contains('@') {
        return Err(CredentialError::Email);
    }
    if trimmed.bytes().any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control()) {
        return Err(CredentialError::Email);
    }
    Ok(trimmed.to_ascii_lowercase())
}

fn expand(ikm: &[u8], salt: &[u8], info: &[u8], out: &mut [u8]) {
    Hkdf::<Sha256>::new(Some(salt), ikm)
        .expand(info, out)
        .expect("output length within hkdf bound");
}

pub fn salt(email: &str) -> [u8; SALT_LENGTH] {
    let mut out = [0u8; SALT_LENGTH];
    expand(email.as_bytes(), &[], SALT_INFO, &mut out);
    out
}

#[cfg(feature = "vault")]
pub fn stretch(email: &str, password: &str) -> Result<Key, CredentialError> {
    let length = password.chars().count();
    if !(PASSWORD_MIN..=PASSWORD_MAX).contains(&length) {
        return Err(CredentialError::Password);
    }
    let params = Params::new(STRETCH_MEMORY, STRETCH_TIME, STRETCH_LANES, Some(KEY_LENGTH))
        .map_err(|_| CredentialError::Stretch)?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; KEY_LENGTH];
    argon
        .hash_password_into(password.as_bytes(), &salt(email), &mut out)
        .map_err(|_| CredentialError::Stretch)?;
    Ok(out)
}

pub fn auth_key(stretch: &Key) -> Key {
    let mut out = [0u8; KEY_LENGTH];
    expand(stretch, &[], AUTH_INFO, &mut out);
    out
}

pub fn wrap_key(stretch: &Key) -> Key {
    let mut out = [0u8; KEY_LENGTH];
    expand(stretch, &[], WRAP_INFO, &mut out);
    out
}

pub fn verifier(pepper: &[u8], auth_key: &Key) -> Key {
    let mut mac = Hmac::<Sha256>::new_from_slice(pepper).expect("hmac accepts any key length");
    mac.update(auth_key);
    mac.finalize().into_bytes().into()
}

pub fn verify(pepper: &[u8], auth_key: &Key, stored: &Key) -> bool {
    let mut mac = Hmac::<Sha256>::new_from_slice(pepper).expect("hmac accepts any key length");
    mac.update(auth_key);
    mac.verify_slice(stored).is_ok()
}

pub fn digest(token: &[u8]) -> Key {
    Sha256::digest(token).into()
}

pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    out
}

pub fn unhex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16)?;
            let low = (pair[1] as char).to_digit(16)?;
            Some((high * 16 + low) as u8)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_email() {
        assert_eq!(normalize("  Alice@Example.JP "), Ok("alice@example.jp".to_string()));
        assert_eq!(normalize("alice"), Err(CredentialError::Email));
        assert_eq!(normalize("@example.jp"), Err(CredentialError::Email));
        assert_eq!(normalize("a b@example.jp"), Err(CredentialError::Email));
        assert_eq!(normalize("あ@example.jp"), Err(CredentialError::Email));
    }

    #[test]
    fn salt_deterministic_and_distinct() {
        assert_eq!(salt("alice@example.jp"), salt("alice@example.jp"));
        assert_ne!(salt("alice@example.jp"), salt("bob@example.jp"));
    }

    #[test]
    fn keys_separated() {
        let stretch = [7u8; KEY_LENGTH];
        assert_ne!(auth_key(&stretch), wrap_key(&stretch));
        assert_eq!(auth_key(&stretch), auth_key(&stretch));
    }

    #[test]
    fn verifier_pepper_bound() {
        let auth = [1u8; KEY_LENGTH];
        let stored = verifier(b"pepper", &auth);
        assert!(verify(b"pepper", &auth, &stored));
        assert!(!verify(b"other", &auth, &stored));
        assert!(!verify(b"pepper", &[2u8; KEY_LENGTH], &stored));
    }

    #[test]
    fn hex_roundtrip() {
        let bytes = [0u8, 15, 16, 255];
        assert_eq!(hex(&bytes), "000f10ff");
        assert_eq!(unhex("000f10ff"), Some(bytes.to_vec()));
        assert_eq!(unhex("0"), None);
        assert_eq!(unhex("zz"), None);
    }

    #[cfg(feature = "vault")]
    #[test]
    fn stretch_bounds_and_determinism() {
        assert_eq!(stretch("alice@example.jp", "short"), Err(CredentialError::Password));
        let first = stretch("alice@example.jp", "correct horse battery").unwrap();
        let second = stretch("alice@example.jp", "correct horse battery").unwrap();
        assert_eq!(first, second);
        assert_ne!(first, stretch("bob@example.jp", "correct horse battery").unwrap());
    }
}
