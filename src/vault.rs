use alloc::{
    fmt::{self, Display, Formatter},
    string::{String, ToString},
    vec::Vec,
};
use core::{
    primitive::{str, u8, u16, usize},
    result::Result::{self, Err, Ok},
};

use chacha20poly1305::{
    KeyInit, XChaCha20Poly1305, XNonce,
    aead::{Aead, Payload},
};
use hkdf::Hkdf;
use sha2::Sha256;

use crate::{
    WireError,
    credential::{KEY_LENGTH, Key},
};

pub const NONCE_LENGTH: usize = 24;
pub const TAG_LENGTH: usize = 16;
pub const WRAPPED_ROOT_LENGTH: usize = NONCE_LENGTH + KEY_LENGTH + TAG_LENGTH;

const ROOT_INFO: &[u8] = b"animagram/vault/1/root";
const STORE_INFO: &[u8] = b"animagram/vault/1/store/";
const RECORD_INFO: &[u8] = b"animagram/vault/1/record";

#[derive(Debug, PartialEq)]
pub enum VaultError {
    Random(getrandom::Error),
    Length(usize),
    Operation,
}

impl Display for VaultError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl WireError for VaultError {
    fn identifiers(&self, path: &mut Vec<u16>) {
        path.push(match self {
            VaultError::Random(_) => 1,
            VaultError::Length(_) => 2,
            VaultError::Operation => 3,
        });
    }

    fn detail(&self) -> String {
        self.to_string()
    }
}

fn cipher(key: &Key) -> XChaCha20Poly1305 {
    XChaCha20Poly1305::new(key.into())
}

fn encrypt(key: &Key, aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, VaultError> {
    let mut nonce = [0u8; NONCE_LENGTH];
    getrandom::fill(&mut nonce).map_err(VaultError::Random)?;
    let ciphertext = cipher(key)
        .encrypt(&XNonce::from(nonce), Payload { msg: plaintext, aad })
        .map_err(|_| VaultError::Operation)?;
    let mut out = Vec::with_capacity(NONCE_LENGTH + ciphertext.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

fn decrypt(key: &Key, aad: &[u8], envelope: &[u8]) -> Result<Vec<u8>, VaultError> {
    if envelope.len() < NONCE_LENGTH + TAG_LENGTH {
        return Err(VaultError::Length(envelope.len()));
    }
    let (nonce, ciphertext) = envelope.split_at(NONCE_LENGTH);
    let nonce: [u8; NONCE_LENGTH] =
        nonce.try_into().map_err(|_| VaultError::Length(envelope.len()))?;
    cipher(key)
        .decrypt(&XNonce::from(nonce), Payload { msg: ciphertext, aad })
        .map_err(|_| VaultError::Operation)
}

pub fn generate_root() -> Result<Key, VaultError> {
    let mut root = [0u8; KEY_LENGTH];
    getrandom::fill(&mut root).map_err(VaultError::Random)?;
    Ok(root)
}

pub fn wrap_root(wrap_key: &Key, root: &Key) -> Result<Vec<u8>, VaultError> {
    encrypt(wrap_key, ROOT_INFO, root)
}

pub fn unwrap_root(wrap_key: &Key, wrapped: &[u8]) -> Result<Key, VaultError> {
    if wrapped.len() != WRAPPED_ROOT_LENGTH {
        return Err(VaultError::Length(wrapped.len()));
    }
    let root = decrypt(wrap_key, ROOT_INFO, wrapped)?;
    root.as_slice().try_into().map_err(|_| VaultError::Length(root.len()))
}

pub fn store_key(root: &Key, name: &str) -> Key {
    let mut info = Vec::with_capacity(STORE_INFO.len() + name.len());
    info.extend_from_slice(STORE_INFO);
    info.extend_from_slice(name.as_bytes());
    let mut out = [0u8; KEY_LENGTH];
    Hkdf::<Sha256>::new(None, root)
        .expand(&info, &mut out)
        .expect("output length within hkdf bound");
    out
}

fn record_aad(key: &[u8]) -> Vec<u8> {
    let mut aad = Vec::with_capacity(RECORD_INFO.len() + key.len());
    aad.extend_from_slice(RECORD_INFO);
    aad.extend_from_slice(key);
    aad
}

pub fn encrypt_record(
    store_key: &Key,
    key: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, VaultError> {
    encrypt(store_key, &record_aad(key), plaintext)
}

pub fn decrypt_record(store_key: &Key, key: &[u8], envelope: &[u8]) -> Result<Vec<u8>, VaultError> {
    decrypt(store_key, &record_aad(key), envelope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credential;

    #[test]
    fn root_roundtrip_and_rewrap() {
        let stretch = [5u8; KEY_LENGTH];
        let wrap = credential::wrap_key(&stretch);
        let root = generate_root().unwrap();
        let wrapped = wrap_root(&wrap, &root).unwrap();
        assert_eq!(wrapped.len(), WRAPPED_ROOT_LENGTH);
        assert_eq!(unwrap_root(&wrap, &wrapped), Ok(root));
        assert_eq!(
            unwrap_root(&credential::auth_key(&stretch), &wrapped),
            Err(VaultError::Operation)
        );
        assert_eq!(
            unwrap_root(&wrap, &wrapped[1..]),
            Err(VaultError::Length(WRAPPED_ROOT_LENGTH - 1))
        );
        let mut tampered = wrapped.clone();
        tampered[NONCE_LENGTH + 3] ^= 1;
        assert_eq!(unwrap_root(&wrap, &tampered), Err(VaultError::Operation));

        let new_wrap = credential::wrap_key(&[6u8; KEY_LENGTH]);
        let rewrapped = wrap_root(&new_wrap, &root).unwrap();
        assert_ne!(rewrapped, wrapped);
        assert_eq!(unwrap_root(&new_wrap, &rewrapped), Ok(root));
    }

    #[test]
    fn store_keys_separated() {
        let root = [1u8; KEY_LENGTH];
        assert_eq!(store_key(&root, "calendar"), store_key(&root, "calendar"));
        assert_ne!(store_key(&root, "calendar"), store_key(&root, "memo"));
        assert_ne!(store_key(&root, "calendar"), store_key(&[2u8; KEY_LENGTH], "calendar"));
        assert_ne!(store_key(&root, "calendar"), root);
    }

    #[test]
    fn record_bound_to_key() {
        let store = store_key(&[3u8; KEY_LENGTH], "calendar");
        let key = [9u8; 16];
        let envelope = encrypt_record(&store, &key, b"plain").unwrap();
        assert_eq!(envelope.len(), NONCE_LENGTH + 5 + TAG_LENGTH);
        assert_eq!(decrypt_record(&store, &key, &envelope), Ok(b"plain".to_vec()));
        assert_eq!(decrypt_record(&store, &[8u8; 16], &envelope), Err(VaultError::Operation));
        assert_eq!(
            decrypt_record(&store_key(&[3u8; KEY_LENGTH], "memo"), &key, &envelope),
            Err(VaultError::Operation)
        );
        assert_eq!(
            decrypt_record(&store, &key, &envelope[..NONCE_LENGTH + TAG_LENGTH - 1]),
            Err(VaultError::Length(39))
        );
        let empty = encrypt_record(&store, &key, b"").unwrap();
        assert_eq!(decrypt_record(&store, &key, &empty), Ok(Vec::new()));
        let again = encrypt_record(&store, &key, b"plain").unwrap();
        assert_ne!(again, envelope);
    }
}
