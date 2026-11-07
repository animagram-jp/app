use alloc::vec::Vec;
use core::{
    cmp::PartialEq,
    ops::{Deref, DerefMut},
    option::Option::{self, None, Some},
    primitive::{i64, u8, u16, u32, u64, u128},
    result::Result::{self},
};

use crate::{
    Input, Output,
    bit::{Bits, Zero},
    file_store::FileStore,
    parse,
    time::{Stamp, Zone, from_unix_timestamp_in_ms},
};

#[derive(Debug)]
pub enum ObjectError {
    Decode(u32),
}

// --- record ---

/// ```
/// # use app::object::Head;
/// let mut head = Head::new(7);
/// head.stamp(946684800000);
/// let created = head.created;
/// head.touch(946684860000);
/// assert_eq!(head.created, created);
/// assert_ne!(head.modified, created);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Head {
    pub id:       u32,
    pub created:  Stamp,
    pub modified: Stamp,
}

impl Head {
    pub fn new(id: u32) -> Self {
        Self { id, created: Stamp::ZERO, modified: Stamp::ZERO }
    }

    pub fn stamp(&mut self, now_in_ms: u64) {
        self.created = from_unix_timestamp_in_ms(now_in_ms as i64, Zone::ZERO);
        self.modified = self.created;
    }

    pub fn touch(&mut self, now_in_ms: u64) {
        self.modified = from_unix_timestamp_in_ms(now_in_ms as i64, Zone::ZERO);
    }
}

impl Input<'_> for Head {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        Some(Self {
            id:       u32::decode(input)?,
            created:  Stamp::decode(input)?,
            modified: Stamp::decode(input)?,
        })
    }
}

impl Output for Head {
    fn encode(&self, output: &mut Vec<u8>) {
        self.id.encode(output);
        self.created.encode(output);
        self.modified.encode(output);
    }
}

pub trait Body: Output + for<'a> Input<'a> {
    type Kind: Bits<Value = u8> + PartialEq;
    const KIND: Self::Kind;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Record<B: Body> {
    pub head: Head,
    pub body: B,
}

impl<B: Body> Record<B> {
    pub fn new(id: u32, body: B) -> Self {
        Self { head: Head::new(id), body }
    }
}

impl<B: Body> Deref for Record<B> {
    type Target = B;

    fn deref(&self) -> &B {
        &self.body
    }
}

impl<B: Body> DerefMut for Record<B> {
    fn deref_mut(&mut self) -> &mut B {
        &mut self.body
    }
}

impl<B: Body> Input<'_> for Record<B> {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        if B::Kind::from(u8::decode(input)?) != B::KIND {
            return None;
        }
        Some(Self { head: Head::decode(input)?, body: B::decode(input)? })
    }
}

impl<B: Body> Output for Record<B> {
    fn encode(&self, output: &mut Vec<u8>) {
        let kind: u8 = B::KIND.into();
        kind.encode(output);
        self.head.encode(output);
        self.body.encode(output);
    }
}

pub fn new_uid(now_in_ms: u64) -> u128 {
    let unix_timestamp_in_ms = u128::from(now_in_ms);
    let mut bytes = [0u8; 10];
    getrandom::fill(&mut bytes).unwrap();
    let random_a = u128::from(u16::from_le_bytes([bytes[0], bytes[1]]) & 0x0FFF);
    let random_b = u128::from(u64::from_le_bytes(bytes[2..].try_into().unwrap()) >> 2);
    (unix_timestamp_in_ms & 0xFFFF_FFFF_FFFF) << 80 | 7 << 76 | random_a << 64 | 2 << 62 | random_b
}

// --- store ---

pub fn get<B: Body>(store: &impl FileStore, id: u32) -> Option<Record<B>> {
    parse(store.get(id)?)
}

pub fn set<B: Body>(store: &mut impl FileStore, record: &Record<B>) {
    store.set(record.head.id, record.to_bytes());
}

pub fn list<B: Body>(store: &impl FileStore) -> Result<Vec<Record<B>>, ObjectError> {
    let kind: u8 = B::KIND.into();
    store
        .range(0, u32::MAX)
        .filter(|(_, bytes)| bytes.first() == Some(&kind))
        .map(|(id, bytes)| parse(bytes).ok_or(ObjectError::Decode(id)))
        .collect()
}

pub fn exists(store: &impl FileStore) -> bool {
    store.range(0, u32::MAX).next().is_some()
}
