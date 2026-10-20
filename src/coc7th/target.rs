use alloc::vec::Vec;
use core::{
    clone::Clone,
    cmp::PartialEq,
    fmt::Debug,
    marker::Copy,
    option::Option::{self, None, Some},
    primitive::u32,
};

use crate::host::dom::{Id, Tag};

const LANGUAGE_BUTTON: u32 = 1;
const CHARACTER_BUTTON: u32 = 2;
const EDIT_BUTTON: u32 = 3;
const DICE_BUTTON: u32 = 4;
const SAVE_BUTTON: u32 = 5;
const APPEARANCE_BUTTON: u32 = 6;
const SHEET_SECTION: u32 = 1;
pub const SHEET_PARTS: u32 = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sheet {
    Immutable,
    Editable,
}

impl Sheet {
    fn tag(self) -> Tag {
        match self {
            Self::Immutable => Tag::Section,
            Self::Editable => Tag::Fieldset,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Body,
    Language,
    Character,
    Edit,
    Dice,
    Save,
    Appearance,
    Sheet(Sheet, u32),
}

impl Target {
    pub fn to_dom(self) -> Id {
        let button = |n| Vec::from([(Tag::Header, None), (Tag::Button, Some(n))]);
        let path = match self {
            Self::Body => Vec::from([(Tag::Body, None)]),
            Self::Language => button(LANGUAGE_BUTTON),
            Self::Character => button(CHARACTER_BUTTON),
            Self::Edit => button(EDIT_BUTTON),
            Self::Dice => button(DICE_BUTTON),
            Self::Save => button(SAVE_BUTTON),
            Self::Appearance => button(APPEARANCE_BUTTON),
            Self::Sheet(sheet, part) => Vec::from([
                (Tag::Main, None),
                (Tag::Section, Some(SHEET_SECTION)),
                (sheet.tag(), Some(part)),
            ]),
        };
        Id::new(&path)
    }

    pub fn from_dom(id: &Id) -> Option<Self> {
        let path: Vec<(&Tag, Option<u32>)> = id.0.iter().map(|s| (&s.tag, s.n)).collect();
        Some(match path.as_slice() {
            [(Tag::Body, None)] => Self::Body,
            [(Tag::Header, None), (Tag::Button, Some(n))] => match *n {
                LANGUAGE_BUTTON => Self::Language,
                CHARACTER_BUTTON => Self::Character,
                EDIT_BUTTON => Self::Edit,
                DICE_BUTTON => Self::Dice,
                SAVE_BUTTON => Self::Save,
                APPEARANCE_BUTTON => Self::Appearance,
                _ => return None,
            },
            [
                (Tag::Main, None),
                (Tag::Section, Some(SHEET_SECTION)),
                (Tag::Section, Some(n)),
                ..,
            ] if (1..=SHEET_PARTS).contains(n) => Self::Sheet(Sheet::Immutable, *n),
            [
                (Tag::Main, None),
                (Tag::Section, Some(SHEET_SECTION)),
                (Tag::Fieldset, Some(n)),
                ..,
            ] if (1..=SHEET_PARTS).contains(n) => Self::Sheet(Sheet::Editable, *n),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use alloc::{format, string::String, vec::Vec};
    use std::fs;

    use super::*;

    fn every_target() -> Vec<Target> {
        let mut targets = Vec::from([
            Target::Body,
            Target::Language,
            Target::Character,
            Target::Edit,
            Target::Dice,
            Target::Save,
            Target::Appearance,
        ]);
        for part in 1..=SHEET_PARTS {
            targets.push(Target::Sheet(Sheet::Immutable, part));
            targets.push(Target::Sheet(Sheet::Editable, part));
        }
        targets
    }

    fn html_id(id: &Id) -> String {
        let mut parts = Vec::new();
        for segment in &id.0 {
            let name = format!("{:?}", segment.tag).to_lowercase();
            parts.push(match segment.n {
                Some(n) => format!("{name}-{n}"),
                None => name,
            });
        }
        parts.join("_")
    }

    #[test]
    fn every_target_survives_the_dom_round_trip() {
        for target in every_target() {
            assert_eq!(Target::from_dom(&target.to_dom()), Some(target), "{target:?}");
        }
    }

    #[test]
    fn distinct_targets_have_distinct_dom_ids() {
        let targets = every_target();
        for (i, a) in targets.iter().enumerate() {
            for b in &targets[i + 1..] {
                assert_ne!(a.to_dom(), b.to_dom(), "{a:?} / {b:?}");
            }
        }
    }

    #[test]
    fn every_target_exists_in_the_html() {
        let html =
            fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/distribution/index.html"))
                .unwrap();
        for target in every_target() {
            let id = html_id(&target.to_dom());
            assert!(html.contains(&format!("id=\"{id}\"")), "{target:?} -> {id}");
        }
    }

    #[test]
    fn descendants_of_a_sheet_resolve_to_the_sheet() {
        let inside = Id::new(&[
            (Tag::Main, None),
            (Tag::Section, Some(1)),
            (Tag::Fieldset, Some(2)),
            (Tag::Label, Some(3)),
            (Tag::Input, None),
        ]);
        assert_eq!(Target::from_dom(&inside), Some(Target::Sheet(Sheet::Editable, 2)));
    }

    #[test]
    fn unknown_ids_resolve_to_nothing() {
        let unknown = [
            Id::new(&[(Tag::Footer, None)]),
            Id::new(&[(Tag::Header, None), (Tag::Button, Some(7))]),
            Id::new(&[(Tag::Header, None), (Tag::Div, Some(1))]),
            Id::new(&[(Tag::Main, None), (Tag::Section, Some(3))]),
            Id::new(&[(Tag::Main, None), (Tag::Section, Some(2)), (Tag::Fieldset, Some(2))]),
            Id::new(&[(Tag::Main, None), (Tag::Section, Some(1)), (Tag::H3, Some(1))]),
            Id::new(&[(Tag::Main, None), (Tag::Section, Some(1)), (Tag::Fieldset, Some(9))]),
            Id::new(&[(Tag::Main, None), (Tag::H2, Some(1))]),
        ];
        for id in unknown {
            assert_eq!(Target::from_dom(&id), None);
        }
    }
}
