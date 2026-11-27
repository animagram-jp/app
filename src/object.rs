use alloc::{string::String, vec, vec::Vec};
use core::{
    clone::Clone,
    cmp::PartialEq,
    convert::Into,
    fmt::Debug,
    iter::Iterator,
    marker::Copy,
    ops::{Deref, DerefMut},
    option::Option::{self, None, Some},
    primitive::{i64, u8, u16, u32, u64, u128},
    result::Result::{self},
};

use arbitrary_int::u2;

use crate::{
    Input, Output,
    bit::{Bits, Zero},
    event::store::Persistence,
    file_store::FileStore,
    host::{
        Attribute, Command, FnName,
        dom::{Id, Tag},
    },
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

// --- toast ---

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ToastPart {
    Slot(u32),
    Text(u32),
}

pub struct Toast {
    last: u2,
}

impl Zero for Toast {
    const ZERO: Self = Self { last: u2::new(0) };
}

impl Toast {
    pub const SLOTS: u32 = 2;

    pub fn to_dom(part: ToastPart) -> Id {
        match part {
            ToastPart::Slot(n) => Id::new(&[(Tag::Toast, None), (Tag::Button, Some(n))]),
            ToastPart::Text(n) => {
                Id::new(&[(Tag::Toast, None), (Tag::Button, Some(n)), (Tag::P, None)])
            }
        }
    }

    pub fn from_dom(id: &Id) -> Option<ToastPart> {
        let path: Vec<(Tag, Option<u32>)> = id.0.iter().map(|s| (s.tag, s.n)).collect();
        let part = match path.as_slice() {
            [(Tag::Toast, None), (Tag::Button, Some(n))] => ToastPart::Slot(*n),
            [(Tag::Toast, None), (Tag::Button, Some(n)), (Tag::P, None)] => ToastPart::Text(*n),
            _ => return None,
        };
        let (ToastPart::Slot(n) | ToastPart::Text(n)) = part;
        (1..=Self::SLOTS).contains(&n).then_some(part)
    }

    pub fn show(&mut self, message: impl Into<String>) -> Vec<Command> {
        let slot = u32::from(self.last.value()) % Self::SLOTS + 1;
        self.last = u2::new(slot as u8);
        vec![
            Command::SetText { id: Self::to_dom(ToastPart::Text(slot)), value: message.into() },
            Command::JsFn { id: Self::to_dom(ToastPart::Slot(slot)), name: FnName::ShowToast },
        ]
    }

    pub fn hide(slot: u32) -> Command {
        Command::JsFn { id: Self::to_dom(ToastPart::Slot(slot)), name: FnName::HideToast }
    }
}

#[cfg(test)]
impl Toast {
    pub fn parts() -> impl Iterator<Item = ToastPart> {
        (1..=Self::SLOTS).flat_map(|n| [ToastPart::Slot(n), ToastPart::Text(n)])
    }
}

// --- preservation ---

pub struct Preservation<const N: u32> {
    persistence: Persistence,
}

impl<const N: u32> Preservation<N> {
    pub const fn new(persistence: Persistence) -> Self {
        Self { persistence }
    }

    pub fn to_dom() -> Id {
        Id::new(&[(Tag::Header, None), (Tag::Input, Some(N))])
    }

    pub fn persistence(&self) -> Persistence {
        self.persistence
    }

    pub fn saved(&self) -> bool {
        self.persistence.saved()
    }

    pub fn mark(&mut self, persistence: Persistence) -> Command {
        self.persistence = persistence;
        self.draw()
    }

    pub fn draw(&self) -> Command {
        let id = Self::to_dom();
        if self.saved() {
            Command::SetAttribute { id, attribute: Attribute::Checked, value: String::new() }
        } else {
            Command::RemoveAttribute { id, attribute: Attribute::Checked }
        }
    }
}

#[cfg(test)]
mod toast_tests {
    use alloc::format;

    use super::*;
    use crate::host::dom::Segment;

    #[test]
    fn every_toast_part_survives_the_dom_round_trip_under_a_distinct_id() {
        let parts: Vec<ToastPart> = Toast::parts().collect();
        assert_eq!(parts.len(), 2 * Toast::SLOTS as usize);
        for (index, &part) in parts.iter().enumerate() {
            let id = Toast::to_dom(part);
            assert_eq!(Toast::from_dom(&id), Some(part), "{part:?}");
            for &other in &parts[index + 1..] {
                assert_ne!(id, Toast::to_dom(other), "{part:?} {other:?}");
            }
        }
    }

    #[test]
    fn ids_outside_the_slots_belong_to_no_toast() {
        for n in [0, Toast::SLOTS + 1, u32::MAX - 1] {
            assert_eq!(Toast::from_dom(&Toast::to_dom(ToastPart::Slot(n))), None, "{n}");
            assert_eq!(Toast::from_dom(&Toast::to_dom(ToastPart::Text(n))), None, "{n}");
        }
        let mut deeper = Toast::to_dom(ToastPart::Text(1));
        deeper.0.push(Segment { tag: Tag::Span, n: None });
        assert_eq!(Toast::from_dom(&deeper), None);
        assert_eq!(Toast::from_dom(&Id::new(&[(Tag::Toast, None)])), None);
    }

    #[test]
    fn each_message_takes_the_slot_after_the_last_one() {
        let mut toast = Toast::ZERO;
        for index in 0..(4 * Toast::SLOTS) {
            let slot = index % Toast::SLOTS + 1;
            let message = format!("message {index}");
            assert_eq!(
                toast.show(message.clone()),
                [
                    Command::SetText {
                        id:    Toast::to_dom(ToastPart::Text(slot)),
                        value: message,
                    },
                    Command::JsFn {
                        id:   Toast::to_dom(ToastPart::Slot(slot)),
                        name: FnName::ShowToast,
                    },
                ],
                "{index}"
            );
            assert_eq!(
                Toast::hide(slot),
                Command::JsFn {
                    id:   Toast::to_dom(ToastPart::Slot(slot)),
                    name: FnName::HideToast,
                }
            );
        }
    }
}

#[cfg(test)]
mod preservation_tests {
    use super::*;

    #[test]
    fn the_checkbox_sits_at_the_header_input_given_by_its_type() {
        assert_eq!(
            Preservation::<1>::to_dom(),
            Id::new(&[(Tag::Header, None), (Tag::Input, Some(1))])
        );
        assert_eq!(
            Preservation::<2>::to_dom(),
            Id::new(&[(Tag::Header, None), (Tag::Input, Some(2))])
        );
    }

    #[test]
    fn every_mark_checks_the_box_exactly_when_the_new_persistence_is_saved() {
        for &from in Persistence::LIST {
            for &to in Persistence::LIST {
                let mut preservation = Preservation::<2>::new(from);
                let command = preservation.mark(to);
                let id = Preservation::<2>::to_dom();
                let expected = if to.saved() {
                    Command::SetAttribute {
                        id,
                        attribute: Attribute::Checked,
                        value: String::new(),
                    }
                } else {
                    Command::RemoveAttribute { id, attribute: Attribute::Checked }
                };
                assert_eq!(command, expected, "{from:?} -> {to:?}");
                assert_eq!(preservation.persistence(), to, "{from:?} -> {to:?}");
                assert_eq!(preservation.draw(), expected, "{from:?} -> {to:?}");
            }
        }
    }
}
