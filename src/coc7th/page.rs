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
    ColorScheme, Language,
    coc7th::investigator::{Investigator, Possession, Profile, join, sheet},
    credential::Field,
    host::dom::{Id, Tag},
    object::SubjectTrait as _,
};

const INVESTIGATOR_BUTTON: u32 = 1;
const DICE_BUTTON: u32 = 2;
const APPEARANCE_BUTTON: u32 = 3;
const INVESTIGATOR_LIST: u32 = 1;
const APPEARANCE_LIST: u32 = 2;
const LANGUAGE_LIST: u32 = 3;
const SHEET_SECTION: u32 = 1;
const CHAT_LOG_SECTION: u32 = 2;
const CHAT_BOX_SECTION: u32 = 3;
const CREATE_LABEL: u32 = 1;
const NUMBER_LABEL: u32 = 2;
pub const INVESTIGATOR_OPTIONS: u32 = 4;
pub const TOAST_SLOTS: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Label {
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
    Login,
    Create,
}

impl Label {
    pub fn display(&self, language: Language) -> &'static str {
        match (self, language) {
            (Self::InvestigatorSheet, Language::En(_)) => "Modern Investigator",
            (Self::InvestigatorSheet, Language::Ja) => "キャラクターシート",
            (Self::ManagementData, Language::En(_)) => "Management Data",
            (Self::ManagementData, Language::Ja) => "管理データ",
            (Self::ChatLog, Language::En(_)) => "Chat Log",
            (Self::ChatLog, Language::Ja) => "チャットログ",
            (Self::ChatBox, Language::En(_)) => "Chat Box",
            (Self::ChatBox, Language::Ja) => "チャットボックス",
            (Self::Pronunciation, Language::En(_)) => "Pronunciation",
            (Self::Pronunciation, Language::Ja) => "読み",
            (Self::Title, Language::En(_)) => "Title",
            (Self::Title, Language::Ja) => "肩書",
            (Self::Initial, Language::En(_)) => "Initial",
            (Self::Initial, Language::Ja) => "初期値",
            (Self::Change, Language::En(_)) => "Change",
            (Self::Change, Language::Ja) => "変動値",
            (Self::Modifier, Language::En(_)) => "Modifier",
            (Self::Modifier, Language::Ja) => "修正値",
            (Self::Sum, Language::En(_)) => "Sum",
            (Self::Sum, Language::Ja) => "合計",
            (Self::Specialization, Language::En(_)) => "Specialization",
            (Self::Specialization, Language::Ja) => "専門分野",
            (Self::BasePercent, Language::En(_)) => "Base",
            (Self::BasePercent, Language::Ja) => "基本値",
            (Self::OccupationPoints, Language::En(_)) => "Occupation",
            (Self::OccupationPoints, Language::Ja) => "職業PT",
            (Self::InterestPoints, Language::En(_)) => "Interest",
            (Self::InterestPoints, Language::Ja) => "興味PT",
            (Self::Name, Language::En(_)) => "Name",
            (Self::Name, Language::Ja) => "名称",
            (Self::Damage, Language::En(_)) => "Damage",
            (Self::Damage, Language::Ja) => "ダメージ",
            (Self::UsesPerRound, Language::En(_)) => "Uses per Round",
            (Self::UsesPerRound, Language::Ja) => "回数",
            (Self::Range, Language::En(_)) => "Range",
            (Self::Range, Language::Ja) => "射程",
            (Self::Ammunition, Language::En(_)) => "Ammo",
            (Self::Ammunition, Language::Ja) => "装弾数",
            (Self::Malfunction, Language::En(_)) => "Malf.",
            (Self::Malfunction, Language::Ja) => "故障",
            (Self::ArmorPoints, Language::En(_)) => "Armor Points",
            (Self::ArmorPoints, Language::Ja) => "装甲値",
            (Self::Notes, Language::En(_)) => "Notes",
            (Self::Notes, Language::Ja) => "備考",
            (Self::Login, Language::En(_)) => "Login",
            (Self::Login, Language::Ja) => "ログイン",
            (Self::Create, Language::En(_)) => "Create",
            (Self::Create, Language::Ja) => "作成",
        }
    }

    pub fn dom(&self) -> Vec<Id> {
        match self {
            Self::InvestigatorSheet => {
                vec![Id::new(&[
                    (Tag::Main, None),
                    (Tag::Section, Some(SHEET_SECTION)),
                    (Tag::H2, None),
                ])]
            }
            Self::ManagementData => vec![join(sheet(1), &[(Tag::H3, None)])],
            Self::ChatLog => {
                vec![Id::new(&[
                    (Tag::Main, None),
                    (Tag::Section, Some(CHAT_LOG_SECTION)),
                    (Tag::H2, None),
                ])]
            }
            Self::ChatBox => {
                vec![Id::new(&[
                    (Tag::Main, None),
                    (Tag::Section, Some(CHAT_BOX_SECTION)),
                    (Tag::H2, None),
                ])]
            }
            Self::Pronunciation => vec![join(
                sheet(Investigator::Profile.base_id()),
                &[(Tag::Dl, None), (Tag::Dt, Some(Profile::Name.ids()[1]))],
            )],
            Self::Title => vec![join(
                sheet(Investigator::Profile.base_id()),
                &[(Tag::Dl, None), (Tag::Dt, Some(Profile::Occupation.ids()[2]))],
            )],
            Self::Initial => vec![join(
                sheet(Investigator::Characteristic.base_id()),
                &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(2))],
            )],
            Self::Change => vec![
                join(
                    sheet(Investigator::Characteristic.base_id()),
                    &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(3))],
                ),
                join(
                    sheet(Investigator::Skill.base_id()),
                    &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(6))],
                ),
            ],
            Self::Modifier => vec![
                join(
                    sheet(Investigator::Characteristic.base_id()),
                    &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(4))],
                ),
                join(
                    sheet(Investigator::Skill.base_id()),
                    &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(7))],
                ),
            ],
            Self::Sum => vec![
                join(
                    sheet(Investigator::Characteristic.base_id()),
                    &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(5))],
                ),
                join(
                    sheet(Investigator::Skill.base_id()),
                    &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(8))],
                ),
            ],
            Self::Specialization => vec![join(
                sheet(Investigator::Skill.base_id()),
                &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(2))],
            )],
            Self::BasePercent => vec![join(
                sheet(Investigator::Skill.base_id()),
                &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(3))],
            )],
            Self::OccupationPoints => vec![join(
                sheet(Investigator::Skill.base_id()),
                &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(4))],
            )],
            Self::InterestPoints => vec![join(
                sheet(Investigator::Skill.base_id()),
                &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(5))],
            )],
            Self::Name => vec![
                join(
                    sheet(Investigator::Possession.base_id()),
                    &[
                        (Tag::Table, Some(1)),
                        (Tag::Thead, None),
                        (Tag::Tr, None),
                        (Tag::Th, Some(1)),
                    ],
                ),
                join(
                    sheet(Investigator::Possession.base_id()),
                    &[
                        (Tag::Table, Some(2)),
                        (Tag::Thead, None),
                        (Tag::Tr, None),
                        (Tag::Th, Some(1)),
                    ],
                ),
            ],
            Self::Damage => vec![join(
                sheet(Investigator::Possession.base_id()),
                &[(Tag::Table, Some(1)), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(4))],
            )],
            Self::UsesPerRound => vec![join(
                sheet(Investigator::Possession.base_id()),
                &[(Tag::Table, Some(1)), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(5))],
            )],
            Self::Range => vec![join(
                sheet(Investigator::Possession.base_id()),
                &[(Tag::Table, Some(1)), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(6))],
            )],
            Self::Ammunition => vec![join(
                sheet(Investigator::Possession.base_id()),
                &[(Tag::Table, Some(1)), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(7))],
            )],
            Self::Malfunction => vec![join(
                sheet(Investigator::Possession.base_id()),
                &[(Tag::Table, Some(1)), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(8))],
            )],
            Self::ArmorPoints => vec![join(
                sheet(Investigator::Possession.base_id()),
                &[(Tag::Table, Some(2)), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(3))],
            )],
            Self::Notes => vec![join(
                sheet(Investigator::Possession.base_id()),
                &[(Tag::Table, Some(2)), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(4))],
            )],
            Self::Login => {
                vec![Id::new(&[(Tag::Header, None), (Tag::Label, None), (Tag::Span, None)])]
            }
            Self::Create => vec![Id::new(&[
                (Tag::Main, None),
                (Tag::Dialog, None),
                (Tag::Label, Some(CREATE_LABEL)),
                (Tag::Span, None),
            ])],
        }
    }

    pub fn list() -> &'static [Label] {
        &[
            Self::InvestigatorSheet,
            Self::ManagementData,
            Self::ChatLog,
            Self::ChatBox,
            Self::Pronunciation,
            Self::Title,
            Self::Initial,
            Self::Change,
            Self::Modifier,
            Self::Sum,
            Self::Specialization,
            Self::BasePercent,
            Self::OccupationPoints,
            Self::InterestPoints,
            Self::Name,
            Self::Damage,
            Self::UsesPerRound,
            Self::Range,
            Self::Ammunition,
            Self::Malfunction,
            Self::ArmorPoints,
            Self::Notes,
            Self::Login,
            Self::Create,
        ]
    }
}

pub fn every_label(language: Language) -> Vec<(Id, &'static str)> {
    Label::list()
        .iter()
        .flat_map(|label| label.dom().into_iter().map(|id| (id, label.display(language))))
        .chain([
            (
                join(
                    sheet(Investigator::Skill.base_id()),
                    &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(1))],
                ),
                Investigator::Skill.display(language),
            ),
            (
                join(
                    sheet(Investigator::Possession.base_id()),
                    &[
                        (Tag::Table, Some(1)),
                        (Tag::Thead, None),
                        (Tag::Tr, None),
                        (Tag::Th, Some(2)),
                    ],
                ),
                Possession::Weapon.display(language),
            ),
            (
                join(
                    sheet(Investigator::Possession.base_id()),
                    &[
                        (Tag::Table, Some(1)),
                        (Tag::Thead, None),
                        (Tag::Tr, None),
                        (Tag::Th, Some(3)),
                    ],
                ),
                Investigator::Skill.display(language),
            ),
            (
                join(
                    sheet(Investigator::Possession.base_id()),
                    &[
                        (Tag::Table, Some(2)),
                        (Tag::Thead, None),
                        (Tag::Tr, None),
                        (Tag::Th, Some(2)),
                    ],
                ),
                Possession::Armor.display(language),
            ),
        ])
        .collect()
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
    AppearanceOption(ColorScheme),
    LanguageOption(Language),
    Unlock,
    Sheet,
    ChatLog,
    ChatInput,
    ChatSend,
    Dialog,
    Credential(Field),
    DialogHeading,
    DialogOptions,
    DialogNumber,
    DialogNumberInput,
    DialogForm,
    Create,
    CreateInput,
    Toast(u32),
    ToastText(u32),
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
            Self::AppearanceOption(c) => header(&[
                (Tag::Div, Some(APPEARANCE_LIST)),
                (Tag::Label, Some(c.number())),
                (Tag::Input, None),
            ]),
            Self::LanguageOption(l) => {
                header(&[(Tag::Div, Some(LANGUAGE_LIST)), (Tag::Button, Some(l.number()))])
            }
            Self::Unlock => header(&[(Tag::Label, None), (Tag::Input, None)]),
            Self::Sheet => main(&[(Tag::Section, Some(SHEET_SECTION))]),
            Self::ChatLog => main(&[(Tag::Section, Some(CHAT_LOG_SECTION))]),
            Self::ChatInput => {
                main(&[(Tag::Section, Some(CHAT_BOX_SECTION)), (Tag::Textarea, None)])
            }
            Self::ChatSend => main(&[(Tag::Section, Some(CHAT_BOX_SECTION)), (Tag::Button, None)]),
            Self::Dialog => main(&[(Tag::Dialog, None)]),
            Self::Credential(field) => {
                main(&[(Tag::Dialog, None), (Tag::Input, Some(field.number()))])
            }
            Self::DialogHeading => main(&[(Tag::Dialog, None), (Tag::H3, None)]),
            Self::DialogOptions => main(&[(Tag::Dialog, None), (Tag::Div, None)]),
            Self::DialogNumber => main(&[(Tag::Dialog, None), (Tag::Label, Some(NUMBER_LABEL))]),
            Self::DialogNumberInput => {
                main(&[(Tag::Dialog, None), (Tag::Label, Some(NUMBER_LABEL)), (Tag::Input, None)])
            }
            Self::Create => main(&[(Tag::Dialog, None), (Tag::Label, Some(CREATE_LABEL))]),
            Self::CreateInput => {
                main(&[(Tag::Dialog, None), (Tag::Label, Some(CREATE_LABEL)), (Tag::Input, None)])
            }
            Self::DialogForm => main(&[(Tag::Form, None)]),
            Self::Toast(n) => Id::new(&[(Tag::Toast, None), (Tag::Button, Some(n))]),
            Self::ToastText(n) => {
                Id::new(&[(Tag::Toast, None), (Tag::Button, Some(n)), (Tag::P, None)])
            }
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
            [(Tag::Header, None), (Tag::Label, None), (Tag::Input, None)] => Self::Unlock,
            [(Tag::Header, None), (Tag::Div, Some(INVESTIGATOR_LIST)), (Tag::Button, Some(n))]
                if (1..=INVESTIGATOR_OPTIONS).contains(n) =>
            {
                Self::InvestigatorOption(*n)
            }
            [(Tag::Header, None), (Tag::Div, Some(LANGUAGE_LIST)), (Tag::Button, Some(n))] => {
                Self::LanguageOption(*Language::list().iter().find(|l| l.number() == *n)?)
            }
            [
                (Tag::Header, None),
                (Tag::Div, Some(APPEARANCE_LIST)),
                (Tag::Label, Some(n)),
                (Tag::Input, None),
            ] => Self::AppearanceOption(*ColorScheme::list().iter().find(|c| c.number() == *n)?),
            [(Tag::Main, None), (Tag::Section, Some(SHEET_SECTION)), ..] => Self::Sheet,
            [(Tag::Main, None), (Tag::Section, Some(CHAT_LOG_SECTION)), ..] => Self::ChatLog,
            [(Tag::Main, None), (Tag::Section, Some(CHAT_BOX_SECTION)), (Tag::Textarea, None)] => {
                Self::ChatInput
            }
            [(Tag::Main, None), (Tag::Section, Some(CHAT_BOX_SECTION)), (Tag::Button, None)] => {
                Self::ChatSend
            }
            [(Tag::Main, None), (Tag::Dialog, None), (Tag::Input, Some(n))] => {
                Self::Credential(*Field::list().get(n.checked_sub(1)? as usize)?)
            }
            [(Tag::Main, None), (Tag::Dialog, None), (Tag::H3, None)] => Self::DialogHeading,
            [(Tag::Main, None), (Tag::Dialog, None), (Tag::Div, None)] => Self::DialogOptions,
            [(Tag::Main, None), (Tag::Dialog, None), (Tag::Label, Some(n))] => match *n {
                CREATE_LABEL => Self::Create,
                NUMBER_LABEL => Self::DialogNumber,
                _ => Self::Dialog,
            },
            [(Tag::Main, None), (Tag::Dialog, None), (Tag::Label, Some(n)), (Tag::Input, None)] => {
                match *n {
                    CREATE_LABEL => Self::CreateInput,
                    NUMBER_LABEL => Self::DialogNumberInput,
                    _ => Self::Dialog,
                }
            }
            [(Tag::Main, None), (Tag::Dialog, None), ..] => Self::Dialog,
            [(Tag::Main, None), (Tag::Form, None)] => Self::DialogForm,
            [(Tag::Toast, None), (Tag::Button, Some(n))] if (1..=TOAST_SLOTS).contains(n) => {
                Self::Toast(*n)
            }
            [(Tag::Toast, None), (Tag::Button, Some(n)), (Tag::P, None)]
                if (1..=TOAST_SLOTS).contains(n) =>
            {
                Self::ToastText(*n)
            }
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
            Self::Unlock,
            Self::Sheet,
            Self::ChatLog,
            Self::ChatInput,
            Self::ChatSend,
            Self::Dialog,
            Self::DialogHeading,
            Self::DialogOptions,
            Self::DialogNumber,
            Self::DialogNumberInput,
            Self::DialogForm,
            Self::Create,
            Self::CreateInput,
        ]);
        pages.extend(Field::list().iter().copied().map(Self::Credential));
        pages.extend((1..=INVESTIGATOR_OPTIONS).map(Self::InvestigatorOption));
        pages.extend((1..=TOAST_SLOTS).flat_map(|n| [Self::Toast(n), Self::ToastText(n)]));
        pages.extend(Language::list().iter().map(|&l| Self::LanguageOption(l)));
        pages.extend(ColorScheme::list().iter().map(|&c| Self::AppearanceOption(c)));
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
        let html = fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/distribution/coc7th/index.html"
        ))
        .unwrap();
        for page in Page::every() {
            let id = html_id(&page.to_dom());
            assert!(html.contains(&format!("id=\"{id}\"")), "{page:?} -> {id}");
        }
    }

    #[test]
    fn every_label_has_a_home_in_the_html() {
        let html = fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/distribution/coc7th/index.html"
        ))
        .unwrap();
        let labels = every_label(Language::Ja);
        assert_eq!(labels.len(), 32);
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
