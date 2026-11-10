use alloc::{collections::VecDeque, vec, vec::Vec};
use core::{
    iter::Extend,
    mem,
    option::Option::Some,
    primitive::{u8, u64},
    result::Result::{self, Ok},
};

use crate::{
    Error, FrameError, Input,
    server::{
        Action, Open, Resource,
        event::{Event, Request},
        handler::{Handler, reject},
        host::Command,
        http::Response,
    },
};

pub struct App<S: Open> {
    events:   VecDeque<Event>,
    handler:  Handler<S>,
    commands: Vec<Command>,
}

impl<S: Open> App<S> {
    pub fn new(handler: Handler<S>) -> Self {
        Self { events: VecDeque::new(), handler, commands: Vec::new() }
    }

    fn dispatch(&mut self, now: u64, event: Event) -> (Vec<Event>, Vec<Command>) {
        let Self { handler, .. } = self;

        match event {
            Event::Request(request) => {
                let response =
                    process_request(handler, now, request).unwrap_or_else(|response| response);
                (vec![], vec![Command::Response(response)])
            }
            Event::Maintain => {
                let _ = handler.maintain(now);
                (vec![], vec![])
            }
        }
    }

    pub(crate) fn run(&mut self, now: u64, event: Event) {
        self.events.push_back(event);

        while let Some(event) = self.events.pop_front() {
            let (new_events, new_commands) = self.dispatch(now, event);
            self.events.extend(new_events);
            self.commands.extend(new_commands);
        }
    }

    pub fn process(&mut self, now: u64, frame: &[u8]) {
        let Some(request) = Request::decode(&mut { frame }) else {
            self.commands.push(Command::Response(reject(Error::Frame(FrameError::Decode))));
            return;
        };

        self.run(now, Event::Request(request));
    }

    pub fn take_commands(&mut self) -> Vec<Command> {
        mem::take(&mut self.commands)
    }
}

fn process_request<S: Open>(
    handler: &mut Handler<S>,
    now: u64,
    request: Request,
) -> Result<Response, Response> {
    Ok(match request {
        Request::Health => handler.process_health(now),
        Request::Activate { credential, wrapped_root } => {
            handler.process_activate(now, &credential, &wrapped_root)
        }
        Request::Issue { credential } => handler.process_issue(now, &credential),
        Request::Revoke { token } => handler.process_revoke(&token),
        Request::Check { token } => {
            handler.admit(now, &token, Action::Read, |session| Resource::Account(session.user))?;
            Response::new(204)
        }
        Request::ChangePassword { token, current_key, next_key, wrapped_root } => {
            let session = handler
                .admit(now, &token, Action::Write, |session| Resource::Account(session.user))?;
            handler.process_change_password(
                now,
                session.user,
                &current_key,
                &next_key,
                &wrapped_root,
            )
        }
        Request::Poll { token } => {
            let session =
                handler.admit(now, &token, Action::Read, |session| Resource::Peer(session.peer))?;
            handler.process_poll(now, session.peer)
        }
        Request::Signal { token, target, kind, payload } => {
            let session = handler
                .admit(now, &token, Action::Write, |session| Resource::Peer(session.peer))?;
            handler.process_signal(now, session.peer, target, kind, payload)
        }
        Request::Pull { token, store, since } => {
            let session = handler.admit(now, &token, Action::Read, |session| Resource::Store {
                user: session.user,
                name: store.clone(),
            })?;
            handler.process_pull(session.user, &store, since)
        }
        Request::Push { token, store, entries } => {
            let session = handler.admit(now, &token, Action::Write, |session| Resource::Store {
                user: session.user,
                name: store.clone(),
            })?;
            handler.process_push(session.user, &store, &entries)
        }
    })
}

#[cfg(test)]
mod tests {
    use alloc::{
        format,
        string::{String, ToString},
    };

    use super::*;
    use crate::{
        Output,
        credential::{self, Key, PROTOCOL},
        file_store::MemoryStore,
        server::{
            account::parse_allowlist,
            meet, session,
            sync::{Flags, Status},
        },
    };

    const PEPPER: &[u8] = b"pepper pepper pepper pepper pepper";
    const ROOT: [u8; 72] = [7u8; 72];

    struct Harness {
        app: App<MemoryStore>,
        now: u64,
    }

    fn frame(method: &str, path: &str, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
        let mut raw = format!("{method} {path} HTTP/1.1\r\nContent-Length: {}\r\n", body.len());
        for (name, value) in headers {
            raw.push_str(&format!("{name}: {value}\r\n"));
        }
        raw.push_str("\r\n");
        let mut bytes = raw.into_bytes();
        bytes.extend_from_slice(body);
        bytes
    }

    fn header<'a>(response: &'a Response, name: &str) -> Option<&'a str> {
        response.headers.iter().find(|(field, _)| field == name).map(|(_, value)| value.as_str())
    }

    impl Harness {
        fn new() -> Self {
            let handler = Handler::<MemoryStore>::open(
                (),
                100,
                PEPPER.to_vec(),
                parse_allowlist("alice@example.jp\nbob@example.jp\n"),
            )
            .unwrap();
            Self { app: App::new(handler), now: 100 }
        }

        fn send(
            &mut self,
            method: &str,
            path: &str,
            headers: &[(&str, &str)],
            body: &[u8],
        ) -> Response {
            self.app.process(self.now, &frame(method, path, headers, body));
            let mut commands = self.app.take_commands();
            assert_eq!(commands.len(), 1);
            let Command::Response(response) = commands.remove(0);
            response
        }

        fn activate(&mut self, email: &str, authentication_key: &Key) -> Response {
            let mut writer = Vec::new();
            PROTOCOL.encode(&mut writer);
            email.encode(&mut writer);
            authentication_key.encode(&mut writer);
            ROOT[..].encode(&mut writer);
            self.send("PUT", "/v1/account", &[], &writer)
        }

        fn issue(&mut self, email: &str, authentication_key: &Key) -> (Response, String) {
            let mut writer = Vec::new();
            PROTOCOL.encode(&mut writer);
            email.encode(&mut writer);
            authentication_key.encode(&mut writer);
            let response = self.send("PUT", "/v1/session", &[], &writer);
            let cookie = header(&response, "Set-Cookie")
                .map(|value| value.split(';').next().unwrap().to_string())
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
    fn health_answers_and_unknown_routes_do_not_decode() {
        let mut harness = Harness::new();
        harness.now = 160;
        let response = harness.send("GET", "/v1/health", &[], &[]);
        assert_eq!(response.status, 200);
        let mut reader = &response.body[..];
        assert_eq!((u8::decode(&mut reader), u64::decode(&mut reader)), (Some(1), Some(60)));

        let missing = harness.send("GET", "/v1/nothing", &[], &[]);
        assert_eq!(missing.status, 400);
        assert_eq!(error_path(&missing), [5, 1]);
        assert_eq!(harness.send("POST", "/v1/health", &[], &[]).status, 400);
    }

    #[test]
    fn an_account_activates_issues_and_revokes_a_session() {
        let mut harness = Harness::new();
        let key = credential::authentication_key(&[1u8; 32]);
        let created = harness.activate("alice@example.jp", &key);
        assert_eq!(created.status, 201);
        assert_eq!(u32::decode(&mut &created.body[..]), Some(1));
        assert_eq!(harness.activate("alice@example.jp", &key).status, 409);
        assert_eq!(harness.activate("carol@example.jp", &key).status, 403);
        assert_eq!(harness.activate("alice", &key).status, 400);
        assert_eq!(harness.send("PUT", "/v1/account", &[], &[1, 2]).status, 400);

        let (response, cookie) = harness.issue("alice@example.jp", &key);
        assert_eq!(response.status, 200);
        assert!(cookie.starts_with("session="));
        let mut reader = &response.body[..];
        assert_eq!((u32::decode(&mut reader), u16::decode(&mut reader)), (Some(1), Some(1)));
        let peer = u64::decode(&mut reader).unwrap();
        assert_ne!(peer, 0);
        assert_eq!(<&[u8]>::decode(&mut reader), Some(&ROOT[..]));
        assert!(reader.is_empty());

        let (wrong, _) = harness.issue("alice@example.jp", &[0u8; 32]);
        assert_eq!(wrong.status, 401);
        let (unknown, _) = harness.issue("nobody@example.jp", &key);
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

        let (_, cookie) = harness.issue("alice@example.jp", &key);
        let revoked = harness.send("DELETE", "/v1/session", &[("Cookie", &cookie)], &[]);
        assert_eq!(revoked.status, 204);
        assert!(header(&revoked, "Set-Cookie").is_some_and(|value| value.contains("Max-Age=0")));
        assert_eq!(harness.send("DELETE", "/v1/session", &[("Cookie", &cookie)], &[]).status, 401);
        assert_eq!(harness.send("GET", "/v1/session", &[("Cookie", &cookie)], &[]).status, 401);
    }

    #[test]
    fn a_password_change_revokes_every_session() {
        let mut harness = Harness::new();
        let old = credential::authentication_key(&[1u8; 32]);
        let new = credential::authentication_key(&[2u8; 32]);
        harness.activate("bob@example.jp", &old);
        let (_, first) = harness.issue("bob@example.jp", &old);
        let (_, second) = harness.issue("bob@example.jp", &old);

        let mut body = Vec::new();
        old.encode(&mut body);
        new.encode(&mut body);
        [9u8; 72][..].encode(&mut body);
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
        assert_eq!(harness.issue("bob@example.jp", &old).0.status, 401);
        let (response, _) = harness.issue("bob@example.jp", &new);
        assert_eq!(response.status, 200);
        let mut reader = &response.body[..];
        u32::decode(&mut reader).unwrap();
        u16::decode(&mut reader).unwrap();
        u64::decode(&mut reader).unwrap();
        assert_eq!(<&[u8]>::decode(&mut reader), Some(&[9u8; 72][..]));
    }

    #[test]
    fn a_signal_reaches_only_the_target_peer() {
        let mut harness = Harness::new();
        let key = credential::authentication_key(&[1u8; 32]);
        harness.activate("alice@example.jp", &key);
        harness.activate("bob@example.jp", &key);
        let (alice, alice_cookie) = harness.issue("alice@example.jp", &key);
        let (bob, bob_cookie) = harness.issue("bob@example.jp", &key);
        let peer = |response: &Response| {
            let mut reader = &response.body[..];
            u32::decode(&mut reader).unwrap();
            u16::decode(&mut reader).unwrap();
            u64::decode(&mut reader).unwrap()
        };
        let (alice_peer, bob_peer) = (peer(&alice), peer(&bob));

        let mut offer = Vec::new();
        bob_peer.encode(&mut offer);
        meet::Kind::Offer.encode(&mut offer);
        b"v=0"[..].encode(&mut offer);
        assert_eq!(harness.send("POST", "/v1/meet/signal", &[], &offer).status, 401);
        assert_eq!(
            harness.send("POST", "/v1/meet/signal", &[("Cookie", &alice_cookie)], &offer).status,
            204
        );
        let mut unknown = Vec::new();
        alice_peer.encode(&mut unknown);
        9u8.encode(&mut unknown);
        b""[..].encode(&mut unknown);
        assert_eq!(
            harness.send("POST", "/v1/meet/signal", &[("Cookie", &alice_cookie)], &unknown).status,
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
    }

    #[test]
    fn sync_stores_are_isolated_per_user() {
        let mut harness = Harness::new();
        let key = credential::authentication_key(&[1u8; 32]);
        harness.activate("alice@example.jp", &key);
        harness.activate("bob@example.jp", &key);
        let (_, alice) = harness.issue("alice@example.jp", &key);
        let (_, bob) = harness.issue("bob@example.jp", &key);

        let mut writer = Vec::new();
        2u16.encode(&mut writer);
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
        assert_eq!((u64::decode(&mut reader), u16::decode(&mut reader)), (Some(2), Some(2)));
        assert_eq!(
            (u8::decode(&mut reader), u64::decode(&mut reader)),
            (Some(Status::Accepted as u8), Some(1))
        );

        let mut writer = Vec::new();
        1u16.encode(&mut writer);
        [1u8; 16].encode(&mut writer);
        0u64.encode(&mut writer);
        Flags::tombstone().encode(&mut writer);
        b""[..].encode(&mut writer);
        let conflict =
            harness.send("POST", "/v1/sync/calendar/push", &[("Cookie", &alice)], &writer);
        let mut reader = &conflict.body[..];
        u64::decode(&mut reader).unwrap();
        u16::decode(&mut reader).unwrap();
        assert_eq!(
            (
                u8::decode(&mut reader),
                u64::decode(&mut reader),
                u8::decode(&mut reader),
                <&[u8]>::decode(&mut reader)
            ),
            (Some(Status::Conflict as u8), Some(1), Some(0), Some(&b"c1"[..]))
        );

        let pulled = harness.send(
            "POST",
            "/v1/sync/calendar/pull",
            &[("Cookie", &alice)],
            &1u64.to_le_bytes(),
        );
        assert_eq!(pulled.status, 200);
        let mut reader = &pulled.body[..];
        assert_eq!(
            (u64::decode(&mut reader), u8::decode(&mut reader), u16::decode(&mut reader)),
            (Some(2), Some(0), Some(1))
        );
        assert_eq!(<[u8; 16]>::decode(&mut reader), Some([2u8; 16]));

        for (cookie, path) in [(&alice, "/v1/sync/memo/pull"), (&bob, "/v1/sync/calendar/pull")] {
            let pulled = harness.send("POST", path, &[("Cookie", cookie)], &0u64.to_le_bytes());
            let mut reader = &pulled.body[..];
            assert_eq!(
                (u64::decode(&mut reader), u8::decode(&mut reader), u16::decode(&mut reader)),
                (Some(0), Some(0), Some(0))
            );
        }

        assert_eq!(
            harness.send("POST", "/v1/sync/calendar/pull", &[("Cookie", &alice)], &[1]).status,
            400
        );
        assert_eq!(
            harness.send("POST", "/v1/sync/calendar/push", &[], &0u16.to_le_bytes()).status,
            401
        );
    }
}
