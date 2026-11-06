use alloc::{
    collections::{BTreeMap, BTreeSet},
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::{
    fmt::{self, Display, Formatter},
    option::Option::{self, Some},
    primitive::{str, u8, u16, u32, u64, usize},
    result::Result::{self, Err, Ok},
};

use crate::{
    Input, Output, WireError,
    credential::{self, CredentialError, Key, PROTOCOL},
    file_store::{FileStore, FileStoreError},
    server::Principal,
};

pub const WRAPPED_ROOT_MIN: usize = 48;
pub const WRAPPED_ROOT_MAX: usize = 256;

#[derive(Debug, PartialEq)]
pub enum AccountError {
    Email(CredentialError),
    Protocol(u8),
    WrappedRoot(usize),
    NotAllowed,
    AlreadyRegistered,
    Unauthorized,
    Record(u32),
    Store(String),
}

impl Display for AccountError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl WireError for AccountError {
    fn identifiers(&self, path: &mut Vec<u16>) {
        match self {
            AccountError::Email(error) => {
                path.push(1);
                error.identifiers(path);
            }
            AccountError::Protocol(_) => path.push(2),
            AccountError::WrappedRoot(_) => path.push(3),
            AccountError::NotAllowed => path.push(4),
            AccountError::AlreadyRegistered => path.push(5),
            AccountError::Unauthorized => path.push(6),
            AccountError::Record(_) => path.push(7),
            AccountError::Store(_) => path.push(8),
        }
    }

    fn detail(&self) -> String {
        match self {
            AccountError::Email(error) => error.detail(),
            AccountError::Store(message) => message.clone(),
            AccountError::Record(id) => format!("account record {id}"),
            other => other.to_string(),
        }
    }
}

impl From<FileStoreError> for AccountError {
    fn from(error: FileStoreError) -> Self {
        AccountError::Store(error.detail())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub protocol:     u8,
    pub email:        String,
    pub verifier:     Key,
    pub wrapped_root: Vec<u8>,
    pub created:      u64,
    pub changed:      u64,
    pub last_device:  u16,
}

impl Output for Record {
    fn encode(&self, output: &mut Vec<u8>) {
        self.protocol.encode(output);
        self.email.encode(output);
        self.verifier.encode(output);
        self.wrapped_root.encode(output);
        self.created.encode(output);
        self.changed.encode(output);
        self.last_device.encode(output);
    }
}

impl Input<'_> for Record {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        Some(Self {
            protocol:     Input::decode(input)?,
            email:        Input::decode(input)?,
            verifier:     Input::decode(input)?,
            wrapped_root: Input::decode(input)?,
            created:      Input::decode(input)?,
            changed:      Input::decode(input)?,
            last_device:  Input::decode(input)?,
        })
    }
}

pub fn parse_allowlist(text: &str) -> BTreeSet<String> {
    text.lines().filter_map(|line| credential::normalize(line).ok()).collect()
}

fn check_protocol(protocol: u8) -> Result<(), AccountError> {
    if protocol == PROTOCOL { Ok(()) } else { Err(AccountError::Protocol(protocol)) }
}

fn check_wrapped_root(wrapped_root: &[u8]) -> Result<(), AccountError> {
    if (WRAPPED_ROOT_MIN..=WRAPPED_ROOT_MAX).contains(&wrapped_root.len()) {
        Ok(())
    } else {
        Err(AccountError::WrappedRoot(wrapped_root.len()))
    }
}

pub struct Account<S: FileStore> {
    pub(crate) store:    S,
    pub(crate) by_email: BTreeMap<String, u32>,
    dummy:               Key,
}

impl<S: FileStore> Account<S> {
    pub fn new(store: S, pepper: &[u8]) -> Result<Self, AccountError> {
        let mut by_email = BTreeMap::new();
        for (id, bytes) in store.range(0, u32::MAX) {
            let account = Record::from_bytes(bytes).map_err(|_| AccountError::Record(id))?;
            by_email.insert(account.email, id);
        }
        let dummy = credential::verifier(pepper, &[0u8; credential::KEY_LENGTH]);
        Ok(Self { store, by_email, dummy })
    }

    pub fn get(&self, user: u32) -> Option<Record> {
        self.store.decode(user)
    }

    pub fn find(&self, email: &str) -> Option<u32> {
        self.by_email.get(&credential::normalize(email).ok()?).copied()
    }

    pub fn activate(
        &mut self,
        allowlist: &BTreeSet<String>,
        pepper: &[u8],
        now: u64,
        protocol: u8,
        email: &str,
        authentication_key: &Key,
        wrapped_root: &[u8],
    ) -> Result<u32, AccountError> {
        check_protocol(protocol)?;
        check_wrapped_root(wrapped_root)?;
        let email = credential::normalize(email).map_err(AccountError::Email)?;
        if !allowlist.contains(&email) {
            return Err(AccountError::NotAllowed);
        }
        if self.by_email.contains_key(&email) {
            return Err(AccountError::AlreadyRegistered);
        }
        let account = Record {
            protocol,
            email: email.clone(),
            verifier: credential::verifier(pepper, authentication_key),
            wrapped_root: wrapped_root.to_vec(),
            created: now,
            changed: now,
            last_device: 0,
        };
        let user = self.store.issue_id();
        self.store.set(user, account.to_bytes());
        self.by_email.insert(email, user);
        Ok(user)
    }

    pub fn authenticate(
        &mut self,
        pepper: &[u8],
        protocol: u8,
        email: &str,
        authentication_key: &Key,
    ) -> Result<(Principal, Vec<u8>), AccountError> {
        check_protocol(protocol)?;
        let found = credential::normalize(email)
            .ok()
            .and_then(|email| self.by_email.get(&email).copied())
            .and_then(|user| Some((user, self.get(user)?)));
        let stored = found.as_ref().map_or(&self.dummy, |(_, account)| &account.verifier);
        let matched = credential::verify(pepper, authentication_key, stored);
        match found {
            Some((user, mut account)) if matched && account.protocol == protocol => {
                account.last_device = account.last_device.wrapping_add(1).max(1);
                self.store.set(user, account.to_bytes());
                Ok((Principal { user, device: account.last_device }, account.wrapped_root))
            }
            _ => Err(AccountError::Unauthorized),
        }
    }

    pub fn change_password(
        &mut self,
        pepper: &[u8],
        now: u64,
        user: u32,
        current_key: &Key,
        next_key: &Key,
        wrapped_root: &[u8],
    ) -> Result<(), AccountError> {
        check_wrapped_root(wrapped_root)?;
        let mut account = self.get(user).ok_or(AccountError::Unauthorized)?;
        if !credential::verify(pepper, current_key, &account.verifier) {
            return Err(AccountError::Unauthorized);
        }
        account.verifier = credential::verifier(pepper, next_key);
        account.wrapped_root = wrapped_root.to_vec();
        account.changed = now;
        self.store.set(user, account.to_bytes());
        Ok(())
    }

    pub fn save(&mut self) -> Result<(), AccountError> {
        Ok(self.store.save()?)
    }

    pub fn compact(&mut self) -> Result<(), AccountError> {
        Ok(self.store.compact()?)
    }
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;

    use super::*;
    use crate::{
        FrameError, block_on,
        file_store::{MemoryStore, Mount, StoreId},
    };

    const PEPPER: &[u8] = b"pepper pepper pepper pepper pepper";
    const ROOT: [u8; 72] = [7u8; 72];

    fn store(id: StoreId) -> MemoryStore {
        block_on(MemoryStore::open(id, true)).and_then(MemoryStore::new).unwrap()
    }

    fn derive_id(name: &str) -> StoreId {
        StoreId { name: Box::leak(format!("account_{name}").into_boxed_str()), version: "0" }
    }

    fn accounts(id: StoreId) -> Account<MemoryStore> {
        Account::new(self::store(id), PEPPER).unwrap()
    }

    fn allowlist() -> BTreeSet<String> {
        parse_allowlist("Alice@Example.jp\n\nnot an email\n bob@example.jp \n")
    }

    #[test]
    fn parse_allow_normalizes_addresses() {
        let allowlist = allowlist();
        assert_eq!(allowlist.len(), 2);
        assert!(allowlist.contains("alice@example.jp"));
        assert!(allowlist.contains("bob@example.jp"));
    }

    #[test]
    fn a_record_round_trips_through_the_wire() {
        let account = Record {
            protocol:     PROTOCOL,
            email:        "alice@example.jp".to_string(),
            verifier:     [3u8; 32],
            wrapped_root: ROOT.to_vec(),
            created:      10,
            changed:      20,
            last_device:  4,
        };
        let bytes = account.to_bytes();
        assert_eq!(Record::from_bytes(&bytes), Ok(account.clone()));
        let mut longer = bytes.clone();
        longer.push(0);
        assert_eq!(Record::from_bytes(&longer), Err(FrameError::Decode));
        assert_eq!(Record::from_bytes(&bytes[..10]), Err(FrameError::Decode));
    }

    #[test]
    fn activate_enforces_protocol_allow_list_and_uniqueness() {
        let id = derive_id("activate_enforces_protocol_allow_list_and_uniqueness");
        let mut accounts = accounts(id);
        let allowlist = allowlist();
        let key = [1u8; 32];
        assert_eq!(
            accounts.activate(&allowlist, PEPPER, 1, 9, "alice@example.jp", &key, &ROOT),
            Err(AccountError::Protocol(9))
        );
        assert_eq!(
            accounts.activate(&allowlist, PEPPER, 1, PROTOCOL, "alice@example.jp", &key, &[0; 10]),
            Err(AccountError::WrappedRoot(10))
        );
        assert_eq!(
            accounts.activate(&allowlist, PEPPER, 1, PROTOCOL, "carol@example.jp", &key, &ROOT),
            Err(AccountError::NotAllowed)
        );
        assert_eq!(
            accounts.activate(&allowlist, PEPPER, 1, PROTOCOL, "alice", &key, &ROOT),
            Err(AccountError::Email(CredentialError::Email))
        );
        let user =
            accounts.activate(&allowlist, PEPPER, 1, PROTOCOL, " ALICE@example.jp", &key, &ROOT);
        assert_eq!(user, Ok(1));
        assert_eq!(
            accounts.activate(&allowlist, PEPPER, 2, PROTOCOL, "alice@example.jp", &key, &ROOT),
            Err(AccountError::AlreadyRegistered)
        );
        assert_eq!(accounts.find("Alice@Example.JP"), Some(1));
        assert_eq!(accounts.by_email.len(), 1);
    }

    #[test]
    fn authenticate_normalizes_the_address_and_rejects_every_mismatch() {
        let id = derive_id("authenticate_normalizes_the_address_and_rejects_every_mismatch");
        let mut accounts = accounts(id);
        let key = [1u8; 32];
        accounts
            .activate(&allowlist(), PEPPER, 1, PROTOCOL, "alice@example.jp", &key, &ROOT)
            .unwrap();

        let first = accounts.authenticate(PEPPER, PROTOCOL, "alice@example.jp", &key).unwrap();
        assert_eq!(first, (Principal { user: 1, device: 1 }, ROOT.to_vec()));
        let second = accounts.authenticate(PEPPER, PROTOCOL, "Alice@example.jp ", &key).unwrap();
        assert_eq!(second.0.device, 2);

        assert_eq!(
            accounts.authenticate(PEPPER, PROTOCOL, "alice@example.jp", &[2u8; 32]),
            Err(AccountError::Unauthorized)
        );
        assert_eq!(
            accounts.authenticate(PEPPER, PROTOCOL, "nobody@example.jp", &key),
            Err(AccountError::Unauthorized)
        );
        assert_eq!(
            accounts.authenticate(b"other", PROTOCOL, "alice@example.jp", &key),
            Err(AccountError::Unauthorized)
        );
        assert_eq!(
            accounts.authenticate(PEPPER, 2, "alice@example.jp", &key),
            Err(AccountError::Protocol(2))
        );
        assert_eq!(
            accounts.authenticate(PEPPER, PROTOCOL, "", &key),
            Err(AccountError::Unauthorized)
        );
    }

    #[test]
    fn the_dummy_verifier_never_matches_a_zero_key() {
        let id = derive_id("decoy_never_matches_zero_key");
        let mut accounts = accounts(id);
        assert_eq!(
            accounts.authenticate(PEPPER, PROTOCOL, "nobody@example.jp", &[0u8; 32]),
            Err(AccountError::Unauthorized)
        );
    }

    #[test]
    fn change_password_replaces_the_verifier_and_wrapped_root() {
        let id = derive_id("change_password_replaces_the_verifier_and_wrapped_root");
        let mut accounts = accounts(id);
        let old = [1u8; 32];
        let new = [2u8; 32];
        let user = accounts
            .activate(&allowlist(), PEPPER, 1, PROTOCOL, "bob@example.jp", &old, &ROOT)
            .unwrap();
        assert_eq!(
            accounts.change_password(PEPPER, 5, user, &new, &new, &[9u8; 72]),
            Err(AccountError::Unauthorized)
        );
        assert_eq!(
            accounts.change_password(PEPPER, 5, user, &old, &new, &[9u8; 300]),
            Err(AccountError::WrappedRoot(300))
        );
        assert_eq!(
            accounts.change_password(PEPPER, 5, user + 1, &old, &new, &[9u8; 72]),
            Err(AccountError::Unauthorized)
        );
        accounts.change_password(PEPPER, 5, user, &old, &new, &[9u8; 72]).unwrap();
        assert_eq!(
            accounts.authenticate(PEPPER, PROTOCOL, "bob@example.jp", &old),
            Err(AccountError::Unauthorized)
        );
        let (_, wrapped_root) =
            accounts.authenticate(PEPPER, PROTOCOL, "bob@example.jp", &new).unwrap();
        assert_eq!(wrapped_root, [9u8; 72].to_vec());
        let account = accounts.get(user).unwrap();
        assert_eq!((account.created, account.changed, account.last_device), (1, 5, 1));
    }

    #[test]
    fn accounts_survive_reopen() {
        let id = derive_id("accounts_survive_reopen");
        let key = [1u8; 32];
        let mut accounts = accounts(id);
        accounts
            .activate(&allowlist(), PEPPER, 1, PROTOCOL, "alice@example.jp", &key, &ROOT)
            .unwrap();
        accounts.authenticate(PEPPER, PROTOCOL, "alice@example.jp", &key).unwrap();
        accounts.save().unwrap();

        let mut reopened = self::accounts(id);
        assert_eq!(reopened.find("alice@example.jp"), Some(1));
        let (principal, _) =
            reopened.authenticate(PEPPER, PROTOCOL, "alice@example.jp", &key).unwrap();
        assert_eq!(principal.device, 2);
    }

    #[test]
    fn open_rejects_a_corrupt_record() {
        let id = derive_id("open_rejects_a_corrupt_record");
        let mut store = self::store(id);
        store.set(3, alloc::vec![1]);
        store.save().unwrap();
        let store = self::store(id);
        assert_eq!(Account::new(store, PEPPER).err(), Some(AccountError::Record(3)));
    }
}
