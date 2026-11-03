use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::{
    fmt::{self, Display, Formatter},
    option::Option::{self, None, Some},
    primitive::{str, u8, u16, u32, u64, usize},
    result::Result::{self, Ok},
};

use crate::{
    FrameError, Input, Output, WireError,
    credential::{self, Key, from_hexadecimal, to_hexadecimal},
    file_store::{FileStore, FileStoreError},
    server::Principal,
};

pub const TOKEN_LENGTH: usize = 32;
pub const LIFETIME_SECONDS_MAX: u64 = 30 * 24 * 3600;
pub const INACTIVITY_SECONDS_MAX: u64 = 7 * 24 * 3600;
pub const COOKIE_NAME: &str = "session";
pub const COOKIE_PATH: &str = "/api";
#[cfg(test)]
const RECORD_LENGTH: usize = 32 + 4 + 2 + 8 + 8 + 8;

pub type Token = [u8; TOKEN_LENGTH];

#[derive(Debug)]
pub enum SessionError {
    Random(getrandom::Error),
    Frame(FrameError),
    Store(FileStoreError),
    Record(u32),
}

impl Display for SessionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl WireError for SessionError {
    fn identifiers(&self, path: &mut Vec<u16>) {
        match self {
            SessionError::Random(_) => path.push(1),
            SessionError::Frame(error) => {
                path.push(2);
                error.identifiers(path);
            }
            SessionError::Store(error) => {
                path.push(3);
                error.identifiers(path);
            }
            SessionError::Record(_) => path.push(4),
        }
    }

    fn detail(&self) -> String {
        match self {
            SessionError::Random(error) => error.to_string(),
            SessionError::Frame(error) => error.detail(),
            SessionError::Store(error) => error.detail(),
            SessionError::Record(id) => format!("session record {id}"),
        }
    }

    fn status(&self) -> u16 {
        500
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub digest:  Key,
    pub user:    u32,
    pub device:  u16,
    pub peer:    u64,
    pub created: u64,
    pub seen:    u64,
}

impl From<&Record> for Principal {
    fn from(session: &Record) -> Self {
        Principal { user: session.user, device: session.device }
    }
}

impl Record {
    pub fn is_expired(&self, now: u64) -> bool {
        now < self.created
            || now < self.seen
            || now - self.created >= LIFETIME_SECONDS_MAX
            || now - self.seen >= INACTIVITY_SECONDS_MAX
    }
}

impl Output for Record {
    fn encode(&self, output: &mut Vec<u8>) {
        self.digest.encode(output);
        self.user.encode(output);
        self.device.encode(output);
        self.peer.encode(output);
        self.created.encode(output);
        self.seen.encode(output);
    }
}

impl Input<'_> for Record {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        Some(Self {
            digest:  Input::decode(input)?,
            user:    Input::decode(input)?,
            device:  Input::decode(input)?,
            peer:    Input::decode(input)?,
            created: Input::decode(input)?,
            seen:    Input::decode(input)?,
        })
    }
}

pub fn set_cookie(token: &Token) -> String {
    format!(
        "{COOKIE_NAME}={}; Max-Age={LIFETIME_SECONDS_MAX}; Path={COOKIE_PATH}; Secure; HttpOnly; SameSite=Strict",
        to_hexadecimal(token)
    )
}

pub fn clear_cookie() -> String {
    format!("{COOKIE_NAME}=; Max-Age=0; Path={COOKIE_PATH}; Secure; HttpOnly; SameSite=Strict")
}

pub fn token_from_cookie(header: &str) -> Option<Token> {
    header.split(';').find_map(|pair| {
        let (name, value) = pair.trim().split_once('=')?;
        if name.trim() != COOKIE_NAME {
            return None;
        }
        from_hexadecimal(value.trim())?.try_into().ok()
    })
}

pub struct Session<S: FileStore> {
    pub(crate) store:     S,
    pub(crate) by_digest: BTreeMap<Key, u32>,
}

impl<S: FileStore> Session<S> {
    pub fn new(store: S) -> Result<Self, SessionError> {
        let mut by_digest = BTreeMap::new();
        for (id, bytes) in store.range(0, u32::MAX) {
            let session = Record::from_bytes(bytes).map_err(|_| SessionError::Record(id))?;
            by_digest.insert(session.digest, id);
        }
        Ok(Self { store, by_digest })
    }

    pub fn issue(
        &mut self,
        now: u64,
        user: u32,
        device: u16,
    ) -> Result<(Token, Record), SessionError> {
        let mut token = [0u8; TOKEN_LENGTH];
        getrandom::fill(&mut token).map_err(SessionError::Random)?;
        let mut peer = [0u8; 8];
        getrandom::fill(&mut peer).map_err(SessionError::Random)?;
        let session = Record {
            digest: credential::digest(&token),
            user,
            device,
            peer: u64::from_le_bytes(peer),
            created: now,
            seen: now,
        };
        let id = self.store.issue_id();
        self.store.set(id, session.to_bytes());
        self.by_digest.insert(session.digest, id);
        Ok((token, session))
    }

    fn find(&self, token: &Token) -> Option<(u32, Record)> {
        let id = *self.by_digest.get(&credential::digest(token))?;
        let session: Record = self.store.decode(id)?;
        Some((id, session))
    }

    pub fn authenticate(&mut self, now: u64, token: &Token) -> Option<Record> {
        let (id, mut session) = self.find(token)?;
        if session.is_expired(now) {
            self.remove(id, &session.digest);
            return None;
        }
        if now > session.seen {
            session.seen = now;
            self.store.set(id, session.to_bytes());
        }
        Some(session)
    }

    pub fn revoke(&mut self, token: &Token) -> bool {
        match self.find(token) {
            Some((id, session)) => {
                self.remove(id, &session.digest);
                true
            }
            None => false,
        }
    }

    pub fn revoke_user(&mut self, user: u32) -> usize {
        let found: Vec<(u32, Key)> = self
            .store
            .range(0, u32::MAX)
            .filter_map(|(id, bytes)| {
                let session = Record::from_bytes(bytes).ok()?;
                (session.user == user).then_some((id, session.digest))
            })
            .collect();
        for (id, digest) in &found {
            self.remove(*id, digest);
        }
        found.len()
    }

    pub fn purge(&mut self, now: u64) -> usize {
        let expired: Vec<(u32, Key)> = self
            .store
            .range(0, u32::MAX)
            .filter_map(|(id, bytes)| {
                let session = Record::from_bytes(bytes).ok()?;
                session.is_expired(now).then_some((id, session.digest))
            })
            .collect();
        for (id, digest) in &expired {
            self.remove(*id, digest);
        }
        expired.len()
    }

    fn remove(&mut self, id: u32, digest: &Key) {
        self.store.delete(id);
        self.by_digest.remove(digest);
    }

    pub fn save(&mut self) -> Result<(), SessionError> {
        self.store.save().map_err(SessionError::Store)
    }

    pub fn compact(&mut self) -> Result<(), SessionError> {
        self.store.compact().map_err(SessionError::Store)
    }
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;

    use super::*;
    use crate::{
        block_on,
        file_store::{MemoryStore, Mount, StoreId},
    };

    fn store(id: StoreId) -> MemoryStore {
        block_on(MemoryStore::open(id, true)).and_then(MemoryStore::new).unwrap()
    }

    fn derive_id(name: &str) -> StoreId {
        StoreId { name: Box::leak(format!("session_{name}").into_boxed_str()), version: "0" }
    }

    fn sessions(id: StoreId) -> Session<MemoryStore> {
        Session::new(self::store(id)).unwrap()
    }

    #[test]
    fn a_record_round_trips_through_the_wire() {
        let session = Record {
            digest:  [9u8; 32],
            user:    7,
            device:  3,
            peer:    u64::MAX - 1,
            created: 1_700_000_000,
            seen:    1_700_000_500,
        };
        let bytes = session.to_bytes();
        assert_eq!(bytes.len(), RECORD_LENGTH);
        assert_eq!(Record::from_bytes(&bytes), Ok(session));
        assert_eq!(Record::from_bytes(&bytes[..RECORD_LENGTH - 1]), Err(FrameError::Decode));
    }

    #[test]
    fn a_session_expires_at_the_inactivity_or_lifetime_limit() {
        let session = Record {
            digest:  [0; 32],
            user:    1,
            device:  1,
            peer:    1,
            created: 1000,
            seen:    1000,
        };
        assert!(!session.is_expired(1000));
        assert!(!session.is_expired(1000 + INACTIVITY_SECONDS_MAX - 1));
        assert!(session.is_expired(1000 + INACTIVITY_SECONDS_MAX));
        assert!(session.is_expired(999));
        let touched = Record { seen: 1000 + LIFETIME_SECONDS_MAX - 10, ..session.clone() };
        assert!(!touched.is_expired(1000 + LIFETIME_SECONDS_MAX - 1));
        assert!(touched.is_expired(1000 + LIFETIME_SECONDS_MAX));
    }

    #[test]
    fn a_cookie_round_trips_through_its_header() {
        let token = [0xabu8; TOKEN_LENGTH];
        let header = set_cookie(&token);
        assert!(header.starts_with("session=abab"));
        assert!(header.contains("; HttpOnly"));
        assert!(header.contains("; Secure"));
        assert!(header.contains("; SameSite=Strict"));
        assert!(header.contains("; Path=/api"));
        let value = header.split(';').next().unwrap();
        assert_eq!(token_from_cookie(&format!("other=1; {value}; x=y")), Some(token));
        assert_eq!(token_from_cookie("session=abcd"), None);
        assert_eq!(token_from_cookie("sessions=abab"), None);
        assert_eq!(token_from_cookie(""), None);
        assert!(clear_cookie().contains("Max-Age=0"));
    }

    #[test]
    fn an_issued_token_authenticates_until_revoked() {
        let id = derive_id("an_issued_token_authenticates_until_revoked");
        let mut sessions = sessions(id);
        let (token, issued) = sessions.issue(100, 7, 1).unwrap();
        let (other, _) = sessions.issue(100, 8, 1).unwrap();
        assert_ne!(token, other);
        assert_eq!(sessions.by_digest.len(), 2);

        let seen = sessions.authenticate(150, &token).unwrap();
        assert_eq!((seen.user, seen.device, seen.peer, seen.seen), (7, 1, issued.peer, 150));
        assert_eq!(sessions.authenticate(150, &[0u8; 32]), None);

        assert!(sessions.revoke(&token));
        assert!(!sessions.revoke(&token));
        assert_eq!(sessions.authenticate(150, &token), None);
        assert_eq!(sessions.by_digest.len(), 1);
    }

    #[test]
    fn purge_removes_expired_sessions_and_the_rest_survive_reopen() {
        let id = derive_id("purge_removes_expired_sessions_and_the_rest_survive_reopen");
        let mut sessions = sessions(id);
        let (fresh, _) = sessions.issue(1000, 1, 1).unwrap();
        let (idle, _) = sessions.issue(1000, 1, 2).unwrap();
        let (old, _) = sessions.issue(1000, 2, 1).unwrap();
        sessions.save().unwrap();

        let mut reopened = self::sessions(id);
        assert_eq!(reopened.by_digest.len(), 3);
        assert!(reopened.authenticate(1000 + INACTIVITY_SECONDS_MAX - 1, &fresh).is_some());
        assert!(reopened.authenticate(1000 + 2 * INACTIVITY_SECONDS_MAX - 2, &fresh).is_some());
        assert_eq!(reopened.authenticate(1000 + INACTIVITY_SECONDS_MAX, &idle), None);
        assert_eq!(reopened.by_digest.len(), 2);
        assert_eq!(reopened.purge(1000 + LIFETIME_SECONDS_MAX), 2);
        assert_eq!(reopened.by_digest.len(), 0);
        let _ = old;
    }

    #[test]
    fn revoke_user_drops_every_device() {
        let id = derive_id("revoke_user_drops_every_device");
        let mut sessions = sessions(id);
        let (a, _) = sessions.issue(1, 5, 1).unwrap();
        let (b, _) = sessions.issue(1, 5, 2).unwrap();
        let (c, _) = sessions.issue(1, 6, 1).unwrap();
        assert_eq!(sessions.revoke_user(5), 2);
        assert_eq!(sessions.authenticate(2, &a), None);
        assert_eq!(sessions.authenticate(2, &b), None);
        assert!(sessions.authenticate(2, &c).is_some());
    }

    #[test]
    fn open_rejects_a_corrupt_record() {
        let id = derive_id("open_rejects_a_corrupt_record");
        let mut store = self::store(id);
        store.set(1, alloc::vec![1, 2, 3]);
        store.save().unwrap();
        let store = self::store(id);
        assert!(matches!(Session::new(store), Err(SessionError::Record(1))));
    }
}
