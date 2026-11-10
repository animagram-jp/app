use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::{
    option::Option::Some,
    primitive::{u8, u16, u32, u64},
    result::Result::{self, Err, Ok},
};

use crate::{
    Error, Output,
    credential::Key,
    file_store::StoreId,
    server::{
        Action, Open, Resource,
        account::{Account, AccountError},
        authorize,
        event::Credential,
        http::{HttpError, Response},
        meet::{self, Board, Kind, MeetError},
        session::{self, Session, Token},
        sync::{self, Collection, Entry, SyncError},
    },
};

pub const STORE_VERSION: &str = "1";
pub const ACCOUNT_STORE: &str = "account";
pub const SESSION_STORE: &str = "session";
pub const COMPACT_INTERVAL_SECONDS: u64 = 24 * 3600;

pub struct Handler<S: Open> {
    pub started_at: u64,
    compacted_at:   u64,
    pepper:         Vec<u8>,
    allowlist:      BTreeSet<String>,
    root:           S::Root,
    account:        Account<S>,
    session:        Session<S>,
    by_user_store:  BTreeMap<(u32, String), Collection<S>>,
    board:          Board,
}

fn mount<S: Open>(root: &S::Root, id: StoreId) -> Result<S, Error> {
    S::open(root, id, true).and_then(S::new).map_err(Error::FileStore)
}

pub fn failure(status: u16, error: Error) -> Response {
    Response::new(status).body(error.to_bytes())
}

fn status(error: &Error) -> u16 {
    match error {
        Error::Http(HttpError::HeaderTooLarge) => 431,
        Error::Http(HttpError::BodyTooLarge) => 413,
        Error::Http(HttpError::TransferEncoding) => 411,
        Error::Http(HttpError::Version) => 505,
        Error::Account(AccountError::Unauthorized) => 401,
        Error::Account(AccountError::NotAllowed) => 403,
        Error::Account(AccountError::AlreadyRegistered) => 409,
        Error::Account(AccountError::Record(_) | AccountError::Store(_)) => 500,
        Error::Session(_) => 500,
        Error::Meet(MeetError::Full(_)) => 409,
        Error::Sync(SyncError::Record(_) | SyncError::Store(_)) => 500,
        _ => 400,
    }
}

pub fn reject(error: Error) -> Response {
    failure(status(&error), error)
}

impl<S: Open> Handler<S> {
    pub fn open(
        root: S::Root,
        started: u64,
        pepper: Vec<u8>,
        allowlist: BTreeSet<String>,
    ) -> Result<Self, Error> {
        let mut accounts =
            mount::<S>(&root, StoreId { name: ACCOUNT_STORE, version: STORE_VERSION })
                .and_then(|store| Account::new(store, &pepper).map_err(Error::Account))?;
        accounts.compact().map_err(Error::Account)?;
        let mut sessions =
            mount::<S>(&root, StoreId { name: SESSION_STORE, version: STORE_VERSION })
                .and_then(|store| Session::new(store).map_err(Error::Session))?;
        sessions.compact().map_err(Error::Session)?;
        Ok(Self {
            started_at: started,
            compacted_at: started,
            pepper,
            allowlist,
            root,
            account: accounts,
            session: sessions,
            by_user_store: BTreeMap::new(),
            board: Board::new(),
        })
    }

    pub fn maintain(&mut self, now: u64) -> Result<(), Error> {
        self.board.sweep(now);
        if self.session.purge(now) > 0 {
            self.session.save().map_err(Error::Session)?;
        }
        if now.saturating_sub(self.compacted_at) >= COMPACT_INTERVAL_SECONDS {
            self.compact()?;
            self.compacted_at = now;
        }
        Ok(())
    }

    pub fn compact(&mut self) -> Result<(), Error> {
        self.account.compact().map_err(Error::Account)?;
        self.session.compact().map_err(Error::Session)?;
        for collection in self.by_user_store.values_mut() {
            collection.compact().map_err(Error::Sync)?;
        }
        Ok(())
    }

    fn collection(&mut self, user: u32, store: &str) -> Result<&mut Collection<S>, Error> {
        let key = (user, store.to_string());
        if !self.by_user_store.contains_key(&key) {
            let id = StoreId {
                name:    Box::leak(format!("user/{user}/{store}").into_boxed_str()),
                version: STORE_VERSION,
            };
            let mut collection = mount::<S>(&self.root, id)
                .and_then(|store| Collection::new(store).map_err(Error::Sync))?;
            collection.compact().map_err(Error::Sync)?;
            self.by_user_store.insert(key.clone(), collection);
        }
        Ok(self.by_user_store.get_mut(&key).expect("inserted above"))
    }

    pub fn admit(
        &mut self,
        now: u64,
        token: &Token,
        action: Action,
        resource: impl FnOnce(&session::Record) -> Resource,
    ) -> Result<session::Record, Response> {
        let Some(session) = self.session.authenticate(now, token) else {
            return Err(reject(Error::Account(AccountError::Unauthorized)));
        };
        if !authorize(&(&session).into(), action, &resource(&session)) {
            return Err(reject(Error::Account(AccountError::NotAllowed)));
        }
        Ok(session)
    }

    pub fn process_health(&self, now: u64) -> Response {
        let mut output = Vec::new();
        1u8.encode(&mut output);
        now.saturating_sub(self.started_at).encode(&mut output);
        Response::new(200).body(output)
    }

    pub fn process_activate(
        &mut self,
        now: u64,
        credential: &Credential,
        wrapped_root: &[u8],
    ) -> Response {
        let result = self.account.activate(
            &self.allowlist,
            &self.pepper,
            now,
            credential.protocol,
            &credential.email,
            &credential.authentication_key,
            wrapped_root,
        );
        match result.and_then(|user| self.account.save().map(|_| user)) {
            Ok(user) => Response::new(201).body(user.to_bytes()),
            Err(error) => reject(Error::Account(error)),
        }
    }

    pub fn process_issue(&mut self, now: u64, credential: &Credential) -> Response {
        let (principal, wrapped_root) = match self.account.authenticate(
            &self.pepper,
            credential.protocol,
            &credential.email,
            &credential.authentication_key,
        ) {
            Ok(authenticated) => authenticated,
            Err(error) => return reject(Error::Account(error)),
        };
        let (token, issued) = match self.session.issue(now, principal.user, principal.device) {
            Ok(issued) => issued,
            Err(error) => return reject(Error::Session(error)),
        };
        if let Err(error) = self.account.save() {
            return reject(Error::Account(error));
        }
        if let Err(error) = self.session.save() {
            return reject(Error::Session(error));
        }
        let mut output = Vec::new();
        principal.user.encode(&mut output);
        principal.device.encode(&mut output);
        issued.peer.encode(&mut output);
        wrapped_root.encode(&mut output);
        Response::new(200).header("Set-Cookie", &session::set_cookie(&token)).body(output)
    }

    pub fn process_revoke(&mut self, token: &Token) -> Response {
        if !self.session.revoke(token) {
            return reject(Error::Account(AccountError::Unauthorized));
        }
        match self.session.save() {
            Ok(()) => Response::new(204).header("Set-Cookie", &session::clear_cookie()),
            Err(error) => reject(Error::Session(error)),
        }
    }

    pub fn process_change_password(
        &mut self,
        now: u64,
        user: u32,
        current_key: &Key,
        next_key: &Key,
        wrapped_root: &[u8],
    ) -> Response {
        if let Err(error) = self.account.change_password(
            &self.pepper,
            now,
            user,
            current_key,
            next_key,
            wrapped_root,
        ) {
            return reject(Error::Account(error));
        }
        self.session.revoke_user(user);
        if let Err(error) = self.account.save() {
            return reject(Error::Account(error));
        }
        if let Err(error) = self.session.save() {
            return reject(Error::Session(error));
        }
        Response::new(204).header("Set-Cookie", &session::clear_cookie())
    }

    pub fn process_poll(&mut self, now: u64, peer: u64) -> Response {
        Response::new(200).body(meet::encode(&self.board.poll(now, peer)))
    }

    pub fn process_signal(
        &mut self,
        now: u64,
        from: u64,
        target: u64,
        kind: Kind,
        payload: Vec<u8>,
    ) -> Response {
        match self.board.post(now, from, target, kind, payload) {
            Ok(()) => Response::new(204),
            Err(error) => reject(Error::Meet(error)),
        }
    }

    pub fn process_pull(&mut self, user: u32, store: &str, since: u64) -> Response {
        let collection = match self.collection(user, store) {
            Ok(collection) => collection,
            Err(error) => return reject(error),
        };
        let (records, more) = collection.pull(since, sync::PULL_SIZE_MAX);
        Response::new(200).body(sync::encode_pull(collection.head(), &records, more))
    }

    pub fn process_push(&mut self, user: u32, store: &str, entries: &[Entry]) -> Response {
        let collection = match self.collection(user, store) {
            Ok(collection) => collection,
            Err(error) => return reject(error),
        };
        match collection.push(entries).and_then(|outcomes| collection.save().map(|_| outcomes)) {
            Ok(outcomes) => {
                Response::new(200).body(sync::encode_push(collection.head(), &outcomes))
            }
            Err(error) => reject(Error::Sync(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        credential::{self, PROTOCOL},
        file_store::{File, MemoryStore},
        server::account::parse_allowlist,
    };

    const PEPPER: &[u8] = b"pepper pepper pepper pepper pepper";
    const ROOT: [u8; 72] = [7u8; 72];

    fn credential(email: &str) -> Credential {
        Credential {
            protocol:           PROTOCOL,
            email:              String::from(email),
            authentication_key: credential::authentication_key(&[1u8; 32]),
        }
    }

    fn open(started: u64) -> Handler<MemoryStore> {
        Handler::<MemoryStore>::open(
            (),
            started,
            PEPPER.to_vec(),
            parse_allowlist("alice@example.jp\n"),
        )
        .unwrap()
    }

    fn log_size<S: Open>(store: &S) -> u32 {
        store.size(File::Log).unwrap()
    }

    #[test]
    fn maintain_purges_expired_sessions() {
        let mut handler = open(100);
        handler.process_activate(100, &credential("alice@example.jp"), &ROOT);
        assert_eq!(handler.process_issue(100, &credential("alice@example.jp")).status, 200);
        assert_eq!(handler.session.by_digest.len(), 1);
        handler.maintain(100 + session::LIFETIME_SECONDS_MAX).unwrap();
        assert_eq!(handler.session.by_digest.len(), 0);
    }

    #[test]
    fn maintain_compacts_once_a_day() {
        let mut handler = open(100);
        handler.process_activate(100, &credential("alice@example.jp"), &ROOT);
        handler.process_issue(100, &credential("alice@example.jp"));
        assert!(log_size(&handler.account.store) > 0);
        assert!(log_size(&handler.session.store) > 0);
        handler.maintain(100 + COMPACT_INTERVAL_SECONDS - 1).unwrap();
        assert!(log_size(&handler.account.store) > 0);
        handler.maintain(100 + COMPACT_INTERVAL_SECONDS).unwrap();
        assert_eq!(log_size(&handler.account.store), 0);
        assert_eq!(log_size(&handler.session.store), 0);
        assert_eq!(handler.account.by_email.len(), 1);
        assert_eq!(handler.session.by_digest.len(), 1);
    }

    #[test]
    fn open_compacts_the_stores() {
        let mut handler = open(100);
        assert_eq!(
            handler.process_activate(100, &credential("alice@example.jp"), &ROOT).status,
            201
        );
        assert!(log_size(&handler.account.store) > 0);
        let handler = open(200);
        assert_eq!(log_size(&handler.account.store), 0);
        assert_eq!(handler.account.by_email.len(), 1);
    }
}
