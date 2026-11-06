use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::{
    option::Option::{self, None, Some},
    primitive::{str, u8, u16, u32, u64, usize},
    result::Result::{self, Err, Ok},
};

use crate::{
    Error, FrameError, Input, Output, WireError,
    credential::{KEY_LENGTH, Key},
    file_store::StoreId,
    server::{
        Action, Open, Resource,
        account::{Account, AccountError},
        authorize,
        http::{HttpError, Method, Request, Response},
        meet::{self, Board, MeetError},
        session::{self, Session, Token},
        sync::{self, Collection, SyncError},
    },
};

pub const API_VERSION: &str = "v1";
pub const STORE_VERSION: &str = "1";
pub const ACCOUNT_STORE: &str = "account";
pub const SESSION_STORE: &str = "session";
pub const STORE_NAME_LENGTH_MAX: usize = 32;
pub const COMPACT_INTERVAL_SECONDS: u64 = 24 * 3600;

pub struct Handler<S: Open> {
    pub started_at: u64,
    compacted_at:   u64,
    pepper:         Vec<u8>,
    allowlist:      BTreeSet<String>,
    origins:        Vec<String>,
    root:           S::Root,
    account:        Account<S>,
    session:        Session<S>,
    by_user_store:  BTreeMap<(u32, String), Collection<S>>,
    board:          Board,
}

impl<S: Open> Handler<S> {
    pub fn open(
        root: S::Root,
        started: u64,
        pepper: Vec<u8>,
        allowlist: BTreeSet<String>,
        origins: Vec<String>,
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
            origins,
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
}

fn mount<S: Open>(root: &S::Root, id: StoreId) -> Result<S, Error> {
    S::open(root, id, true).and_then(S::new).map_err(Error::FileStore)
}

fn encode_error(error: &Error) -> Vec<u8> {
    let mut path = Vec::new();
    error.identifiers(&mut path);
    let mut output = Vec::new();
    (path.len() as u8).encode(&mut output);
    for identifier in path {
        identifier.encode(&mut output);
    }
    error.detail().as_str().encode(&mut output);
    output
}

pub fn failure(status: u16, error: Error) -> Response {
    Response::new(status).body(encode_error(&error))
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

fn unauthorized() -> Response {
    reject(Error::Account(AccountError::Unauthorized))
}

fn forbidden() -> Response {
    reject(Error::Account(AccountError::NotAllowed))
}

fn is_store_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= STORE_NAME_LENGTH_MAX
        && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn token_from_request(request: &Request) -> Option<Token> {
    session::token_from_cookie(request.header("cookie")?)
}

fn is_origin_allowed(origins: &[String], request: &Request) -> bool {
    if request.method == Method::Get || origins.is_empty() {
        return true;
    }
    request.header("origin").is_some_and(|origin| origins.iter().any(|allowed| allowed == origin))
}

struct Credential<'a> {
    protocol:           u8,
    email:              &'a str,
    authentication_key: Key,
    wrapped_root:       &'a [u8],
}

fn read_credential(body: &[u8], with_root: bool) -> Result<Credential<'_>, FrameError> {
    let mut rest = body;
    let credential = (|| {
        let credential = Credential {
            protocol:           u8::decode(&mut rest)?,
            email:              <&str>::decode(&mut rest)?,
            authentication_key: <[u8; KEY_LENGTH]>::decode(&mut rest)?,
            wrapped_root:       if with_root { <&[u8]>::decode(&mut rest)? } else { &[] },
        };
        rest.is_empty().then_some(credential)
    })();
    credential.ok_or(FrameError::Decode)
}

impl<S: Open> Handler<S> {
    pub fn process_request(&mut self, now: u64, request: &Request) -> Response {
        let segments: Vec<&str> = request.segments().collect();
        if segments.first() != Some(&API_VERSION) {
            return failure(404, Error::Http(HttpError::Target));
        }
        if !is_origin_allowed(&self.origins, request) {
            return failure(403, Error::Http(HttpError::Header));
        }
        let presented = token_from_request(request);
        let body = &request.body;
        match (request.method, &segments[1..]) {
            (Method::Get, ["health"]) => {
                let mut output = Vec::new();
                1u8.encode(&mut output);
                now.saturating_sub(self.started_at).encode(&mut output);
                Response::new(200).body(output)
            }
            (Method::Put, ["account"]) => activate(self, now, body),
            (Method::Put, ["session"]) => issue(self, now, body),
            (Method::Get, ["session"]) => {
                match presented.and_then(|token| authenticate(self, now, &token)) {
                    Some(_) => Response::new(204),
                    None => unauthorized(),
                }
            }
            (Method::Delete, ["session"]) => revoke(self, presented),
            (Method::Put, ["account", "password"]) => {
                match presented.and_then(|token| authenticate(self, now, &token)) {
                    Some(authenticated) => {
                        let resource = Resource::Account(authenticated.user);
                        if !authorize(&(&authenticated).into(), Action::Write, &resource) {
                            return forbidden();
                        }
                        change_password(self, now, authenticated.user, body)
                    }
                    None => unauthorized(),
                }
            }
            (Method::Post, ["meet", "poll"]) => {
                match presented.and_then(|token| authenticate(self, now, &token)) {
                    Some(authenticated) => {
                        let resource = Resource::Peer(authenticated.peer);
                        if !authorize(&(&authenticated).into(), Action::Read, &resource) {
                            return forbidden();
                        }
                        Response::new(200)
                            .body(meet::encode(&self.board.poll(now, authenticated.peer)))
                    }
                    None => unauthorized(),
                }
            }
            (Method::Post, ["meet", "signal"]) => {
                match presented.and_then(|token| authenticate(self, now, &token)) {
                    Some(authenticated) => {
                        let resource = Resource::Peer(authenticated.peer);
                        if !authorize(&(&authenticated).into(), Action::Write, &resource) {
                            return forbidden();
                        }
                        signal(self, now, authenticated.peer, body)
                    }
                    None => unauthorized(),
                }
            }
            (Method::Post, ["sync", store, operation @ ("pull" | "push")])
                if is_store_name(store) =>
            {
                match presented.and_then(|token| authenticate(self, now, &token)) {
                    Some(authenticated) => {
                        let pulling = *operation == "pull";
                        let resource = Resource::Store {
                            user: authenticated.user,
                            name: (*store).to_string(),
                        };
                        let action = if pulling { Action::Read } else { Action::Write };
                        if !authorize(&(&authenticated).into(), action, &resource) {
                            return forbidden();
                        }
                        if pulling {
                            pull(self, authenticated.user, store, body)
                        } else {
                            push(self, authenticated.user, store, body)
                        }
                    }
                    None => unauthorized(),
                }
            }
            (_, ["sync", store, "pull" | "push"]) if is_store_name(store) => {
                failure(405, Error::Http(HttpError::Method))
            }
            (
                _,
                ["health"]
                | ["account"]
                | ["session"]
                | ["account", "password"]
                | ["meet", "poll" | "signal"],
            ) => failure(405, Error::Http(HttpError::Method)),
            _ => failure(404, Error::Http(HttpError::Target)),
        }
    }
}

fn authenticate<S: Open>(
    handler: &mut Handler<S>,
    now: u64,
    token: &Token,
) -> Option<session::Record> {
    handler.session.authenticate(now, token)
}

fn activate<S: Open>(handler: &mut Handler<S>, now: u64, body: &[u8]) -> Response {
    let credential = match read_credential(body, true) {
        Ok(credential) => credential,
        Err(error) => return reject(Error::Frame(error)),
    };
    let allowlist = handler.allowlist.clone();
    let result = handler.account.activate(
        &allowlist,
        &handler.pepper,
        now,
        credential.protocol,
        credential.email,
        &credential.authentication_key,
        credential.wrapped_root,
    );
    match result.and_then(|user| handler.account.save().map(|_| user)) {
        Ok(user) => Response::new(201).body(user.to_le_bytes().to_vec()),
        Err(error) => reject(Error::Account(error)),
    }
}

fn issue<S: Open>(handler: &mut Handler<S>, now: u64, body: &[u8]) -> Response {
    let credential = match read_credential(body, false) {
        Ok(credential) => credential,
        Err(error) => return reject(Error::Frame(error)),
    };
    let (principal, wrapped_root) = match handler.account.authenticate(
        &handler.pepper,
        credential.protocol,
        credential.email,
        &credential.authentication_key,
    ) {
        Ok(authenticated) => authenticated,
        Err(error) => return reject(Error::Account(error)),
    };
    let (token, issued) = match handler.session.issue(now, principal.user, principal.device) {
        Ok(issued) => issued,
        Err(error) => return reject(Error::Session(error)),
    };
    if let Err(error) = handler.account.save() {
        return reject(Error::Account(error));
    }
    if let Err(error) = handler.session.save() {
        return reject(Error::Session(error));
    }
    let mut output = Vec::new();
    principal.user.encode(&mut output);
    principal.device.encode(&mut output);
    issued.peer.encode(&mut output);
    wrapped_root.encode(&mut output);
    Response::new(200).header("Set-Cookie", &session::set_cookie(&token)).body(output)
}

fn revoke<S: Open>(handler: &mut Handler<S>, token: Option<Token>) -> Response {
    let revoked = token.is_some_and(|token| handler.session.revoke(&token));
    if !revoked {
        return unauthorized();
    }
    match handler.session.save() {
        Ok(()) => Response::new(204).header("Set-Cookie", &session::clear_cookie()),
        Err(error) => reject(Error::Session(error)),
    }
}

fn change_password<S: Open>(
    handler: &mut Handler<S>,
    now: u64,
    user: u32,
    body: &[u8],
) -> Response {
    let mut rest = body;
    let parsed = (|| {
        let current_key = <[u8; KEY_LENGTH]>::decode(&mut rest)?;
        let next_key = <[u8; KEY_LENGTH]>::decode(&mut rest)?;
        let wrapped_root = <&[u8]>::decode(&mut rest)?;
        rest.is_empty().then_some((current_key, next_key, wrapped_root))
    })();
    let (current_key, next_key, wrapped_root) = match parsed {
        Some(parsed) => parsed,
        None => return reject(Error::Frame(FrameError::Decode)),
    };
    if let Err(error) = handler.account.change_password(
        &handler.pepper,
        now,
        user,
        &current_key,
        &next_key,
        wrapped_root,
    ) {
        return reject(Error::Account(error));
    }
    handler.session.revoke_user(user);
    if let Err(error) = handler.account.save() {
        return reject(Error::Account(error));
    }
    if let Err(error) = handler.session.save() {
        return reject(Error::Session(error));
    }
    Response::new(204).header("Set-Cookie", &session::clear_cookie())
}

fn signal<S: Open>(handler: &mut Handler<S>, now: u64, from: u64, body: &[u8]) -> Response {
    let (target, kind, payload) = match meet::decode(body) {
        Ok(decoded) => decoded,
        Err(error) => return reject(Error::Meet(error)),
    };
    match handler.board.post(now, from, target, kind, payload) {
        Ok(()) => Response::new(204),
        Err(error) => reject(Error::Meet(error)),
    }
}

fn pull<S: Open>(handler: &mut Handler<S>, user: u32, store: &str, body: &[u8]) -> Response {
    let since = match sync::decode_pull(body) {
        Ok(since) => since,
        Err(error) => return reject(Error::Sync(error)),
    };
    let collection = match handler.collection(user, store) {
        Ok(collection) => collection,
        Err(error) => return reject(error),
    };
    let (records, more) = collection.pull(since, sync::PULL_SIZE_MAX);
    Response::new(200).body(sync::encode_pull(collection.head(), &records, more))
}

fn push<S: Open>(handler: &mut Handler<S>, user: u32, store: &str, body: &[u8]) -> Response {
    let entries = match sync::decode_push(body) {
        Ok(entries) => entries,
        Err(error) => return reject(Error::Sync(error)),
    };
    let collection = match handler.collection(user, store) {
        Ok(collection) => collection,
        Err(error) => return reject(error),
    };
    match collection.push(&entries).and_then(|outcomes| collection.save().map(|_| outcomes)) {
        Ok(outcomes) => Response::new(200).body(sync::encode_push(collection.head(), &outcomes)),
        Err(error) => reject(Error::Sync(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        credential::{self, PROTOCOL},
        file_store::{File, MemoryStore},
        server::{
            account::parse_allowlist,
            http::parse,
            sync::{Flags, Status},
        },
    };

    const PEPPER: &[u8] = b"pepper pepper pepper pepper pepper";
    const ROOT: [u8; 72] = [7u8; 72];

    struct Harness {
        handler: Handler<MemoryStore>,
        now:     u64,
    }

    impl Harness {
        fn new(origins: &[&str]) -> Self {
            let handler = Handler::<MemoryStore>::open(
                (),
                100,
                PEPPER.to_vec(),
                parse_allowlist("alice@example.jp\nbob@example.jp\n"),
                origins.iter().map(|origin| origin.to_string()).collect(),
            )
            .unwrap();
            Self { handler, now: 100 }
        }

        fn send(
            &mut self,
            method: &str,
            path: &str,
            headers: &[(&str, &str)],
            body: &[u8],
        ) -> Response {
            let mut raw = format!("{method} {path} HTTP/1.1\r\nContent-Length: {}\r\n", body.len());
            for (name, value) in headers {
                raw.push_str(&format!("{name}: {value}\r\n"));
            }
            raw.push_str("\r\n");
            let mut bytes = raw.into_bytes();
            bytes.extend_from_slice(body);
            let (request, _) = parse(&bytes).unwrap().unwrap();
            self.handler.process_request(self.now, &request)
        }

        fn activate(&mut self, email: &str, authentication_key: &Key) -> Response {
            let mut writer = Vec::new();
            PROTOCOL.encode(&mut writer);
            email.encode(&mut writer);
            authentication_key.encode(&mut writer);
            ROOT[..].encode(&mut writer);
            self.send("PUT", "/v1/account", &[], &writer)
        }

        fn authenticate(&mut self, email: &str, authentication_key: &Key) -> (Response, String) {
            let mut writer = Vec::new();
            PROTOCOL.encode(&mut writer);
            email.encode(&mut writer);
            authentication_key.encode(&mut writer);
            let response = self.send("PUT", "/v1/session", &[], &writer);
            let cookie = response
                .headers
                .iter()
                .find(|(name, _)| name == "Set-Cookie")
                .map(|(_, value)| value.split(';').next().unwrap().to_string())
                .unwrap_or_default();
            (response, cookie)
        }
    }

    fn error_path(response: &Response) -> Vec<u16> {
        let mut reader = &response.body[..];
        let depth = u8::decode(&mut reader).unwrap();
        (0..depth).map(|_| u16::decode(&mut reader).unwrap()).collect()
    }

    #[test]
    fn health_answers_and_unknown_routes_are_rejected() {
        let mut harness = Harness::new(&[]);
        harness.now = 160;
        let response = harness.send("GET", "/v1/health", &[], &[]);
        assert_eq!(response.status, 200);
        let mut reader = &response.body[..];
        assert_eq!((u8::decode(&mut reader), u64::decode(&mut reader)), (Some(1), Some(60)));

        assert_eq!(harness.send("POST", "/v1/health", &[], &[]).status, 405);
        assert_eq!(harness.send("GET", "/v0/health", &[], &[]).status, 404);
        let missing = harness.send("GET", "/v1/nothing", &[], &[]);
        assert_eq!(missing.status, 404);
        assert_eq!(error_path(&missing), [6, 3]);
        assert_eq!(harness.send("POST", "/v1/sync/mismatched.name/pull", &[], &[]).status, 404);
        assert_eq!(harness.send("GET", "/v1/sync/calendar/pull", &[], &[]).status, 405);
    }

    #[test]
    fn an_account_activates_authenticates_and_logs_out() {
        let mut harness = Harness::new(&[]);
        let key = credential::authentication_key(&[1u8; 32]);
        let created = harness.activate("alice@example.jp", &key);
        assert_eq!(created.status, 201);
        assert_eq!(u32::decode(&mut &created.body[..]), Some(1));
        assert_eq!(harness.activate("alice@example.jp", &key).status, 409);
        assert_eq!(harness.activate("carol@example.jp", &key).status, 403);
        assert_eq!(harness.activate("alice", &key).status, 400);
        assert_eq!(harness.send("PUT", "/v1/account", &[], &[1, 2]).status, 400);

        let (response, cookie) = harness.authenticate("alice@example.jp", &key);
        assert_eq!(response.status, 200);
        assert!(cookie.starts_with("session="));
        let mut reader = &response.body[..];
        assert_eq!((u32::decode(&mut reader), u16::decode(&mut reader)), (Some(1), Some(1)));
        let peer = u64::decode(&mut reader).unwrap();
        assert_ne!(peer, 0);
        assert_eq!(<&[u8]>::decode(&mut reader), Some(&ROOT[..]));
        assert!(reader.is_empty());

        let (wrong, _) = harness.authenticate("alice@example.jp", &[0u8; 32]);
        assert_eq!(wrong.status, 401);
        let (unknown, _) = harness.authenticate("nobody@example.jp", &key);
        assert_eq!(unknown.status, 401);
        assert_eq!(error_path(&wrong), error_path(&unknown));
        assert_eq!(wrong.body, unknown.body);

        assert_eq!(harness.send("GET", "/v1/session", &[("Cookie", &cookie)], &[]).status, 204);
        assert_eq!(harness.send("GET", "/v1/session", &[], &[]).status, 401);
        assert_eq!(
            harness.send("GET", "/v1/session", &[("Cookie", "session=00")], &[]).status,
            401
        );

        harness.now += session::INACTIVITY_SECONDS_MAX;
        assert_eq!(harness.send("GET", "/v1/session", &[("Cookie", &cookie)], &[]).status, 401);

        let (_, cookie) = harness.authenticate("alice@example.jp", &key);
        let revoked = harness.send("DELETE", "/v1/session", &[("Cookie", &cookie)], &[]);
        assert_eq!(revoked.status, 204);
        assert!(
            revoked
                .headers
                .iter()
                .any(|(name, value)| name == "Set-Cookie" && value.contains("Max-Age=0"))
        );
        assert_eq!(harness.send("DELETE", "/v1/session", &[("Cookie", &cookie)], &[]).status, 401);
        assert_eq!(harness.send("GET", "/v1/session", &[("Cookie", &cookie)], &[]).status, 401);
    }

    #[test]
    fn a_password_change_revokes_every_session() {
        let mut harness = Harness::new(&[]);
        let old = credential::authentication_key(&[1u8; 32]);
        let new = credential::authentication_key(&[2u8; 32]);
        harness.activate("bob@example.jp", &old);
        let (_, first) = harness.authenticate("bob@example.jp", &old);
        let (_, second) = harness.authenticate("bob@example.jp", &old);

        let mut writer = Vec::new();
        old.encode(&mut writer);
        new.encode(&mut writer);
        [9u8; 72][..].encode(&mut writer);
        let body = writer;
        assert_eq!(harness.send("PUT", "/v1/account/password", &[], &body).status, 401);
        let mut mismatched = Vec::new();
        new.encode(&mut mismatched);
        new.encode(&mut mismatched);
        [9u8; 72][..].encode(&mut mismatched);
        assert_eq!(
            harness.send("PUT", "/v1/account/password", &[("Cookie", &first)], &mismatched).status,
            401
        );
        assert_eq!(
            harness.send("PUT", "/v1/account/password", &[("Cookie", &first)], &body[..10]).status,
            400
        );
        assert_eq!(
            harness.send("PUT", "/v1/account/password", &[("Cookie", &first)], &body).status,
            204
        );

        assert_eq!(harness.send("GET", "/v1/session", &[("Cookie", &first)], &[]).status, 401);
        assert_eq!(harness.send("GET", "/v1/session", &[("Cookie", &second)], &[]).status, 401);
        assert_eq!(harness.authenticate("bob@example.jp", &old).0.status, 401);
        let (response, _) = harness.authenticate("bob@example.jp", &new);
        assert_eq!(response.status, 200);
        let mut reader = &response.body[..];
        u32::decode(&mut reader).unwrap();
        u16::decode(&mut reader).unwrap();
        u64::decode(&mut reader).unwrap();
        assert_eq!(<&[u8]>::decode(&mut reader), Some(&[9u8; 72][..]));
    }

    #[test]
    fn a_signal_reaches_only_the_target_peer() {
        let mut harness = Harness::new(&[]);
        let key = credential::authentication_key(&[1u8; 32]);
        harness.activate("alice@example.jp", &key);
        harness.activate("bob@example.jp", &key);
        let (alice, alice_cookie) = harness.authenticate("alice@example.jp", &key);
        let (bob, bob_cookie) = harness.authenticate("bob@example.jp", &key);
        let peer = |response: &Response| {
            let mut reader = &response.body[..];
            u32::decode(&mut reader).unwrap();
            u16::decode(&mut reader).unwrap();
            u64::decode(&mut reader).unwrap()
        };
        let (alice_peer, bob_peer) = (peer(&alice), peer(&bob));

        let mut writer = Vec::new();
        bob_peer.encode(&mut writer);
        meet::Kind::Offer.encode(&mut writer);
        b"v=0"[..].encode(&mut writer);
        assert_eq!(harness.send("POST", "/v1/meet/signal", &[], &writer).status, 401);
        let mut writer = Vec::new();
        bob_peer.encode(&mut writer);
        meet::Kind::Offer.encode(&mut writer);
        b"v=0"[..].encode(&mut writer);
        assert_eq!(
            harness.send("POST", "/v1/meet/signal", &[("Cookie", &alice_cookie)], &writer).status,
            204
        );
        let mut writer = Vec::new();
        alice_peer.encode(&mut writer);
        9u8.encode(&mut writer);
        b""[..].encode(&mut writer);
        assert_eq!(
            harness.send("POST", "/v1/meet/signal", &[("Cookie", &alice_cookie)], &writer).status,
            400
        );

        let empty = harness.send("POST", "/v1/meet/poll", &[("Cookie", &alice_cookie)], &[]);
        assert_eq!(u16::decode(&mut &empty.body[..]), Some(0));
        let polled = harness.send("POST", "/v1/meet/poll", &[("Cookie", &bob_cookie)], &[]);
        assert_eq!(polled.status, 200);
        let mut reader = &polled.body[..];
        assert_eq!(u16::decode(&mut reader), Some(1));
        assert_eq!(u64::decode(&mut reader), Some(alice_peer));
        assert_eq!(u8::decode(&mut reader), Some(meet::Kind::Offer as u8));
        assert_eq!(<&[u8]>::decode(&mut reader), Some(&b"v=0"[..]));
        assert!(harness.handler.board.is_online(harness.now, bob_peer));
    }

    #[test]
    fn sync_stores_are_isolated_per_user() {
        let mut harness = Harness::new(&[]);
        let key = credential::authentication_key(&[1u8; 32]);
        harness.activate("alice@example.jp", &key);
        harness.activate("bob@example.jp", &key);
        let (_, alice) = harness.authenticate("alice@example.jp", &key);
        let (_, bob) = harness.authenticate("bob@example.jp", &key);

        let mut writer = Vec::new();
        2u32.encode(&mut writer);
        [1u8; 16].encode(&mut writer);
        0u64.encode(&mut writer);
        0u8.encode(&mut writer);
        b"c1"[..].encode(&mut writer);
        [2u8; 16].encode(&mut writer);
        0u64.encode(&mut writer);
        0u8.encode(&mut writer);
        b"c2"[..].encode(&mut writer);
        let pushed = harness.send("POST", "/v1/sync/calendar/push", &[("Cookie", &alice)], &writer);
        assert_eq!(pushed.status, 200);
        let mut reader = &pushed.body[..];
        assert_eq!((u64::decode(&mut reader), u32::decode(&mut reader)), (Some(2), Some(2)));
        assert_eq!(
            (u8::decode(&mut reader), u64::decode(&mut reader)),
            (Some(Status::Accepted as u8), Some(1))
        );

        let mut writer = Vec::new();
        1u32.encode(&mut writer);
        [1u8; 16].encode(&mut writer);
        0u64.encode(&mut writer);
        Flags::tombstone().encode(&mut writer);
        b""[..].encode(&mut writer);
        let conflict =
            harness.send("POST", "/v1/sync/calendar/push", &[("Cookie", &alice)], &writer);
        let mut reader = &conflict.body[..];
        u64::decode(&mut reader).unwrap();
        u32::decode(&mut reader).unwrap();
        assert_eq!(
            (
                u8::decode(&mut reader),
                u64::decode(&mut reader),
                u8::decode(&mut reader),
                <&[u8]>::decode(&mut reader)
            ),
            (Some(Status::Conflict as u8), Some(1), Some(0), Some(&b"c1"[..]))
        );

        let mut writer = Vec::new();
        1u64.encode(&mut writer);
        let pulled = harness.send("POST", "/v1/sync/calendar/pull", &[("Cookie", &alice)], &writer);
        assert_eq!(pulled.status, 200);
        let mut reader = &pulled.body[..];
        assert_eq!(
            (u64::decode(&mut reader), u8::decode(&mut reader), u32::decode(&mut reader)),
            (Some(2), Some(0), Some(1))
        );
        assert_eq!(<[u8; 16]>::decode(&mut reader), Some([2u8; 16]));

        let mut writer = Vec::new();
        0u64.encode(&mut writer);
        let other_store =
            harness.send("POST", "/v1/sync/memo/pull", &[("Cookie", &alice)], &writer);
        let mut reader = &other_store.body[..];
        assert_eq!(
            (u64::decode(&mut reader), u8::decode(&mut reader), u32::decode(&mut reader)),
            (Some(0), Some(0), Some(0))
        );
        let mut writer = Vec::new();
        0u64.encode(&mut writer);
        let other_user =
            harness.send("POST", "/v1/sync/calendar/pull", &[("Cookie", &bob)], &writer);
        let mut reader = &other_user.body[..];
        assert_eq!(
            (u64::decode(&mut reader), u8::decode(&mut reader), u32::decode(&mut reader)),
            (Some(0), Some(0), Some(0))
        );

        assert_eq!(
            harness.send("POST", "/v1/sync/calendar/pull", &[("Cookie", &alice)], &[1]).status,
            400
        );
        assert_eq!(harness.send("POST", "/v1/sync/calendar/push", &[], &[]).status, 401);
    }

    #[test]
    fn mutations_require_a_matching_origin() {
        let mut harness = Harness::new(&["https://app.example.jp"]);
        let key = credential::authentication_key(&[1u8; 32]);
        assert_eq!(harness.send("GET", "/v1/health", &[], &[]).status, 200);
        let denied = harness.activate("alice@example.jp", &key);
        assert_eq!(denied.status, 403);
        let mut writer = Vec::new();
        PROTOCOL.encode(&mut writer);
        "alice@example.jp".encode(&mut writer);
        key.encode(&mut writer);
        ROOT[..].encode(&mut writer);
        let body = writer;
        assert_eq!(
            harness.send("PUT", "/v1/account", &[("Origin", "https://evil.example")], &body).status,
            403
        );
        assert_eq!(
            harness
                .send("PUT", "/v1/account", &[("Origin", "https://app.example.jp")], &body)
                .status,
            201
        );
    }

    #[test]
    fn maintain_purges_expired_sessions() {
        let mut harness = Harness::new(&[]);
        let key = credential::authentication_key(&[1u8; 32]);
        harness.activate("alice@example.jp", &key);
        harness.authenticate("alice@example.jp", &key);
        assert_eq!(harness.handler.session.by_digest.len(), 1);
        harness.handler.maintain(harness.now + session::LIFETIME_SECONDS_MAX).unwrap();
        assert_eq!(harness.handler.session.by_digest.len(), 0);
    }

    fn log_size<S: Open>(store: &S) -> u32 {
        store.size(File::Log).unwrap()
    }

    #[test]
    fn maintain_compacts_once_a_day() {
        let mut harness = Harness::new(&[]);
        let key = credential::authentication_key(&[1u8; 32]);
        harness.activate("alice@example.jp", &key);
        harness.authenticate("alice@example.jp", &key);
        assert!(log_size(&harness.handler.account.store) > 0);
        assert!(log_size(&harness.handler.session.store) > 0);
        harness.handler.maintain(harness.now + COMPACT_INTERVAL_SECONDS - 1).unwrap();
        assert!(log_size(&harness.handler.account.store) > 0);
        harness.handler.maintain(harness.now + COMPACT_INTERVAL_SECONDS).unwrap();
        assert_eq!(log_size(&harness.handler.account.store), 0);
        assert_eq!(log_size(&harness.handler.session.store), 0);
        assert_eq!(harness.handler.account.by_email.len(), 1);
        assert_eq!(harness.handler.session.by_digest.len(), 1);
    }

    #[test]
    fn open_compacts_the_stores() {
        let root = ();
        let allowlist = parse_allowlist("alice@example.jp\n");
        let mut handler =
            Handler::<MemoryStore>::open(root, 100, PEPPER.to_vec(), allowlist.clone(), Vec::new())
                .unwrap();
        let mut body = Vec::new();
        PROTOCOL.encode(&mut body);
        "alice@example.jp".encode(&mut body);
        credential::authentication_key(&[1u8; 32]).encode(&mut body);
        ROOT[..].encode(&mut body);
        let raw = format!("PUT /v1/account HTTP/1.1\r\nContent-Length: {}\r\n\r\n", body.len());
        let mut bytes = raw.into_bytes();
        bytes.extend_from_slice(&body);
        assert_eq!(handler.process_request(100, &parse(&bytes).unwrap().unwrap().0).status, 201);
        assert!(log_size(&handler.account.store) > 0);
        let handler =
            Handler::<MemoryStore>::open((), 200, PEPPER.to_vec(), allowlist, Vec::new()).unwrap();
        assert_eq!(log_size(&handler.account.store), 0);
        assert_eq!(handler.account.by_email.len(), 1);
    }
}
