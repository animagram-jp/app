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
    coc7th::investigator::{Possession, Profile, Section, join, sheet},
    credential::Field,
    host::dom::{Id, Tag},
    object::{Preservation, Toast, ToastPart},
};

pub const INVESTIGATOR_OPTIONS: u32 = 4;

crate::enumerate! {
    pub HeaderButton(u8) {
        Other,
        Investigator,
        Dice,
        Appearance,
    }
}

crate::enumerate! {
    pub HeaderInput(u8) {
        Other,
        Preservation,
        Import,
    }
}

crate::enumerate! {
    pub HeaderDiv(u8) {
        Other,
        Investigator,
        Appearance,
        Language,
    }
}

crate::enumerate! {
    pub MainSection(u8) {
        Other,
        Sheet,
        ChatLog,
        ChatBox,
    }
}

crate::enumerate! {
    pub DialogLabel(u8) {
        Other,
        Email,
        Password,
        Create,
        Number,
    }
}

crate::enumerate! {
    pub PasswordInput(u8) {
        Other,
        Entry,
        Reveal,
    }
}

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
                    (Tag::Section, Some(MainSection::Sheet as u32)),
                    (Tag::H2, None),
                ])]
            }
            Self::ManagementData => vec![join(sheet(1), &[(Tag::H3, None)])],
            Self::ChatLog => {
                vec![Id::new(&[
                    (Tag::Main, None),
                    (Tag::Section, Some(MainSection::ChatLog as u32)),
                    (Tag::H2, None),
                ])]
            }
            Self::ChatBox => {
                vec![Id::new(&[
                    (Tag::Main, None),
                    (Tag::Section, Some(MainSection::ChatBox as u32)),
                    (Tag::H2, None),
                ])]
            }
            Self::Pronunciation => vec![join(
                sheet(Section::Profile.number()),
                &[(Tag::Dl, None), (Tag::Dt, Some(Profile::Name.numbers()[1]))],
            )],
            Self::Title => vec![join(
                sheet(Section::Profile.number()),
                &[(Tag::Dl, None), (Tag::Dt, Some(Profile::Occupation.numbers()[2]))],
            )],
            Self::Initial => vec![join(
                sheet(Section::Characteristic.number()),
                &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(2))],
            )],
            Self::Change => vec![
                join(
                    sheet(Section::Characteristic.number()),
                    &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(3))],
                ),
                join(
                    sheet(Section::Skill.number()),
                    &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(6))],
                ),
            ],
            Self::Modifier => vec![
                join(
                    sheet(Section::Characteristic.number()),
                    &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(4))],
                ),
                join(
                    sheet(Section::Skill.number()),
                    &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(7))],
                ),
            ],
            Self::Sum => vec![
                join(
                    sheet(Section::Characteristic.number()),
                    &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(5))],
                ),
                join(
                    sheet(Section::Skill.number()),
                    &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(8))],
                ),
            ],
            Self::Specialization => vec![join(
                sheet(Section::Skill.number()),
                &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(2))],
            )],
            Self::BasePercent => vec![join(
                sheet(Section::Skill.number()),
                &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(3))],
            )],
            Self::OccupationPoints => vec![join(
                sheet(Section::Skill.number()),
                &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(4))],
            )],
            Self::InterestPoints => vec![join(
                sheet(Section::Skill.number()),
                &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(5))],
            )],
            Self::Name => vec![
                join(
                    sheet(Section::Possession.number()),
                    &[
                        (Tag::Table, Some(1)),
                        (Tag::Thead, None),
                        (Tag::Tr, None),
                        (Tag::Th, Some(1)),
                    ],
                ),
                join(
                    sheet(Section::Possession.number()),
                    &[
                        (Tag::Table, Some(2)),
                        (Tag::Thead, None),
                        (Tag::Tr, None),
                        (Tag::Th, Some(1)),
                    ],
                ),
            ],
            Self::Damage => vec![join(
                sheet(Section::Possession.number()),
                &[(Tag::Table, Some(1)), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(4))],
            )],
            Self::UsesPerRound => vec![join(
                sheet(Section::Possession.number()),
                &[(Tag::Table, Some(1)), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(5))],
            )],
            Self::Range => vec![join(
                sheet(Section::Possession.number()),
                &[(Tag::Table, Some(1)), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(6))],
            )],
            Self::Ammunition => vec![join(
                sheet(Section::Possession.number()),
                &[(Tag::Table, Some(1)), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(7))],
            )],
            Self::Malfunction => vec![join(
                sheet(Section::Possession.number()),
                &[(Tag::Table, Some(1)), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(8))],
            )],
            Self::ArmorPoints => vec![join(
                sheet(Section::Possession.number()),
                &[(Tag::Table, Some(2)), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(3))],
            )],
            Self::Notes => vec![join(
                sheet(Section::Possession.number()),
                &[(Tag::Table, Some(2)), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(4))],
            )],
            Self::Login => {
                vec![Id::new(&[(Tag::Header, None), (Tag::Label, None), (Tag::Span, None)])]
            }
            Self::Create => vec![Id::new(&[
                (Tag::Dialog, None),
                (Tag::Label, Some(DialogLabel::Create as u32)),
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
                    sheet(Section::Skill.number()),
                    &[(Tag::Table, None), (Tag::Thead, None), (Tag::Tr, None), (Tag::Th, Some(1))],
                ),
                Section::Skill.display(language),
            ),
            (
                join(
                    sheet(Section::Possession.number()),
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
                    sheet(Section::Possession.number()),
                    &[
                        (Tag::Table, Some(1)),
                        (Tag::Thead, None),
                        (Tag::Tr, None),
                        (Tag::Th, Some(3)),
                    ],
                ),
                Section::Skill.display(language),
            ),
            (
                join(
                    sheet(Section::Possession.number()),
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
        .chain(
            Field::list()
                .iter()
                .map(|&field| (Page::CredentialName(field).to_dom(), field.display(language))),
        )
        .collect()
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Page {
    Html,
    Body,
    Investigator,
    InvestigatorOption(u32),
    Dice,
    Preservation,
    Import,
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
    CredentialName(Field),
    CredentialInput(Field),
    DialogHeading,
    DialogOptions,
    DialogNumber,
    DialogNumberInput,
    DialogSubmit,
    Create,
    CreateInput,
    Form,
    Toast(ToastPart),
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
        let dialog = |tail: &[(Tag, Option<u32>)]| {
            let mut path = Vec::from([(Tag::Dialog, None)]);
            path.extend_from_slice(tail);
            Id::new(&path)
        };
        match self {
            Self::Html => Id::new(&[(Tag::Html, None)]),
            Self::Body => Id::new(&[(Tag::Body, None)]),
            Self::Investigator => header(&[(Tag::Button, Some(HeaderButton::Investigator as u32))]),
            Self::InvestigatorOption(n) => {
                header(&[(Tag::Div, Some(HeaderDiv::Investigator as u32)), (Tag::Button, Some(n))])
            }
            Self::Dice => header(&[(Tag::Button, Some(HeaderButton::Dice as u32))]),
            Self::Preservation => Preservation::<{ HeaderInput::Preservation as u32 }>::to_dom(),
            Self::Import => header(&[(Tag::Input, Some(HeaderInput::Import as u32))]),
            Self::Appearance => header(&[(Tag::Button, Some(HeaderButton::Appearance as u32))]),
            Self::AppearanceOption(c) => header(&[
                (Tag::Div, Some(HeaderDiv::Appearance as u32)),
                (Tag::Label, Some(c.number())),
                (Tag::Input, None),
            ]),
            Self::LanguageOption(l) => header(&[
                (Tag::Div, Some(HeaderDiv::Language as u32)),
                (Tag::Button, Some(l.number())),
            ]),
            Self::Unlock => header(&[(Tag::Label, None), (Tag::Input, None)]),
            Self::Sheet => main(&[(Tag::Section, Some(MainSection::Sheet as u32))]),
            Self::ChatLog => main(&[(Tag::Section, Some(MainSection::ChatLog as u32))]),
            Self::ChatInput => {
                main(&[(Tag::Section, Some(MainSection::ChatBox as u32)), (Tag::Textarea, None)])
            }
            Self::ChatSend => {
                main(&[(Tag::Section, Some(MainSection::ChatBox as u32)), (Tag::Button, None)])
            }
            Self::Dialog => dialog(&[]),
            Self::Credential(field) => dialog(&[(Tag::Label, Some(to_label(field) as u32))]),
            Self::CredentialName(field) => {
                dialog(&[(Tag::Label, Some(to_label(field) as u32)), (Tag::Span, None)])
            }
            Self::CredentialInput(field) => dialog(&[
                (Tag::Label, Some(to_label(field) as u32)),
                (Tag::Input, to_input_number(field)),
            ]),
            Self::DialogHeading => dialog(&[(Tag::H3, None)]),
            Self::DialogOptions => dialog(&[(Tag::Div, None)]),
            Self::DialogNumber => dialog(&[(Tag::Label, Some(DialogLabel::Number as u32))]),
            Self::DialogNumberInput => {
                dialog(&[(Tag::Label, Some(DialogLabel::Number as u32)), (Tag::Input, None)])
            }
            Self::DialogSubmit => dialog(&[(Tag::Button, None)]),
            Self::Create => dialog(&[(Tag::Label, Some(DialogLabel::Create as u32))]),
            Self::CreateInput => {
                dialog(&[(Tag::Label, Some(DialogLabel::Create as u32)), (Tag::Input, None)])
            }
            Self::Form => Id::new(&[(Tag::Form, None)]),
            Self::Toast(part) => Toast::to_dom(part),
        }
    }

    pub fn from_dom(id: &Id) -> Option<Self> {
        let path: Vec<(&Tag, Option<u32>)> = id.0.iter().map(|s| (&s.tag, s.n)).collect();
        Some(match path.as_slice() {
            [(Tag::Html, None)] => Self::Html,
            [(Tag::Body, None)] => Self::Body,
            [(Tag::Header, None), (Tag::Button, Some(n))] => {
                match HeaderButton::from(u8::try_from(*n).unwrap()) {
                    HeaderButton::Investigator => Self::Investigator,
                    HeaderButton::Dice => Self::Dice,
                    HeaderButton::Appearance => Self::Appearance,
                    HeaderButton::Other => return None,
                }
            }
            [(Tag::Header, None), (Tag::Input, Some(n))] => {
                match HeaderInput::from(u8::try_from(*n).unwrap()) {
                    HeaderInput::Preservation => Self::Preservation,
                    HeaderInput::Import => Self::Import,
                    HeaderInput::Other => return None,
                }
            }
            [(Tag::Header, None), (Tag::Label, None), (Tag::Input, None)] => Self::Unlock,
            [(Tag::Header, None), (Tag::Div, Some(list)), (Tag::Button, Some(n))] => {
                match HeaderDiv::from(u8::try_from(*list).unwrap()) {
                    HeaderDiv::Investigator if (1..=INVESTIGATOR_OPTIONS).contains(n) => {
                        Self::InvestigatorOption(*n)
                    }
                    HeaderDiv::Language => {
                        Self::LanguageOption(*Language::list().iter().find(|l| l.number() == *n)?)
                    }
                    _ => return None,
                }
            }
            [
                (Tag::Header, None),
                (Tag::Div, Some(list)),
                (Tag::Label, Some(n)),
                (Tag::Input, None),
            ] if HeaderDiv::from(u8::try_from(*list).unwrap()) == HeaderDiv::Appearance => {
                Self::AppearanceOption(*ColorScheme::list().iter().find(|c| c.number() == *n)?)
            }
            [(Tag::Main, None), (Tag::Section, Some(n)), rest @ ..] => {
                match (MainSection::from(u8::try_from(*n).unwrap()), rest) {
                    (MainSection::Sheet, _) => Self::Sheet,
                    (MainSection::ChatLog, _) => Self::ChatLog,
                    (MainSection::ChatBox, [(Tag::Textarea, None)]) => Self::ChatInput,
                    (MainSection::ChatBox, [(Tag::Button, None)]) => Self::ChatSend,
                    _ => return None,
                }
            }
            [(Tag::Dialog, None), (Tag::H3, None)] => Self::DialogHeading,
            [(Tag::Dialog, None), (Tag::Div, None)] => Self::DialogOptions,
            [(Tag::Dialog, None), (Tag::Button, None)] => Self::DialogSubmit,
            [(Tag::Dialog, None), (Tag::Label, Some(n)), rest @ ..] => {
                match (DialogLabel::from(u8::try_from(*n).unwrap()), rest) {
                    (DialogLabel::Create, []) => Self::Create,
                    (DialogLabel::Create, [(Tag::Input, None)]) => Self::CreateInput,
                    (DialogLabel::Number, []) => Self::DialogNumber,
                    (DialogLabel::Number, [(Tag::Input, None)]) => Self::DialogNumberInput,
                    (label, rest) => {
                        let Some(field) = to_field(label) else {
                            return Some(Self::Dialog);
                        };
                        match rest {
                            [] => Self::Credential(field),
                            [(Tag::Span, None)] => Self::CredentialName(field),
                            [(Tag::Input, input)] if *input == to_input_number(field) => {
                                Self::CredentialInput(field)
                            }
                            _ => Self::Dialog,
                        }
                    }
                }
            }
            [(Tag::Dialog, None), ..] => Self::Dialog,
            [(Tag::Form, None)] => Self::Form,
            [(Tag::Toast, None), ..] => Self::Toast(Toast::from_dom(id)?),
            _ => return None,
        })
    }

    pub fn every() -> Vec<Self> {
        let mut pages = Vec::from([
            Self::Html,
            Self::Body,
            Self::Investigator,
            Self::Dice,
            Self::Preservation,
            Self::Import,
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
            Self::DialogSubmit,
            Self::Create,
            Self::CreateInput,
            Self::Form,
        ]);
        pages.extend(Field::list().iter().flat_map(|&field| {
            [Self::Credential(field), Self::CredentialName(field), Self::CredentialInput(field)]
        }));
        pages.extend((1..=INVESTIGATOR_OPTIONS).map(Self::InvestigatorOption));
        pages.extend(Toast::parts().map(Self::Toast));
        pages.extend(Language::list().iter().map(|&l| Self::LanguageOption(l)));
        pages.extend(ColorScheme::list().iter().map(|&c| Self::AppearanceOption(c)));
        pages
    }
}

fn to_input_number(field: Field) -> Option<u32> {
    (field == Field::Password).then_some(PasswordInput::Entry as u32)
}

fn to_label(field: Field) -> DialogLabel {
    match field {
        Field::Email => DialogLabel::Email,
        Field::Password => DialogLabel::Password,
        Field::Other => DialogLabel::Other,
    }
}

fn to_field(label: DialogLabel) -> Option<Field> {
    match label {
        DialogLabel::Email => Some(Field::Email),
        DialogLabel::Password => Some(Field::Password),
        _ => None,
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

    fn opening_tag<'a>(html: &'a str, id: &Id) -> &'a str {
        let at = html.find(&format!("id=\"{}\"", html_id(id))).unwrap();
        let start = html[..at].rfind('<').unwrap();
        let end = at + html[at..].find('>').unwrap();
        &html[start..end]
    }

    #[test]
    fn every_positioned_page_carries_its_html_attribute() {
        let html = fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/distribution/coc7th/index.html"
        ))
        .unwrap();
        for (page, attribute) in [
            (Page::Investigator, "aria-haspopup=\"listbox\""),
            (Page::Dice, "aria-label=\"Dice\""),
            (Page::Appearance, "aria-label=\"Appearance\""),
            (Page::Preservation, "data-facet=\"sync\""),
            (Page::Import, "type=\"file\""),
            (Page::Sheet, "aria-label=\"Character Sheet\""),
            (Page::ChatLog, "aria-label=\"Chat Log\""),
            (Page::CredentialInput(Field::Email), "name=\"email\""),
            (Page::CredentialInput(Field::Password), "name=\"password\""),
            (Page::CreateInput, "name=\"create\""),
            (Page::DialogNumberInput, "inputmode=\"numeric\""),
        ] {
            let tag = opening_tag(&html, &page.to_dom());
            assert!(tag.contains(attribute), "{page:?}: {tag}");
        }
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
        assert_eq!(labels.len(), 34);
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
