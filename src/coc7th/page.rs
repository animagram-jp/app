use alloc::vec::Vec;
use core::{
    clone::Clone,
    cmp::PartialEq,
    fmt::Debug,
    marker::Copy,
    option::Option::{self, None, Some},
    primitive::u32,
};

use crate::{
    Language,
    host::dom::{Id, Tag},
};

const INVESTIGATOR_BUTTON: u32 = 1;
const DICE_BUTTON: u32 = 2;
const APPEARANCE_BUTTON: u32 = 3;
const INVESTIGATOR_LIST: u32 = 1;
const LANGUAGE_LIST: u32 = 3;
const SHEET_SECTION: u32 = 1;
const CHAT_LOG_SECTION: u32 = 2;
const CHAT_BOX_SECTION: u32 = 3;
pub const INVESTIGATOR_OPTIONS: u32 = 4;

pub struct InvestigatorSheet;

impl InvestigatorSheet {
    pub fn display() -> &'static str {
        "Modern Investigator"
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Page {
    Html,
    Body,
    Investigator,
    InvestigatorOption(u32),
    Dice,
    Save,
    Appearance,
    LanguageOption(Language),
    Sheet,
    ChatLog,
    ChatInput,
    ChatSend,
    Dialog,
}

impl Page {
    pub fn to_dom(self) -> Id {
        let header = |tail: &[(Tag, Option<u32>)]| {
            let mut path = Vec::from([(Tag::Header, None)]);
            path.extend_from_slice(tail);
            Id::new(&path)
        };
        let main = |tail: &[(Tag, Option<u32>)]| {
            let mut path = Vec::from([(Tag::Main, None)]);
            path.extend_from_slice(tail);
            Id::new(&path)
        };
        match self {
            Self::Html => Id::new(&[(Tag::Html, None)]),
            Self::Body => Id::new(&[(Tag::Body, None)]),
            Self::Investigator => header(&[(Tag::Button, Some(INVESTIGATOR_BUTTON))]),
            Self::InvestigatorOption(n) => {
                header(&[(Tag::Div, Some(INVESTIGATOR_LIST)), (Tag::Button, Some(n))])
            }
            Self::Dice => header(&[(Tag::Button, Some(DICE_BUTTON))]),
            Self::Save => header(&[(Tag::Input, None)]),
            Self::Appearance => header(&[(Tag::Button, Some(APPEARANCE_BUTTON))]),
            Self::LanguageOption(l) => {
                header(&[(Tag::Div, Some(LANGUAGE_LIST)), (Tag::Button, Some(l.number()))])
            }
            Self::Sheet => main(&[(Tag::Section, Some(SHEET_SECTION))]),
            Self::ChatLog => main(&[(Tag::Section, Some(CHAT_LOG_SECTION))]),
            Self::ChatInput => {
                main(&[(Tag::Section, Some(CHAT_BOX_SECTION)), (Tag::Textarea, None)])
            }
            Self::ChatSend => main(&[(Tag::Section, Some(CHAT_BOX_SECTION)), (Tag::Button, None)]),
            Self::Dialog => main(&[(Tag::Dialog, None)]),
        }
    }

    pub fn from_dom(id: &Id) -> Option<Self> {
        let path: Vec<(&Tag, Option<u32>)> = id.0.iter().map(|s| (&s.tag, s.n)).collect();
        Some(match path.as_slice() {
            [(Tag::Html, None)] => Self::Html,
            [(Tag::Body, None)] => Self::Body,
            [(Tag::Header, None), (Tag::Button, Some(n))] => match *n {
                INVESTIGATOR_BUTTON => Self::Investigator,
                DICE_BUTTON => Self::Dice,
                APPEARANCE_BUTTON => Self::Appearance,
                _ => return None,
            },
            [(Tag::Header, None), (Tag::Input, None)] => Self::Save,
            [(Tag::Header, None), (Tag::Div, Some(INVESTIGATOR_LIST)), (Tag::Button, Some(n))]
                if (1..=INVESTIGATOR_OPTIONS).contains(n) =>
            {
                Self::InvestigatorOption(*n)
            }
            [(Tag::Header, None), (Tag::Div, Some(LANGUAGE_LIST)), (Tag::Button, Some(n))] => {
                Self::LanguageOption(*Language::list().iter().find(|l| l.number() == *n)?)
            }
            [(Tag::Main, None), (Tag::Section, Some(SHEET_SECTION)), ..] => Self::Sheet,
            [(Tag::Main, None), (Tag::Section, Some(CHAT_LOG_SECTION)), ..] => Self::ChatLog,
            [(Tag::Main, None), (Tag::Section, Some(CHAT_BOX_SECTION)), (Tag::Textarea, None)] => {
                Self::ChatInput
            }
            [(Tag::Main, None), (Tag::Section, Some(CHAT_BOX_SECTION)), (Tag::Button, None)] => {
                Self::ChatSend
            }
            [(Tag::Main, None), (Tag::Dialog, None), ..] => Self::Dialog,
            _ => return None,
        })
    }

    pub fn every() -> Vec<Self> {
        let mut pages = Vec::from([
            Self::Html,
            Self::Body,
            Self::Investigator,
            Self::Dice,
            Self::Save,
            Self::Appearance,
            Self::Sheet,
            Self::ChatLog,
            Self::ChatInput,
            Self::ChatSend,
            Self::Dialog,
        ]);
        pages.extend((1..=INVESTIGATOR_OPTIONS).map(Self::InvestigatorOption));
        pages.extend(Language::list().iter().map(|&l| Self::LanguageOption(l)));
        pages
    }
}

#[cfg(test)]
mod tests {
    use alloc::{format, string::String, vec::Vec};
    use std::fs;

    use super::*;

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
    fn every_page_survives_the_dom_round_trip() {
        for page in Page::every() {
            assert_eq!(Page::from_dom(&page.to_dom()), Some(page), "{page:?}");
        }
    }

    #[test]
    fn distinct_pages_have_distinct_dom_ids() {
        let pages = Page::every();
        for (i, a) in pages.iter().enumerate() {
            for b in &pages[i + 1..] {
                assert_ne!(a.to_dom(), b.to_dom(), "{a:?} / {b:?}");
            }
        }
    }

    #[test]
    fn every_page_exists_in_the_html() {
        let html =
            fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/distribution/index.html"))
                .unwrap();
        for page in Page::every() {
            let id = html_id(&page.to_dom());
            assert!(html.contains(&format!("id=\"{id}\"")), "{page:?} -> {id}");
        }
    }

    #[test]
    fn descendants_of_the_sheet_resolve_to_the_sheet() {
        let inside = Id::new(&[
            (Tag::Main, None),
            (Tag::Section, Some(1)),
            (Tag::Section, Some(10)),
            (Tag::Dl, None),
            (Tag::Dd, Some(10)),
            (Tag::Input, None),
        ]);
        assert_eq!(Page::from_dom(&inside), Some(Page::Sheet));
    }

    #[test]
    fn unknown_ids_resolve_to_nothing() {
        let unknown = [
            Id::new(&[(Tag::Footer, None)]),
            Id::new(&[(Tag::Header, None), (Tag::Button, Some(5))]),
            Id::new(&[(Tag::Header, None), (Tag::Div, Some(1)), (Tag::Button, Some(5))]),
            Id::new(&[(Tag::Header, None), (Tag::Div, Some(3)), (Tag::Button, Some(3))]),
            Id::new(&[(Tag::Main, None), (Tag::Section, Some(4))]),
            Id::new(&[(Tag::Main, None), (Tag::H2, None)]),
        ];
        for id in unknown {
            assert_eq!(Page::from_dom(&id), None);
        }
    }
}
