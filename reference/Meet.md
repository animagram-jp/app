// This file includes untranslated text (ja).

ブラウザクライアントを含めたP2P接続機能。

## シグナリング

接続情報の交換

1. サーバーは、各端末の接続に必要な情報(ICE candidate, SDP offer/answer)をインメモリーKVSに保持するリクエストパス(POST /signal)を認証済みクライアントへ開放する。リクエストでは、他クライアントを指定するフィールド(target)と、クライアント向けの通信種値(kind)を定義し、サーバーは、次回の対象端末のリクエスト時にレスポンスに含める。
2. サーバーは、各端末からのオンライン状態の申告を受けるリクエストパス(POST /poll)を開放し、保留中だった他端末のP2P通信希望申請をレスポンスで返却する。
3. クライアントはPOST /pollに定期リクエストを実行する。希望申請があれば1つを選び、STUNサーバーへリクエストして自身のICE candidate(Public IP:port)を取得する。取得できたtrickle ICEから順次POST /signalのパスへ送信する。自身のメディア形式(SDP offer/answer)、選んだ申請者情報も同様に送信する。
4. 3の後、次の申請者端末のサーバーアクセス時に、レスポンスで相手端末のSDP answerとICE candidateが得られるので、以降は各端末のwebRTC APIとTURNサーバーへのフォールバックアクセスに任せる。

---

## 用語

| 用語 | 説明 |
|-|-|
| Offerer | P2P 接続を開始する側。SDP offer を生成する |
| Answerer | offer を受け取り SDP answer を返す側 |
| SDP | Session Description Protocol。メディア形式・コーデック・ネットワーク情報を記述 |
| ICE candidate | STUN/TURN 経由で取得した通信可能なアドレス候補 (IP:port + protocol) |
| Trickle ICE | candidate が揃う前から順次サーバーへ送信して接続を早める手法 |

---

## ワイヤ

サーバー側の経路と検査は [Server.md](./Server.md) の API 表と Meet 節に従う。

| 経路 | リクエスト | レスポンス |
|-|-|-|
| `POST /v1/meet/poll` | なし | `[count: u16]([from: u64][kind: u8][payload: bytes])*` |
| `POST /v1/meet/signal` | `[target: u64][kind: u8][payload: bytes]` | 204。409 は相手の待ち行列が満杯 |
| `GET /v1/meet/ice` | なし | TURN 提供元の `iceServers` JSON。前段が `auth_request` の後に提供元へ proxy する |

| kind | payload |
|-|-|
| 1 offer | SDP 文字列 |
| 2 answer | SDP 文字列 |
| 3 candidate | `[media_line: u16][media_id: str][candidate: str]` |

`from` はサーバーがセッションの `peer` から埋める。`peer` はログインごとに発行される u64 で、相手の `peer` を知る経路はこの設計の範囲外。

app 側では、この順序と表現への整合を `Command::Fetch` の組み立て直前(`Outgoing::encode`)と `Event::Fetched` のデコード直後(`Signal::decode`)でのみ取る。`Meet` の状態遷移は型付きの `Signal` / `Payload` だけを扱い、バイト列を見ない。

---

## Client

`RTCPeerConnection` は Window にしか公開されておらず Worker からは生成できない。app の `Fetch` と同じく、wasm は `Command` を出すだけにして、`init.js` が WebRTC API を実行し、結果を `Event` で wasm へ返す。

```
init.js (Window)
  ├── fetch_request()            Command::Fetch → fetch → Event::FetchChunk
  ├── open_connection(peer, role)   GET /v1/meet/ice → new RTCPeerConnection
  │     ├── onicecandidate       → Event::Candidate
  │     ├── Offerer なら createOffer → setLocalDescription → Event::Description(offer)
  │     └── onconnectionstatechange → Event::ConnectionState
  ├── set_remote_description(peer, kind, sdp)
  │     offer なら createAnswer → setLocalDescription → Event::Description(answer)
  ├── add_candidate(peer, media_line, media_id, candidate)
  ├── send_data(peer, data)      RTCDataChannel.send
  └── close_connection(peer)

worker.js
  └─ Wasm meet.rs (core + alloc)
       ├── tick(now)                 POLL_INTERVAL ごとに Command::Fetch(POST /meet/poll)
       ├── connect(peer)             Command::OpenConnection(Offerer)
       ├── on_signal(Signal)         kind ごとに OpenConnection / SetRemoteDescription / AddCandidate
       ├── on_description(peer, ..)  Command::Fetch(POST /meet/signal, Offer or Answer)
       └── on_candidate(peer, ..)    Command::Fetch(POST /meet/signal, Candidate)
```

### ICE 収集の流れ

1. `init.js` が `RTCPeerConnection` 生成前に `GET /v1/meet/ice` で `iceServers` を取得する
2. `onicecandidate` で candidate を得るたびに `Event::Candidate` を wasm へ渡す (trickle ICE)
3. wasm は受け取るたびに `Kind::Candidate` で `POST /v1/meet/signal` を出す
4. 収集完了を待たない。相手の candidate は `poll` の都度 `AddCandidate` で追加する

### host.rs への追加

```rust
pub enum Command {
    Fetch { request: u32, method: Method, path: String, body: Vec<u8> },
    OpenConnection { peer: u64, role: Role },
    SetRemoteDescription { peer: u64, kind: Kind, sdp: String },
    AddCandidate { peer: u64, media_line: u16, media_id: String, candidate: String },
    SendData { peer: u64, data: Vec<u8> },
    CloseConnection { peer: u64 },
}

pub enum Event {
    Fetched(Response),
    Description { peer: u64, kind: Kind, sdp: String },
    Candidate { peer: u64, media_line: u16, media_id: String, candidate: String },
    Data { peer: u64, data: Vec<u8> },
    ConnectionState { peer: u64, state: ConnectionState },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Offer = 1,
    Answer,
    Candidate,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Offerer = 1,
    Answerer,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConnectionState {
    Connecting = 1,
    Open,
    Closed,
    Failed,
}

crate::wire_enum!(u8: Kind, Role, ConnectionState);

impl Kind {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Offer),
            2 => Some(Self::Answer),
            3 => Some(Self::Candidate),
            _ => None,
        }
    }
}
```

### meet.rs

```rust
use alloc::{collections::BTreeMap, string::String, vec::Vec};

use crate::{
    host::{Command, ConnectionState, Kind, Method, Role},
    Input, Output,
};

pub const POLL_INTERVAL: u64 = 2;

pub enum Payload {
    Description(String),
    Candidate { media_line: u16, media_id: String, candidate: String },
}

pub struct Signal {
    pub from: u64,
    pub kind: Kind,
    pub payload: Payload,
}

impl Input<'_> for Signal {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        let from = u64::decode(input)?;
        let kind = Kind::from_u8(u8::decode(input)?)?;
        let mut bytes = <&[u8]>::decode(input)?;
        let payload = match kind {
            Kind::Offer | Kind::Answer => {
                Payload::Description(String::from(core::str::from_utf8(bytes).ok()?))
            }
            Kind::Candidate => {
                let media_line = u16::decode(&mut bytes)?;
                let media_id = String::decode(&mut bytes)?;
                let candidate = String::decode(&mut bytes)?;
                bytes.is_empty().then_some(Payload::Candidate { media_line, media_id, candidate })?
            }
        };
        Some(Self { from, kind, payload })
    }
}

pub fn decode_polled(body: &[u8]) -> Vec<Signal> {
    let mut rest = body;
    let Some(count) = u16::decode(&mut rest) else { return Vec::new() };
    (0..count).map_while(|_| Signal::decode(&mut rest)).collect()
}

pub struct Outgoing<'a> {
    pub target: u64,
    pub kind: Kind,
    pub payload: &'a Payload,
}

impl Output for Outgoing<'_> {
    fn encode(&self, output: &mut Vec<u8>) {
        self.target.encode(output);
        self.kind.encode(output);
        let mut bytes = Vec::new();
        match self.payload {
            Payload::Description(sdp) => bytes.extend_from_slice(sdp.as_bytes()),
            Payload::Candidate { media_line, media_id, candidate } => {
                media_line.encode(&mut bytes);
                str::encode(media_id, &mut bytes);
                str::encode(candidate, &mut bytes);
            }
        }
        <[u8]>::encode(&bytes, output);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Opening = 1,
    Described,
    Connected,
    Closed,
}

pub struct Peer {
    pub role: Role,
    pub phase: Phase,
}

pub struct Meet {
    peers: BTreeMap<u64, Peer>,
    next_request: u32,
    poll_request: Option<u32>,
    polled_at: u64,
}

impl Meet {
    pub const fn new() -> Self {
        Self { peers: BTreeMap::new(), next_request: 1, poll_request: None, polled_at: 0 }
    }

    pub fn tick(&mut self, now: u64) -> Vec<Command> {
        if self.poll_request.is_some() || now.saturating_sub(self.polled_at) < POLL_INTERVAL {
            return Vec::new();
        }
        let request = self.issue();
        self.poll_request = Some(request);
        self.polled_at = now;
        alloc::vec![Command::Fetch {
            request,
            method: Method::Post,
            path: String::from("/meet/poll"),
            body: Vec::new(),
        }]
    }

    pub fn connect(&mut self, peer: u64) -> Vec<Command> {
        self.peers.insert(peer, Peer { role: Role::Offerer, phase: Phase::Opening });
        alloc::vec![Command::OpenConnection { peer, role: Role::Offerer }]
    }

    pub fn is_polled(&mut self, request: u32) -> bool {
        if self.poll_request == Some(request) {
            self.poll_request = None;
            return true;
        }
        false
    }

    pub fn on_signal(&mut self, signal: Signal) -> Vec<Command> {
        let Signal { from, kind, payload } = signal;
        match (kind, payload) {
            (Kind::Offer, Payload::Description(sdp)) => {
                self.peers.insert(from, Peer { role: Role::Answerer, phase: Phase::Opening });
                alloc::vec![
                    Command::OpenConnection { peer: from, role: Role::Answerer },
                    Command::SetRemoteDescription { peer: from, kind, sdp },
                ]
            }
            (Kind::Answer, Payload::Description(sdp)) => match self.peers.get_mut(&from) {
                Some(peer) if peer.role == Role::Offerer => {
                    peer.phase = Phase::Described;
                    alloc::vec![Command::SetRemoteDescription { peer: from, kind, sdp }]
                }
                _ => Vec::new(),
            },
            (Kind::Candidate, Payload::Candidate { media_line, media_id, candidate }) => {
                if !self.peers.contains_key(&from) {
                    return Vec::new();
                }
                alloc::vec![Command::AddCandidate { peer: from, media_line, media_id, candidate }]
            }
            _ => Vec::new(),
        }
    }

    pub fn on_description(&mut self, peer: u64, kind: Kind, sdp: String) -> Vec<Command> {
        if let Some(entry) = self.peers.get_mut(&peer) {
            entry.phase = Phase::Described;
        }
        alloc::vec![self.signal(peer, kind, &Payload::Description(sdp))]
    }

    pub fn on_candidate(
        &mut self,
        peer: u64,
        media_line: u16,
        media_id: String,
        candidate: String,
    ) -> Vec<Command> {
        let payload = Payload::Candidate { media_line, media_id, candidate };
        alloc::vec![self.signal(peer, Kind::Candidate, &payload)]
    }

    pub fn on_connection_state(&mut self, peer: u64, state: ConnectionState) -> Vec<Command> {
        let Some(entry) = self.peers.get_mut(&peer) else { return Vec::new() };
        match state {
            ConnectionState::Connecting => Vec::new(),
            ConnectionState::Open => {
                entry.phase = Phase::Connected;
                Vec::new()
            }
            ConnectionState::Closed | ConnectionState::Failed => {
                self.peers.remove(&peer);
                alloc::vec![Command::CloseConnection { peer }]
            }
        }
    }

    fn signal(&mut self, target: u64, kind: Kind, payload: &Payload) -> Command {
        let mut body = Vec::new();
        Outgoing { target, kind, payload }.encode(&mut body);
        Command::Fetch {
            request: self.issue(),
            method: Method::Post,
            path: String::from("/meet/signal"),
            body,
        }
    }

    fn issue(&mut self) -> u32 {
        let request = self.next_request;
        self.next_request = self.next_request.wrapping_add(1).max(1);
        request
    }
}
```

`app.rs` は `Event::Fetched(response)` を受けたとき、`meet.is_polled(response.request)` かつ `status == 200` なら `decode_polled(&response.body)` の各 `Signal` を `meet.on_signal` に渡す。

### init.js

```js
const KIND_OFFER = 1;
const KIND_ANSWER = 2;
const connections = new Map();

async function open_connection(peer, role) {
    const response = await fetch(`${API_BASE}/meet/ice`, { credentials: "same-origin" });
    const { iceServers } = await response.json();
    const connection = new RTCPeerConnection({ iceServers });
    const channel = connection.createDataChannel("meet", { negotiated: true, id: 0 });
    connections.set(peer, { connection, channel });
    connection.onicecandidate = (e) => {
        if (e.candidate) emit_candidate(peer, e.candidate.sdpMLineIndex, e.candidate.sdpMid, e.candidate.candidate);
    };
    connection.onconnectionstatechange = () => emit_connection_state(peer, connection.connectionState);
    channel.onmessage = (e) => emit_data(peer, new Uint8Array(e.data));
    if (role === ROLE_OFFERER) {
        await connection.setLocalDescription(await connection.createOffer());
        emit_description(peer, KIND_OFFER, connection.localDescription.sdp);
    }
}

async function set_remote_description(peer, kind, sdp) {
    const { connection } = connections.get(peer);
    await connection.setRemoteDescription({ type: kind === KIND_OFFER ? "offer" : "answer", sdp });
    if (kind === KIND_OFFER) {
        await connection.setLocalDescription(await connection.createAnswer());
        emit_description(peer, KIND_ANSWER, connection.localDescription.sdp);
    }
}

async function add_candidate(peer, media_line, media_id, candidate) {
    const { connection } = connections.get(peer);
    await connection.addIceCandidate({ sdpMLineIndex: media_line, sdpMid: media_id, candidate });
}
```

### 未決

- `setRemoteDescription` 前に届いた candidate の扱い。`init.js` で保留するか、wasm 側で `Phase::Described` まで溜めるか
- 同一 `peer` から 2 本目の offer が来たときの扱い
- `POLL_INTERVAL` の値と、長時間ポーリングへの差し替え時期
- ホスト側の失敗からの復帰。`fetch` の status 0、`signal` の 409、`ConnectionState::Failed` のそれぞれで、再試行するか `peers` から外すか
- `Phase` / `Role` をワイヤに出す必要が生じたときの `bit::field!` 化

---

## Cargo features

WebRTC API は `init.js` が呼ぶので、app 側の `web-sys` に `Rtc*` の feature は追加しない。

```toml
[features]
meet = []
```
