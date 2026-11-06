use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{
    clone::Clone,
    cmp::PartialEq,
    fmt::{self, Debug, Display, Formatter},
    marker::Copy,
    option::Option::{self, None, Some},
    primitive::{str, u8, u16, u32, usize},
    result::Result::{self, Err, Ok},
};

#[cfg(feature = "vault")]
use argon2::{Algorithm, Argon2, Params, Version};
use hkdf::Hkdf;
use hmac::{Hmac, KeyInit, Mac};
use sha2::{Digest, Sha256};

use crate::{WireError, host::parse_url_search_params};

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
const AUTHENTICATION_INFO: &[u8] = b"animagram/credential/1/auth";
const WRAP_INFO: &[u8] = b"animagram/credential/1/wrap";

#[derive(Debug, PartialEq)]
pub enum CredentialError {
    Email,
    Password,
    Parameters,
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
            CredentialError::Parameters => 3,
        });
    }

    fn detail(&self) -> String {
        self.to_string()
    }
}

pub type Key = [u8; KEY_LENGTH];

crate::enumerate! {
    pub Field(u8) {
        Other,
        Email,
        Password,
    }
}

impl Field {
    pub fn list() -> &'static [Self] {
        &Self::LIST[1..]
    }

    /// ```
    /// # use app::credential::Field;
    /// assert_eq!(Field::Email.number(), 1);
    /// assert_eq!(Field::Password.number(), 2);
    /// ```
    pub fn number(self) -> u32 {
        u32::from(u8::from(self))
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Other => "",
            Self::Email => "email",
            Self::Password => "password",
        }
    }
}

pub const CREATE: &str = "create";

#[derive(Debug, PartialEq)]
pub struct Form {
    pub email:    String,
    pub password: String,
    pub create:   bool,
}

pub fn read_form(value: &str) -> Result<Form, CredentialError> {
    let pairs = parse_url_search_params(value);
    let [email, password] = [Field::Email, Field::Password].map(|field| {
        pairs.iter().find(|(key, _)| key == field.name()).map(|(_, value)| value.clone())
    });
    Ok(Form {
        email:    email.ok_or(CredentialError::Email)?,
        password: password.ok_or(CredentialError::Password)?,
        create:   pairs.iter().any(|(key, _)| key == CREATE),
    })
}

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

fn derive(material: &[u8], salt: &[u8], info: &[u8], output: &mut [u8]) {
    Hkdf::<Sha256>::new(Some(salt), material)
        .expand(info, output)
        .expect("output length within hkdf bound");
}

pub fn salt(email: &str) -> [u8; SALT_LENGTH] {
    let mut output = [0u8; SALT_LENGTH];
    derive(email.as_bytes(), &[], SALT_INFO, &mut output);
    output
}

#[cfg(feature = "vault")]
pub fn stretch(email: &str, password: &str) -> Result<Key, CredentialError> {
    let length = password.chars().count();
    if !(PASSWORD_MIN..=PASSWORD_MAX).contains(&length) {
        return Err(CredentialError::Password);
    }
    let parameters = Params::new(STRETCH_MEMORY, STRETCH_TIME, STRETCH_LANES, Some(KEY_LENGTH))
        .map_err(|_| CredentialError::Parameters)?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, parameters);
    let mut output = [0u8; KEY_LENGTH];
    argon
        .hash_password_into(password.as_bytes(), &salt(email), &mut output)
        .map_err(|_| CredentialError::Parameters)?;
    Ok(output)
}

pub fn authentication_key(stretch: &Key) -> Key {
    let mut output = [0u8; KEY_LENGTH];
    derive(stretch, &[], AUTHENTICATION_INFO, &mut output);
    output
}

pub fn wrap_key(stretch: &Key) -> Key {
    let mut output = [0u8; KEY_LENGTH];
    derive(stretch, &[], WRAP_INFO, &mut output);
    output
}

pub fn verifier(pepper: &[u8], authentication_key: &Key) -> Key {
    let mut mac = Hmac::<Sha256>::new_from_slice(pepper).expect("hmac accepts any key length");
    mac.update(authentication_key);
    mac.finalize().into_bytes().into()
}

pub fn verify(pepper: &[u8], authentication_key: &Key, stored: &Key) -> bool {
    let mut mac = Hmac::<Sha256>::new_from_slice(pepper).expect("hmac accepts any key length");
    mac.update(authentication_key);
    mac.verify_slice(stored).is_ok()
}

pub fn digest(token: &[u8]) -> Key {
    Sha256::digest(token).into()
}

pub fn to_hexadecimal(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    output
}

pub fn from_hexadecimal(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16)?;
            let low = (pair[1] as char).to_digit(16)?;
            Some((high * 16 + low) as u8)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use super::*;
    use crate::Rng;

    #[test]
    fn normalize_yields_a_trimmed_lowercase_address_or_an_email_error() {
        assert_eq!(normalize("  Alice@Example.JP "), Ok("alice@example.jp".to_string()));
        assert_eq!(normalize("alice"), Err(CredentialError::Email));
        assert_eq!(normalize("@example.jp"), Err(CredentialError::Email));
        assert_eq!(normalize("a b@example.jp"), Err(CredentialError::Email));
        assert_eq!(normalize("あ@example.jp"), Err(CredentialError::Email));
    }

    #[test]
    fn read_form_takes_only_the_expected_fields() {
        let names = ["email", "password", "create", "1", "2", "8", "x"];
        for seed in 0..500 {
            let mut rng = Rng::new(seed);
            let pairs: Vec<(&str, String)> = (0..rng.below(7))
                .map(|_| (names[rng.below(names.len())], format!("v{}", rng.below(100))))
                .collect();
            let value = pairs
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<String>>()
                .join("&");
            let first = |name: &str| {
                pairs.iter().find(|(key, _)| *key == name).map(|(_, value)| value.clone())
            };
            let expected = match (first("email"), first("password")) {
                (None, _) => Err(CredentialError::Email),
                (_, None) => Err(CredentialError::Password),
                (Some(email), Some(password)) => {
                    Ok(Form { email, password, create: first("create").is_some() })
                }
            };
            assert_eq!(read_form(&value), expected, "{value}");
        }
    }

    #[test]
    fn the_salt_depends_only_on_the_address() {
        assert_eq!(salt("alice@example.jp"), salt("alice@example.jp"));
        assert_ne!(salt("alice@example.jp"), salt("bob@example.jp"));
    }

    #[test]
    fn authentication_and_wrap_keys_differ_for_the_same_stretch() {
        let stretch = [7u8; KEY_LENGTH];
        assert_ne!(authentication_key(&stretch), wrap_key(&stretch));
        assert_eq!(authentication_key(&stretch), authentication_key(&stretch));
    }

    #[test]
    fn the_verifier_matches_only_its_own_pepper_and_authentication_key() {
        let key = [1u8; KEY_LENGTH];
        let stored = verifier(b"pepper", &key);
        assert!(verify(b"pepper", &key, &stored));
        assert!(!verify(b"other", &key, &stored));
        assert!(!verify(b"pepper", &[2u8; KEY_LENGTH], &stored));
    }

    #[test]
    fn from_hexadecimal_inverts_to_hexadecimal_only_for_well_formed_text() {
        let bytes = [0u8, 15, 16, 255];
        assert_eq!(to_hexadecimal(&bytes), "000f10ff");
        assert_eq!(from_hexadecimal("000f10ff"), Some(bytes.to_vec()));
        assert_eq!(from_hexadecimal("0"), None);
        assert_eq!(from_hexadecimal("zz"), None);
    }

    #[cfg(feature = "vault")]
    #[test]
    fn stretch_is_a_function_of_address_and_password_above_the_minimum_length() {
        assert_eq!(stretch("alice@example.jp", "short"), Err(CredentialError::Password));
        let first = stretch("alice@example.jp", "correct horse battery").unwrap();
        let second = stretch("alice@example.jp", "correct horse battery").unwrap();
        assert_eq!(first, second);
        assert_ne!(first, stretch("bob@example.jp", "correct horse battery").unwrap());
    }
}
