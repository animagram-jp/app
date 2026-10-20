use alloc::vec::Vec;
use core::{array::from_fn, primitive::u8};

use crate::{Lang, data_struct::DataStruct, list::ListError};

// === SubjectTrait ===

pub trait SubjectTrait {
    const SCHEMA_SIZE: u32;
    fn ids(&self) -> &'static [u32];
    fn display(&self, lang: Lang) -> &'static str;
    fn list() -> &'static [Self]
    where
        Self: Sized;
}

// === Predicate ===

pub trait Predicate<const N: usize> {
    type Parsed;
    type Subject: SubjectTrait;
    const VARIANT: Self::Subject;

    fn ids() -> [u32; N] {
        Self::VARIANT.ids().try_into().expect("id slice length mismatch")
    }

    fn parse(bytes: [Option<&[u8]>; N]) -> Self::Parsed;

    fn encode(value: &Self::Parsed) -> [Option<Vec<u8>>; N];

    fn read(character: &DataStruct) -> Self::Parsed {
        Self::parse(character.get_many(Self::ids()))
    }

    fn get_from_bytes(instance: &[u8]) -> Self::Parsed {
        let size = Self::Subject::SCHEMA_SIZE;
        Self::parse(Self::ids().map(|id| DataStruct::get_from_bytes(instance, size, id).ok()))
    }

    fn write(
        character: &mut DataStruct,
        value: &Self::Parsed,
        time: Option<f64>,
    ) -> Result<(), ListError> {
        let ids = Self::ids();
        let encoded = Self::encode(value);
        let entries: [(u32, Option<&[u8]>); N] = from_fn(|i| (ids[i], encoded[i].as_deref()));
        character.set_many(entries, time)
    }
}
