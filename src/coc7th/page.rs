use alloc::{vec, vec::Vec};
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
    coc7th::investigator::{Investigator, Possession, Profile, join, sheet},
    host::dom::{Id, Tag},
    object::SubjectTrait as _,
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

const CHARACTERISTIC_HEADS: u32 = 5;
const SKILL_HEADS: u32 = 8;

fn heading(section: u32, tag: Tag) -> Id {
    Id::new(&[(Tag::Main, None), (Tag::Section, Some(section)), (tag, None)])
}

fn head(part: u32, table: Option<u32>, column: u32) -> Id {
    let mut path = sheet(part);
    path.extend([
        (Tag::Table, table),
        (Tag::Thead, None),
        (Tag::Tr, None),
        (Tag::Th, Some(column)),
    ]);
    Id::new(&path)
}

fn characteristic_head(column: u32) -> Id {
    head(Investigator::Characteristic.base_id(), None, column)
}

fn skill_head(column: u32) -> Id {
    head(Investigator::Skill.base_id(), None, column)
}

fn weapon_head(column: u32) -> Id {
    head(Investigator::Possession.base_id(), Some(1), column)
}

fn armor_head(column: u32) -> Id {
    head(Investigator::Possession.base_id(), Some(2), column)
}

fn profile_label(id: u32) -> Id {
    let mut path = sheet(Investigator::Profile.base_id());
    path.extend([(Tag::Dl, None), (Tag::Dt, Some(id))]);
    Id::new(&path)
}

macro_rules! label {
    ($name:ident, $en:literal, $ja:literal, $dom:expr) => {
        pub struct $name;

        impl $name {
            pub fn display(language: Language) -> &'static str {
                match language {
                    Language::En(_) => $en,
                    Language::Ja => $ja,
                }
            }

            pub fn dom() -> Vec<Id> {
                $dom
            }
        }
    };
}

label!(
    InvestigatorSheet,
    "Modern Investigator",
    "キャラクターシート",
    vec![heading(SHEET_SECTION, Tag::H2)]
);
label!(ManagementData, "Management Data", "管理データ", vec![join(sheet(1), &[(Tag::H3, None)])]);
label!(ChatLog, "Chat Log", "チャットログ", vec![heading(CHAT_LOG_SECTION, Tag::H2)]);
label!(ChatBox, "Chat Box", "チャットボックス", vec![heading(CHAT_BOX_SECTION, Tag::H2)]);
label!(Pronunciation, "Pronunciation", "読み", vec![profile_label(Profile::Name.ids()[1])]);
label!(Title, "Title", "肩書", vec![profile_label(Profile::Occupation.ids()[2])]);
label!(Initial, "Initial", "初期値", vec![characteristic_head(2)]);
label!(Change, "Change", "変動値", vec![characteristic_head(3), skill_head(6)]);
label!(Modifier, "Modifier", "修正値", vec![characteristic_head(4), skill_head(7)]);
label!(Sum, "Sum", "合計", vec![characteristic_head(5), skill_head(8)]);
label!(Specialization, "Specialization", "専門分野", vec![skill_head(2)]);
label!(BasePercent, "Base", "基本値", vec![skill_head(3)]);
label!(OccupationPoints, "Occupation", "職業PT", vec![skill_head(4)]);
label!(InterestPoints, "Interest", "興味PT", vec![skill_head(5)]);
label!(Name, "Name", "名称", vec![weapon_head(1), armor_head(1)]);
label!(Damage, "Damage", "ダメージ", vec![weapon_head(4)]);
label!(UsesPerRound, "Uses per Round", "回数", vec![weapon_head(5)]);
label!(Range, "Range", "射程", vec![weapon_head(6)]);
label!(Ammunition, "Ammo", "装弾数", vec![weapon_head(7)]);
label!(Malfunction, "Malf.", "故障", vec![weapon_head(8)]);
label!(ArmorPoints, "Armor Points", "装甲値", vec![armor_head(3)]);
label!(Notes, "Notes", "備考", vec![armor_head(4)]);

pub fn every_label(language: Language) -> Vec<(Id, &'static str)> {
    let mut labels = Vec::new();
    macro_rules! collect {
        ($($name:ident),+ $(,)?) => {
            $(labels.extend($name::dom().into_iter().map(|id| (id, $name::display(language))));)+
        };
    }
    collect!(
        InvestigatorSheet,
        ManagementData,
        ChatLog,
        ChatBox,
        Pronunciation,
        Title,
        Initial,
        Change,
        Modifier,
        Sum,
        Specialization,
        BasePercent,
        OccupationPoints,
        InterestPoints,
        Name,
        Damage,
        UsesPerRound,
        Range,
        Ammunition,
        Malfunction,
        ArmorPoints,
        Notes,
    );
    labels.push((skill_head(1), Investigator::Skill.display(language)));
    labels.push((weapon_head(2), Possession::Weapon.display(language)));
    labels.push((weapon_head(3), Investigator::Skill.display(language)));
    labels.push((armor_head(2), Possession::Armor.display(language)));
    labels
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
    fn every_label_has_a_home_in_the_html() {
        let html =
            fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/distribution/index.html"))
                .unwrap();
        let labels = every_label(Language::Ja);
        assert_eq!(
            labels.len(),
            4 + 2 + (CHARACTERISTIC_HEADS as usize - 1) + (SKILL_HEADS as usize - 1) + 6 + 3 + 4
        );
        for (id, _) in labels {
            let id = html_id(&id);
            assert!(html.contains(&format!("id=\"{id}\"")), "{id}");
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
