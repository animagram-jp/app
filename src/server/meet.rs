#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
use alloc::collections::{BTreeMap, VecDeque};
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
use core::result::Result::Err;
use core::{
    fmt::{self, Display, Formatter},
    option::Option::{None, Some},
    primitive::{u8, u16, u64, usize},
    result::Result::{self, Ok},
};

use crate::{FrameError, Input, Output, WireError};

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub const LIFETIME_SECONDS_MAX: u64 = 60;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub const PRESENCE_SECONDS_MAX: u64 = 60;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub const QUEUE_MAX: usize = 32;
pub const PAYLOAD_MAX: usize = 16 * 1024;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Offer = 1,
    Answer,
    Candidate,
}

crate::wire_enum!(u8: Kind);

impl Kind {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Kind::Offer),
            2 => Some(Kind::Answer),
            3 => Some(Kind::Candidate),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum MeetError {
    Kind(u8),
    Payload(usize),
    SelfAddressed,
    Full(u64),
    Frame(FrameError),
}

impl Display for MeetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl WireError for MeetError {
    fn identifiers(&self, path: &mut Vec<u16>) {
        match self {
            MeetError::Kind(_) => path.push(1),
            MeetError::Payload(_) => path.push(2),
            MeetError::SelfAddressed => path.push(3),
            MeetError::Full(_) => path.push(4),
            MeetError::Frame(error) => {
                path.push(5);
                error.identifiers(path);
            }
        }
    }

    fn detail(&self) -> String {
        match self {
            MeetError::Frame(error) => error.detail(),
            other => other.to_string(),
        }
    }

    fn status(&self) -> u16 {
        match self {
            MeetError::Full(_) => 409,
            _ => 400,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Signal {
    pub from:    u64,
    pub kind:    Kind,
    pub payload: Vec<u8>,
    pub at:      u64,
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
impl Signal {
    fn is_expired(&self, now: u64) -> bool {
        now.saturating_sub(self.at) >= LIFETIME_SECONDS_MAX
    }
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub struct Board {
    by_target: BTreeMap<u64, VecDeque<Signal>>,
    last_seen: BTreeMap<u64, u64>,
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
impl Board {
    pub const fn new() -> Self {
        Self { by_target: BTreeMap::new(), last_seen: BTreeMap::new() }
    }

    pub fn len(&self) -> usize {
        self.by_target.values().map(VecDeque::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.by_target.values().all(VecDeque::is_empty)
    }

    pub fn post(
        &mut self,
        now: u64,
        from: u64,
        target: u64,
        kind: Kind,
        payload: Vec<u8>,
    ) -> Result<(), MeetError> {
        if payload.len() > PAYLOAD_MAX {
            return Err(MeetError::Payload(payload.len()));
        }
        if from == target {
            return Err(MeetError::SelfAddressed);
        }
        let queue = self.by_target.entry(target).or_default();
        queue.retain(|signal| !signal.is_expired(now));
        if queue.len() >= QUEUE_MAX {
            return Err(MeetError::Full(target));
        }
        queue.push_back(Signal { from, kind, payload, at: now });
        Ok(())
    }

    pub fn poll(&mut self, now: u64, peer: u64) -> Vec<Signal> {
        self.last_seen.insert(peer, now);
        match self.by_target.remove(&peer) {
            Some(queue) => queue.into_iter().filter(|signal| !signal.is_expired(now)).collect(),
            None => Vec::new(),
        }
    }

    pub fn is_online(&self, now: u64, peer: u64) -> bool {
        self.last_seen
            .get(&peer)
            .is_some_and(|seen| now.saturating_sub(*seen) < PRESENCE_SECONDS_MAX)
    }

    pub fn sweep(&mut self, now: u64) {
        for queue in self.by_target.values_mut() {
            queue.retain(|signal| !signal.is_expired(now));
        }
        self.by_target.retain(|_, queue| !queue.is_empty());
        self.last_seen.retain(|_, seen| now.saturating_sub(*seen) < PRESENCE_SECONDS_MAX);
    }
}

pub fn encode(signals: &[Signal]) -> Vec<u8> {
    let mut output = Vec::new();
    (signals.len() as u16).encode(&mut output);
    for signal in signals {
        signal.from.encode(&mut output);
        signal.kind.encode(&mut output);
        signal.payload.encode(&mut output);
    }
    output
}

pub fn decode(body: &[u8]) -> Result<(u64, Kind, Vec<u8>), MeetError> {
    let mut rest = body;
    let decoded = (|| {
        let target = u64::decode(&mut rest)?;
        let kind = u8::decode(&mut rest)?;
        let payload = Vec::<u8>::decode(&mut rest)?;
        rest.is_empty().then_some((target, kind, payload))
    })();
    let (target, kind, payload) = decoded.ok_or(MeetError::Frame(FrameError::Decode))?;
    let kind = Kind::from_u8(kind).ok_or(MeetError::Kind(kind))?;
    Ok((target, kind, payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_kinds_are_rejected_at_decode() {
        for value in [0u8, 4, 255] {
            let mut body = Vec::new();
            1u64.encode(&mut body);
            value.encode(&mut body);
            b""[..].encode(&mut body);
            assert_eq!(decode(&body), Err(MeetError::Kind(value)));
        }
    }

    #[test]
    fn signals_round_trip_through_the_wire() {
        let signals = [
            Signal { from: 5, kind: Kind::Offer, payload: b"v=0".to_vec(), at: 1 },
            Signal { from: 6, kind: Kind::Candidate, payload: Vec::new(), at: 2 },
        ];
        let bytes = encode(&signals);
        let mut rest = &bytes[..];
        assert_eq!(u16::decode(&mut rest), Some(2));
        assert_eq!(u64::decode(&mut rest), Some(5));
        assert_eq!(u8::decode(&mut rest), Some(Kind::Offer as u8));
        assert_eq!(<&[u8]>::decode(&mut rest), Some(&b"v=0"[..]));
        assert_eq!(u64::decode(&mut rest), Some(6));
        assert_eq!(u8::decode(&mut rest), Some(Kind::Candidate as u8));
        assert_eq!(<&[u8]>::decode(&mut rest), Some(&b""[..]));
        assert!(rest.is_empty());

        let mut body = Vec::new();
        77u64.encode(&mut body);
        Kind::Answer.encode(&mut body);
        b"sdp"[..].encode(&mut body);
        assert_eq!(decode(&body), Ok((77, Kind::Answer, b"sdp".to_vec())));
        assert_eq!(decode(&[1, 2]), Err(MeetError::Frame(FrameError::Decode)));
        let mut trailing = Vec::new();
        1u64.encode(&mut trailing);
        1u8.encode(&mut trailing);
        b""[..].encode(&mut trailing);
        0u8.encode(&mut trailing);
        assert_eq!(decode(&trailing), Err(MeetError::Frame(FrameError::Decode)));
    }
}

#[cfg(all(test, feature = "server", not(target_arch = "wasm32")))]
mod board_tests {
    use alloc::format;

    use super::*;
    use crate::Rng;

    #[test]
    fn poll_returns_posted_signals_in_order() {
        let mut board = Board::new();
        board.post(10, 1, 2, Kind::Offer, b"o".to_vec()).unwrap();
        board.post(11, 1, 2, Kind::Candidate, b"c".to_vec()).unwrap();
        board.post(11, 3, 2, Kind::Offer, b"x".to_vec()).unwrap();
        assert_eq!(board.len(), 3);
        let got = board.poll(12, 2);
        assert_eq!(
            got.iter().map(|signal| (signal.from, signal.kind)).collect::<Vec<_>>(),
            [(1, Kind::Offer), (1, Kind::Candidate), (3, Kind::Offer)]
        );
        assert!(board.poll(12, 2).is_empty());
        assert!(board.is_empty());
    }

    #[test]
    fn post_rejects_a_self_addressed_or_oversized_signal() {
        let mut board = Board::new();
        assert_eq!(board.post(0, 1, 1, Kind::Offer, Vec::new()), Err(MeetError::SelfAddressed));
        let oversized = alloc::vec![0u8; PAYLOAD_MAX + 1];
        assert_eq!(
            board.post(0, 1, 2, Kind::Offer, oversized),
            Err(MeetError::Payload(PAYLOAD_MAX + 1))
        );
        assert!(board.post(0, 1, 2, Kind::Answer, alloc::vec![0u8; PAYLOAD_MAX]).is_ok());
    }

    #[test]
    fn a_full_queue_accepts_again_after_expiry() {
        let mut board = Board::new();
        for i in 0..QUEUE_MAX {
            board.post(i as u64, 1, 2, Kind::Candidate, Vec::new()).unwrap();
        }
        assert_eq!(
            board.post(QUEUE_MAX as u64, 1, 2, Kind::Candidate, Vec::new()),
            Err(MeetError::Full(2))
        );
        board.post(LIFETIME_SECONDS_MAX, 1, 2, Kind::Candidate, b"late".to_vec()).unwrap();
        let got = board.poll(LIFETIME_SECONDS_MAX, 2);
        assert_eq!(got.len(), QUEUE_MAX);
        assert_eq!(got.last().unwrap().payload, b"late");
    }

    #[test]
    fn poll_drops_expired_signals() {
        let mut board = Board::new();
        board.post(0, 1, 2, Kind::Offer, Vec::new()).unwrap();
        board.post(5, 1, 2, Kind::Offer, Vec::new()).unwrap();
        let got = board.poll(LIFETIME_SECONDS_MAX + 2, 2);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].at, 5);
    }

    #[test]
    fn sweep_forgets_expired_signals_and_presence() {
        let mut board = Board::new();
        assert!(!board.is_online(0, 9));
        board.poll(100, 9);
        assert!(board.is_online(100 + PRESENCE_SECONDS_MAX - 1, 9));
        assert!(!board.is_online(100 + PRESENCE_SECONDS_MAX, 9));
        board.post(100, 9, 8, Kind::Offer, Vec::new()).unwrap();
        board.post(150, 9, 7, Kind::Offer, Vec::new()).unwrap();
        board.sweep(100 + LIFETIME_SECONDS_MAX);
        assert_eq!(board.len(), 1);
        assert!(!board.by_target.contains_key(&8));
        assert!(!board.is_online(100 + LIFETIME_SECONDS_MAX, 9));
        board.sweep(150 + LIFETIME_SECONDS_MAX);
        assert!(board.is_empty());
    }

    #[test]
    fn a_random_walk_matches_the_model() {
        for seed in 0..50 {
            let mut rng = Rng::new(seed);
            let mut board = Board::new();
            let mut model: BTreeMap<u64, Vec<(u64, u64)>> = BTreeMap::new();
            let mut now = 0u64;
            for step in 0..200 {
                now += rng.below(10) as u64;
                let context = format!("seed {seed} step {step}");
                match rng.below(10) {
                    0..7 => {
                        let from = rng.below(4) as u64;
                        let target = rng.below(4) as u64;
                        let result = board.post(now, from, target, Kind::Candidate, Vec::new());
                        if from == target {
                            assert_eq!(result, Err(MeetError::SelfAddressed), "{context}");
                            continue;
                        }
                        let queue = model.entry(target).or_default();
                        queue.retain(|(_, at)| now - at < LIFETIME_SECONDS_MAX);
                        if queue.len() >= QUEUE_MAX {
                            assert_eq!(result, Err(MeetError::Full(target)), "{context}");
                        } else {
                            assert_eq!(result, Ok(()), "{context}");
                            queue.push((from, now));
                        }
                    }
                    7..9 => {
                        let peer = rng.below(4) as u64;
                        let got: Vec<(u64, u64)> =
                            board.poll(now, peer).iter().map(|s| (s.from, s.at)).collect();
                        let mut expected = model.remove(&peer).unwrap_or_default();
                        expected.retain(|(_, at)| now - at < LIFETIME_SECONDS_MAX);
                        assert_eq!(got, expected, "{context}");
                    }
                    _ => {
                        board.sweep(now);
                        for queue in model.values_mut() {
                            queue.retain(|(_, at)| now - at < LIFETIME_SECONDS_MAX);
                        }
                        model.retain(|_, queue| !queue.is_empty());
                        assert_eq!(
                            board.len(),
                            model.values().map(Vec::len).sum::<usize>(),
                            "{context}"
                        );
                    }
                }
            }
        }
    }
}
