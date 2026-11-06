#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
use alloc::collections::BTreeMap;
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
use core::option::Option::None;
use core::{
    fmt::{self, Display, Formatter},
    option::Option::{self, Some},
    primitive::{u8, u16, u32, u64, usize},
    result::Result::{self, Err, Ok},
};

use arbitrary_int::u1;

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
use crate::file_store::FileStore;
use crate::{
    FrameError, Input, Output, WireError,
    bit::{self, Zero},
    file_store::FileStoreError,
    parse,
};

pub const IDENTIFIER_LENGTH: usize = 16;
pub const CIPHERTEXT_SIZE_MAX: usize = 64 * 1024;
pub const PULL_SIZE_MAX: usize = 768 * 1024;
pub const PUSH_COUNT_MAX: usize = 1024;

pub type Identifier = [u8; IDENTIFIER_LENGTH];

bit::field! {
    pub Deleted(u1),
}

bit::composite! {
    pub struct Flags { Deleted }
}

impl Flags {
    pub fn tombstone() -> Self {
        let mut flags = Self::ZERO;
        flags.set(Deleted(u1::new(1)));
        flags
    }

    pub fn is_deleted(&self) -> bool {
        self.get::<Deleted>() != Deleted::ZERO
    }
}

impl Input<'_> for Flags {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        let byte = u8::decode(input)?;
        (byte >> Self::BITS == 0).then(|| Self::from_le_bytes([byte]))
    }
}

#[derive(Debug, PartialEq)]
pub enum SyncError {
    Frame(FrameError),
    Store(String),
    Record(u32),
    Ciphertext(usize),
    PushCount(usize),
}

impl Display for SyncError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl WireError for SyncError {
    fn identifiers(&self, path: &mut Vec<u16>) {
        match self {
            SyncError::Frame(error) => {
                path.push(1);
                error.identifiers(path);
            }
            SyncError::Store(_) => path.push(2),
            SyncError::Record(_) => path.push(3),
            SyncError::Ciphertext(_) => path.push(4),
            SyncError::PushCount(_) => path.push(6),
        }
    }

    fn detail(&self) -> String {
        match self {
            SyncError::Frame(error) => error.detail(),
            SyncError::Store(message) => message.clone(),
            SyncError::Record(id) => format!("sync record {id}"),
            other => other.to_string(),
        }
    }
}

impl From<FileStoreError> for SyncError {
    fn from(error: FileStoreError) -> Self {
        SyncError::Store(error.detail())
    }
}

impl From<FrameError> for SyncError {
    fn from(error: FrameError) -> Self {
        SyncError::Frame(error)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub identifier: Identifier,
    pub version:    u64,
    pub flags:      Flags,
    pub ciphertext: Vec<u8>,
}

impl Record {
    pub fn is_deleted(&self) -> bool {
        self.flags.is_deleted()
    }

    pub fn size(&self) -> usize {
        IDENTIFIER_LENGTH + 8 + Flags::BYTES + 4 + self.ciphertext.len()
    }
}

impl Output for Record {
    fn encode(&self, output: &mut Vec<u8>) {
        self.version.encode(output);
        self.flags.encode(output);
        self.identifier.encode(output);
        self.ciphertext.encode(output);
    }
}

impl Input<'_> for Record {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        Some(Self {
            version:    Input::decode(input)?,
            flags:      Input::decode(input)?,
            identifier: Input::decode(input)?,
            ciphertext: Input::decode(input)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub identifier: Identifier,
    pub base:       u64,
    pub flags:      Flags,
    pub ciphertext: Vec<u8>,
}

impl Input<'_> for Entry {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        Some(Self {
            identifier: Input::decode(input)?,
            base:       Input::decode(input)?,
            flags:      Input::decode(input)?,
            ciphertext: Input::decode(input)?,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Accepted = 1,
    Conflict,
}

crate::wire_enum!(u8: Status);

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Accepted(u64),
    Conflict(Record),
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
fn check_entry(entry: &Entry) -> Result<(), SyncError> {
    if entry.ciphertext.len() > CIPHERTEXT_SIZE_MAX {
        return Err(SyncError::Ciphertext(entry.ciphertext.len()));
    }
    Ok(())
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub struct Collection<S: FileStore> {
    store:         S,
    by_identifier: BTreeMap<Identifier, u32>,
    by_version:    BTreeMap<u64, u32>,
    head:          u64,
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
impl<S: FileStore> Collection<S> {
    pub fn new(store: S) -> Result<Self, SyncError> {
        let mut by_identifier = BTreeMap::new();
        let mut by_version = BTreeMap::new();
        let mut head = 0;
        for (id, bytes) in store.range(0, u32::MAX) {
            let record = Record::from_bytes(bytes).map_err(|_| SyncError::Record(id))?;
            if record.version == 0
                || by_identifier.insert(record.identifier, id).is_some()
                || by_version.insert(record.version, id).is_some()
            {
                return Err(SyncError::Record(id));
            }
            head = head.max(record.version);
        }
        Ok(Self { store, by_identifier, by_version, head })
    }

    pub fn head(&self) -> u64 {
        self.head
    }

    pub fn get(&self, identifier: &Identifier) -> Option<Record> {
        self.store.decode(*self.by_identifier.get(identifier)?)
    }

    pub fn pull(&self, since: u64, budget: usize) -> (Vec<Record>, bool) {
        let mut records = Vec::new();
        let mut used = 0usize;
        let mut newer = self.by_version.range(since.saturating_add(1)..);
        for (_, id) in newer.by_ref() {
            let Some(record): Option<Record> = self.store.decode(*id) else { continue };
            if !records.is_empty() && used + record.size() > budget {
                return (records, true);
            }
            used += record.size();
            records.push(record);
        }
        (records, false)
    }

    pub fn push(&mut self, entries: &[Entry]) -> Result<Vec<Outcome>, SyncError> {
        if entries.len() > PUSH_COUNT_MAX {
            return Err(SyncError::PushCount(entries.len()));
        }
        for entry in entries {
            check_entry(entry)?;
        }
        let mut outcomes = Vec::with_capacity(entries.len());
        for entry in entries {
            let current = self.get(&entry.identifier);
            let matched = match &current {
                Some(record) => record.version == entry.base,
                None => entry.base == 0,
            };
            if !matched {
                outcomes.push(Outcome::Conflict(current.unwrap_or(Record {
                    identifier: entry.identifier,
                    version:    0,
                    flags:      Flags::tombstone(),
                    ciphertext: Vec::new(),
                })));
                continue;
            }
            self.head += 1;
            let record = Record {
                identifier: entry.identifier,
                version:    self.head,
                flags:      entry.flags,
                ciphertext: if entry.flags.is_deleted() {
                    Vec::new()
                } else {
                    entry.ciphertext.clone()
                },
            };
            let id = match self.by_identifier.get(&entry.identifier) {
                Some(id) => *id,
                None => self.store.issue_id(),
            };
            if let Some(previous) = current {
                self.by_version.remove(&previous.version);
            }
            self.store.set(id, record.to_bytes());
            self.by_identifier.insert(entry.identifier, id);
            self.by_version.insert(record.version, id);
            outcomes.push(Outcome::Accepted(record.version));
        }
        Ok(outcomes)
    }

    pub fn save(&mut self) -> Result<(), SyncError> {
        Ok(self.store.save()?)
    }

    pub fn compact(&mut self) -> Result<(), SyncError> {
        Ok(self.store.compact()?)
    }
}

pub fn encode_pull(head: u64, records: &[Record], more: bool) -> Vec<u8> {
    let mut output = Vec::new();
    head.encode(&mut output);
    (more as u8).encode(&mut output);
    (records.len() as u32).encode(&mut output);
    for record in records {
        record.identifier.encode(&mut output);
        record.version.encode(&mut output);
        record.flags.encode(&mut output);
        record.ciphertext.encode(&mut output);
    }
    output
}

pub fn decode_pull(body: &[u8]) -> Result<u64, SyncError> {
    parse(body).ok_or(SyncError::Frame(FrameError::Decode))
}

pub fn decode_push(body: &[u8]) -> Result<Vec<Entry>, SyncError> {
    let mut rest = body;
    let count = u32::decode(&mut rest).ok_or(FrameError::Decode)? as usize;
    if count > PUSH_COUNT_MAX {
        return Err(SyncError::PushCount(count));
    }
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        entries.push(Entry::decode(&mut rest).ok_or(FrameError::Decode)?);
    }
    if !rest.is_empty() {
        return Err(SyncError::Frame(FrameError::Decode));
    }
    Ok(entries)
}

pub fn encode_push(head: u64, outcomes: &[Outcome]) -> Vec<u8> {
    let mut output = Vec::new();
    head.encode(&mut output);
    (outcomes.len() as u32).encode(&mut output);
    for outcome in outcomes {
        match outcome {
            Outcome::Accepted(version) => {
                Status::Accepted.encode(&mut output);
                version.encode(&mut output);
                Flags::ZERO.encode(&mut output);
                [0u8; 0][..].encode(&mut output);
            }
            Outcome::Conflict(record) => {
                Status::Conflict.encode(&mut output);
                record.version.encode(&mut output);
                record.flags.encode(&mut output);
                record.ciphertext.encode(&mut output);
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn identifier(number: u8) -> Identifier {
        [number; IDENTIFIER_LENGTH]
    }

    pub(super) fn entry(number: u8, base: u64, ciphertext: &[u8]) -> Entry {
        Entry {
            identifier: identifier(number),
            base,
            flags: Flags::ZERO,
            ciphertext: ciphertext.to_vec(),
        }
    }

    #[test]
    fn a_record_round_trips_through_the_wire() {
        let record = Record {
            identifier: identifier(3),
            version:    9,
            flags:      Flags::tombstone(),
            ciphertext: b"abc".to_vec(),
        };
        let bytes = record.to_bytes();
        assert_eq!(Record::from_bytes(&bytes), Ok(record.clone()));
        assert_eq!(record.size(), bytes.len());
        let mut longer = bytes.clone();
        longer.push(1);
        assert_eq!(Record::from_bytes(&longer), Err(FrameError::Decode));
        assert_eq!(Record::from_bytes(&bytes[..5]), Err(FrameError::Decode));
    }

    #[test]
    fn unknown_flags_are_rejected_at_decode() {
        let mut body = Vec::new();
        1u32.encode(&mut body);
        identifier(1).encode(&mut body);
        0u64.encode(&mut body);
        2u8.encode(&mut body);
        b""[..].encode(&mut body);
        assert_eq!(decode_push(&body), Err(SyncError::Frame(FrameError::Decode)));
    }

    #[test]
    fn push_and_pull_frames_round_trip() {
        let records = [Record {
            identifier: identifier(1),
            version:    2,
            flags:      Flags::ZERO,
            ciphertext: b"x".to_vec(),
        }];
        let bytes = encode_pull(7, &records, true);
        let mut rest = &bytes[..];
        assert_eq!(u64::decode(&mut rest), Some(7));
        assert_eq!(u8::decode(&mut rest), Some(1));
        assert_eq!(u32::decode(&mut rest), Some(1));
        assert_eq!(<[u8; 16]>::decode(&mut rest), Some(identifier(1)));
        assert_eq!(u64::decode(&mut rest), Some(2));
        assert_eq!(u8::decode(&mut rest), Some(0));
        assert_eq!(<&[u8]>::decode(&mut rest), Some(&b"x"[..]));
        assert!(rest.is_empty());

        assert_eq!(decode_pull(&9u64.to_le_bytes()), Ok(9));
        assert_eq!(decode_pull(&[0; 9]), Err(SyncError::Frame(FrameError::Decode)));

        let mut body = Vec::new();
        2u32.encode(&mut body);
        identifier(1).encode(&mut body);
        0u64.encode(&mut body);
        0u8.encode(&mut body);
        b"c1"[..].encode(&mut body);
        identifier(2).encode(&mut body);
        3u64.encode(&mut body);
        Flags::tombstone().encode(&mut body);
        b""[..].encode(&mut body);
        let entries = decode_push(&body).unwrap();
        assert_eq!(
            entries,
            [
                entry(1, 0, b"c1"),
                Entry {
                    identifier: identifier(2),
                    base:       3,
                    flags:      Flags::tombstone(),
                    ciphertext: Vec::new(),
                }
            ]
        );
        assert_eq!(
            decode_push(&(PUSH_COUNT_MAX as u32 + 1).to_le_bytes()),
            Err(SyncError::PushCount(PUSH_COUNT_MAX + 1))
        );
        assert_eq!(decode_push(&[1, 0, 0, 0]), Err(SyncError::Frame(FrameError::Decode)));

        let conflict = Record {
            identifier: identifier(1),
            version:    5,
            flags:      Flags::ZERO,
            ciphertext: b"srv".to_vec(),
        };
        let bytes = encode_push(5, &[Outcome::Accepted(4), Outcome::Conflict(conflict)]);
        let mut rest = &bytes[..];
        assert_eq!(u64::decode(&mut rest), Some(5));
        assert_eq!(u32::decode(&mut rest), Some(2));
        assert_eq!(
            (
                u8::decode(&mut rest),
                u64::decode(&mut rest),
                u8::decode(&mut rest),
                <&[u8]>::decode(&mut rest)
            ),
            (Some(Status::Accepted as u8), Some(4), Some(0), Some(&b""[..]))
        );
        assert_eq!(
            (
                u8::decode(&mut rest),
                u64::decode(&mut rest),
                u8::decode(&mut rest),
                <&[u8]>::decode(&mut rest)
            ),
            (Some(Status::Conflict as u8), Some(5), Some(0), Some(&b"srv"[..]))
        );
        assert!(rest.is_empty());
    }
}

#[cfg(all(test, feature = "server", not(target_arch = "wasm32")))]
mod collection_tests {
    use alloc::boxed::Box;

    use super::{
        tests::{entry, identifier},
        *,
    };
    use crate::{
        Rng, block_on,
        file_store::{MemoryStore, Mount, StoreId},
    };

    type State = BTreeMap<Identifier, (u64, bool, Vec<u8>)>;

    fn store(id: StoreId) -> MemoryStore {
        block_on(MemoryStore::open(id, true)).and_then(MemoryStore::new).unwrap()
    }

    fn collection(id: StoreId) -> Collection<MemoryStore> {
        Collection::new(store(id)).unwrap()
    }

    fn derive_id(name: &str) -> StoreId {
        StoreId { name: Box::leak(format!("user/1/{name}").into_boxed_str()), version: "0" }
    }

    fn tombstone(number: u8, base: u64) -> Entry {
        Entry {
            identifier: identifier(number),
            base,
            flags: Flags::tombstone(),
            ciphertext: b"ignored".to_vec(),
        }
    }

    #[test]
    fn push_assigns_versions_and_reports_conflicts() {
        let mut collection = collection(derive_id("push"));
        let outcomes = collection
            .push(&[entry(1, 0, b"a1"), entry(2, 0, b"b1"), entry(1, 0, b"a2"), entry(3, 5, b"c1")])
            .unwrap();
        assert_eq!(outcomes[0], Outcome::Accepted(1));
        assert_eq!(outcomes[1], Outcome::Accepted(2));
        assert_eq!(
            outcomes[2],
            Outcome::Conflict(Record {
                identifier: identifier(1),
                version:    1,
                flags:      Flags::ZERO,
                ciphertext: b"a1".to_vec(),
            })
        );
        assert_eq!(
            outcomes[3],
            Outcome::Conflict(Record {
                identifier: identifier(3),
                version:    0,
                flags:      Flags::tombstone(),
                ciphertext: Vec::new(),
            })
        );
        assert_eq!(collection.head(), 2);
        assert_eq!(collection.by_identifier.len(), 2);

        let outcomes = collection.push(&[entry(1, 1, b"a2"), tombstone(2, 2)]).unwrap();
        assert_eq!(outcomes, [Outcome::Accepted(3), Outcome::Accepted(4)]);
        let deleted = collection.get(&identifier(2)).unwrap();
        assert!(deleted.is_deleted());
        assert!(deleted.ciphertext.is_empty());
        assert_eq!(collection.get(&identifier(1)).unwrap().ciphertext, b"a2");
        assert_eq!(collection.push(&[entry(2, 4, b"b2")]).unwrap(), [Outcome::Accepted(5)]);
        assert!(!collection.get(&identifier(2)).unwrap().is_deleted());
    }

    #[test]
    fn push_rejects_an_oversized_ciphertext_and_too_many_entries() {
        let mut collection = collection(derive_id("limits"));
        let oversized = Entry {
            identifier: identifier(1),
            base:       0,
            flags:      Flags::ZERO,
            ciphertext: alloc::vec![0; CIPHERTEXT_SIZE_MAX + 1],
        };
        assert_eq!(
            collection.push(&[oversized]),
            Err(SyncError::Ciphertext(CIPHERTEXT_SIZE_MAX + 1))
        );
        let many: Vec<Entry> = (0..=PUSH_COUNT_MAX).map(|_| entry(1, 0, b"")).collect();
        assert_eq!(collection.push(&many), Err(SyncError::PushCount(PUSH_COUNT_MAX + 1)));
        assert!(collection.by_identifier.is_empty());
    }

    #[test]
    fn pull_returns_records_in_version_order_up_to_the_size_limit() {
        let mut collection = collection(derive_id("pull"));
        collection.push(&[entry(1, 0, b"a"), entry(2, 0, b"b"), entry(3, 0, b"c")]).unwrap();
        collection.push(&[entry(1, 1, b"a2")]).unwrap();

        let (all, more) = collection.pull(0, usize::MAX);
        assert!(!more);
        assert_eq!(
            all.iter().map(|record| (record.identifier[0], record.version)).collect::<Vec<_>>(),
            [(2, 2), (3, 3), (1, 4)]
        );

        let (first, more) = collection.pull(0, 1);
        assert!(more);
        assert_eq!(first.len(), 1);
        let (second, more) = collection.pull(first[0].version, first[0].size() * 2 + 1);
        assert!(!more);
        assert_eq!(second.len(), 2);
        let (rest, more) = collection.pull(second[1].version, 1);
        assert!(!more);
        assert!(rest.is_empty());
        assert_eq!(collection.pull(4, usize::MAX).0.len(), 0);
    }

    #[test]
    fn a_reopened_collection_keeps_records_and_rejects_corrupt_ones() {
        let id = derive_id("reopen");
        let mut collection = collection(id);
        collection.push(&[entry(1, 0, b"a"), entry(2, 0, b"b")]).unwrap();
        collection.push(&[entry(2, 2, b"b2")]).unwrap();
        collection.save().unwrap();

        let reopened = self::collection(id);
        assert_eq!(reopened.head(), 3);
        assert_eq!(reopened.get(&identifier(2)).unwrap().version, 3);
        assert_eq!(reopened.pull(1, usize::MAX).0.len(), 1);

        let mut store = store(id);
        store.set(9, alloc::vec![0]);
        store.save().unwrap();
        assert_eq!(Collection::new(self::store(id)).err(), Some(SyncError::Record(9)));

        let mut store = self::store(id);
        store.delete(9);
        store.set(
            10,
            Record {
                identifier: identifier(1),
                version:    3,
                flags:      Flags::ZERO,
                ciphertext: Vec::new(),
            }
            .to_bytes(),
        );
        store.save().unwrap();
        assert_eq!(Collection::new(self::store(id)).err(), Some(SyncError::Record(10)));
    }

    #[test]
    fn two_devices_converge() {
        for seed in 0..60 {
            let mut rng = Rng::new(seed);
            let id = derive_id(&format!("converge_{seed}"));
            let mut collection = collection(id);
            let mut oracle: State = BTreeMap::new();
            let mut devices: [(u64, State); 2] = [(0, BTreeMap::new()), (0, BTreeMap::new())];
            for step in 0..120 {
                let context = format!("seed {seed} step {step}");
                let which = rng.below(2);
                if rng.chance(60) {
                    let identifier = identifier(rng.below(6) as u8);
                    let deleted = rng.chance(20);
                    let length = rng.below(8);
                    let ciphertext = rng.bytes(length);
                    let base =
                        devices[which].1.get(&identifier).map_or(0, |(version, _, _)| *version);
                    let entry = Entry {
                        identifier,
                        base,
                        flags: if deleted { Flags::tombstone() } else { Flags::ZERO },
                        ciphertext: ciphertext.clone(),
                    };
                    let outcome = collection.push(&[entry]).unwrap().remove(0);
                    let truth = oracle.get(&identifier).map_or(0, |(version, _, _)| *version);
                    if truth == base {
                        let Outcome::Accepted(version) = outcome else {
                            panic!("{context}: {outcome:?}")
                        };
                        assert_eq!(version, collection.head(), "{context}");
                        let stored = if deleted { Vec::new() } else { ciphertext };
                        oracle.insert(identifier, (version, deleted, stored.clone()));
                        devices[which].1.insert(identifier, (version, deleted, stored));
                    } else {
                        let Outcome::Conflict(current) = outcome else {
                            panic!("{context}: {outcome:?}")
                        };
                        let (version, deleted, stored) = &oracle[&identifier];
                        assert_eq!(
                            (current.version, current.is_deleted(), &current.ciphertext),
                            (*version, *deleted, stored),
                            "{context}"
                        );
                    }
                } else {
                    let since = devices[which].0;
                    let budget = if rng.chance(50) { usize::MAX } else { 1 + rng.below(80) };
                    let (records, more) = collection.pull(since, budget);
                    let mut last = since;
                    for record in &records {
                        assert!(record.version > last, "{context}");
                        last = record.version;
                        devices[which].1.insert(
                            record.identifier,
                            (record.version, record.is_deleted(), record.ciphertext.clone()),
                        );
                    }
                    devices[which].0 = last;
                    if !more {
                        assert_eq!(last, collection.head(), "{context}");
                        for (identifier, state) in &devices[which].1 {
                            assert_eq!(state, &oracle[identifier], "{context}");
                        }
                    }
                }
                if rng.chance(10) {
                    collection.save().unwrap();
                    collection = self::collection(id);
                }
            }
        }
    }
}
