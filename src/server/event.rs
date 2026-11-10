use alloc::{string::String, vec::Vec};
use core::{
    option::Option::{self, None, Some},
    primitive::{bool, str, u8, u32, u64, usize},
};

use crate::{
    Input,
    credential::Key,
    server::{
        http::{self, Method},
        meet::Kind,
        session::Token,
        sync::{Entry, PUSH_COUNT_MAX},
    },
};

pub const API_VERSION: &str = "v1";
pub const STORE_NAME_LENGTH_MAX: usize = 32;

pub enum Event {
    Chunk(Chunk),
    Request { connection: u32, keep_alive: bool, token: Option<Token>, request: Request },
    Closed(u32),
    Maintain,
}

pub struct Chunk {
    pub connection: u32,
    pub bytes:      Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub struct Credential {
    pub protocol:           u8,
    pub email:              String,
    pub authentication_key: Key,
}

impl Input<'_> for Credential {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        Some(Self {
            protocol:           Input::decode(input)?,
            email:              Input::decode(input)?,
            authentication_key: Input::decode(input)?,
        })
    }
}

#[derive(Debug, PartialEq)]
pub enum Request {
    Health,
    Activate { credential: Credential, wrapped_root: Vec<u8> },
    Issue { credential: Credential },
    Check,
    Revoke,
    ChangePassword { current_key: Key, next_key: Key, wrapped_root: Vec<u8> },
    Poll,
    Signal { target: u64, kind: Kind, payload: Vec<u8> },
    Pull { store: String, since: u64 },
    Push { store: String, entries: Vec<Entry> },
}

fn is_store_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= STORE_NAME_LENGTH_MAX
        && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

impl Input<'_> for Request {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        let (message, consumed) = http::parse(input).ok()??;
        *input = &input[consumed..];
        let segments: Vec<&str> = message.segments().collect();
        let body = &mut { message.body };
        let request = match (message.method, segments.as_slice()) {
            (Method::Get, [API_VERSION, "health"]) => Request::Health,
            (Method::Put, [API_VERSION, "account"]) => Request::Activate {
                credential:   Input::decode(body)?,
                wrapped_root: Input::decode(body)?,
            },
            (Method::Put, [API_VERSION, "session"]) => {
                Request::Issue { credential: Input::decode(body)? }
            }
            (Method::Get, [API_VERSION, "session"]) => Request::Check,
            (Method::Delete, [API_VERSION, "session"]) => Request::Revoke,
            (Method::Put, [API_VERSION, "account", "password"]) => Request::ChangePassword {
                current_key:  Input::decode(body)?,
                next_key:     Input::decode(body)?,
                wrapped_root: Input::decode(body)?,
            },
            (Method::Post, [API_VERSION, "meet", "poll"]) => Request::Poll,
            (Method::Post, [API_VERSION, "meet", "signal"]) => Request::Signal {
                target:  Input::decode(body)?,
                kind:    Some(Kind::from(u8::decode(body)?)).filter(|kind| *kind != Kind::Other)?,
                payload: Input::decode(body)?,
            },
            (Method::Post, [API_VERSION, "sync", store, "pull"]) if is_store_name(store) => {
                Request::Pull { store: String::from(*store), since: Input::decode(body)? }
            }
            (Method::Post, [API_VERSION, "sync", store, "push"]) if is_store_name(store) => {
                Request::Push {
                    store:   String::from(*store),
                    entries: Some(Vec::<Entry>::decode(body)?)
                        .filter(|entries| entries.len() <= PUSH_COUNT_MAX)?,
                }
            }
            _ => return None,
        };
        body.is_empty().then_some(request)
    }
}

#[cfg(test)]
mod tests {
    use alloc::{format, vec};

    use super::*;
    use crate::{
        Output,
        bit::Zero,
        credential::PROTOCOL,
        server::sync::{Flags, IDENTIFIER_LENGTH},
    };

    fn frame(method: &str, path: &str, body: &[u8]) -> Vec<u8> {
        let mut bytes =
            format!("{method} {path} HTTP/1.1\r\nContent-Length: {}\r\n\r\n", body.len())
                .into_bytes();
        bytes.extend_from_slice(body);
        bytes
    }

    fn decode(method: &str, path: &str, body: &[u8]) -> Option<Request> {
        Request::decode(&mut &frame(method, path, body)[..])
    }

    #[test]
    fn a_request_selects_its_variant_by_method_and_path() {
        assert_eq!(decode("GET", "/v1/health", &[]), Some(Request::Health));
        assert_eq!(decode("GET", "/v1/session", &[]), Some(Request::Check));
        assert_eq!(decode("DELETE", "/v1/session", &[]), Some(Request::Revoke));
        assert_eq!(decode("POST", "/v1/meet/poll", &[]), Some(Request::Poll));
        assert_eq!(decode("POST", "/v1/health", &[]), None);
        assert_eq!(decode("GET", "/v0/health", &[]), None);
        assert_eq!(decode("GET", "/v1/nothing", &[]), None);
        assert_eq!(decode("POST", "/v1/sync/mismatched.name/pull", &0u64.to_le_bytes()), None);
        assert_eq!(decode("GET", "/v1/sync/calendar/pull", &0u64.to_le_bytes()), None);
    }

    #[test]
    fn a_request_decodes_its_body_without_trailing_bytes() {
        let mut body = Vec::new();
        PROTOCOL.encode(&mut body);
        "alice@example.jp".encode(&mut body);
        [3u8; 32].encode(&mut body);
        let credential = Credential {
            protocol:           PROTOCOL,
            email:              String::from("alice@example.jp"),
            authentication_key: [3u8; 32],
        };
        assert_eq!(decode("PUT", "/v1/session", &body), Some(Request::Issue { credential }));
        body.push(0);
        assert_eq!(decode("PUT", "/v1/session", &body), None);

        assert_eq!(
            decode("POST", "/v1/sync/calendar/pull", &9u64.to_le_bytes()),
            Some(Request::Pull { store: String::from("calendar"), since: 9 })
        );
        assert_eq!(decode("POST", "/v1/sync/calendar/pull", &[0; 9]), None);
    }

    #[test]
    fn a_signal_with_an_unknown_kind_does_not_decode() {
        for value in [0u8, 4, 255] {
            let mut body = Vec::new();
            1u64.encode(&mut body);
            value.encode(&mut body);
            b""[..].encode(&mut body);
            assert_eq!(decode("POST", "/v1/meet/signal", &body), None);
        }
        let mut body = Vec::new();
        77u64.encode(&mut body);
        Kind::Answer.encode(&mut body);
        b"sdp"[..].encode(&mut body);
        assert_eq!(
            decode("POST", "/v1/meet/signal", &body),
            Some(Request::Signal { target: 77, kind: Kind::Answer, payload: b"sdp".to_vec() })
        );
    }

    #[test]
    fn a_push_decodes_entries_up_to_the_count_limit() {
        let mut body = Vec::new();
        2u16.encode(&mut body);
        [1u8; IDENTIFIER_LENGTH].encode(&mut body);
        0u64.encode(&mut body);
        0u8.encode(&mut body);
        b"c1"[..].encode(&mut body);
        [2u8; IDENTIFIER_LENGTH].encode(&mut body);
        3u64.encode(&mut body);
        Flags::tombstone().encode(&mut body);
        b""[..].encode(&mut body);
        assert_eq!(
            decode("POST", "/v1/sync/calendar/push", &body),
            Some(Request::Push {
                store:   String::from("calendar"),
                entries: vec![
                    Entry {
                        identifier: [1u8; IDENTIFIER_LENGTH],
                        base:       0,
                        flags:      Flags::ZERO,
                        ciphertext: b"c1".to_vec(),
                    },
                    Entry {
                        identifier: [2u8; IDENTIFIER_LENGTH],
                        base:       3,
                        flags:      Flags::tombstone(),
                        ciphertext: Vec::new(),
                    },
                ],
            })
        );

        let mut unknown_flags = Vec::new();
        1u16.encode(&mut unknown_flags);
        [1u8; IDENTIFIER_LENGTH].encode(&mut unknown_flags);
        0u64.encode(&mut unknown_flags);
        2u8.encode(&mut unknown_flags);
        b""[..].encode(&mut unknown_flags);
        assert_eq!(decode("POST", "/v1/sync/calendar/push", &unknown_flags), None);

        let mut oversized = Vec::new();
        (PUSH_COUNT_MAX as u16 + 1).encode(&mut oversized);
        for _ in 0..=PUSH_COUNT_MAX {
            [1u8; IDENTIFIER_LENGTH].encode(&mut oversized);
            0u64.encode(&mut oversized);
            0u8.encode(&mut oversized);
            b""[..].encode(&mut oversized);
        }
        assert_eq!(decode("POST", "/v1/sync/calendar/push", &oversized), None);
    }

    #[test]
    fn a_request_advances_the_input_by_one_frame() {
        let mut bytes = frame("GET", "/v1/health", &[]);
        bytes.extend_from_slice(b"NEXT");
        let mut input = &bytes[..];
        assert_eq!(Request::decode(&mut input), Some(Request::Health));
        assert_eq!(input, b"NEXT");
    }
}
