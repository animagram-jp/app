use alloc::vec::Vec;
use core::{
    clone::Clone,
    cmp::PartialEq,
    fmt::Debug,
    marker::Copy,
    option::Option::{self, None, Some},
    primitive::{u32, usize},
};

use super::handler::View;
use crate::js_client::dom::{Id, Tag};

const VIEWS: [View; 3] = [View::Day, View::ThreeDays, View::Week];
const FIELDS: [EditField; 8] = [
    EditField::Title,
    EditField::Category,
    EditField::Status,
    EditField::Date,
    EditField::Resource,
    EditField::Start,
    EditField::End,
    EditField::Note,
];
const RELOAD_BUTTON: u32 = 1;
const MONTH_BUTTON: u32 = 2;
const SAVE_BUTTON: u32 = 3;
const MONTH_COLUMNS: u32 = 7;
const MONTH_ROWS: u32 = 6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EditField {
    Title,
    Category,
    Status,
    Date,
    Resource,
    Start,
    End,
    Note,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CardPart {
    Status,
    Time,
    Title,
    Category,
    Note,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Title,
    ViewButton(View),
    Step(u32),
    Zoom,
    Reload,
    MonthOpen,
    Save,
    TimeAxis(u32),
    Surface,
    DayList,
    Day(u32),
    DayTitle(u32),
    Resource(u32),
    ResourceName(u32),
    ResourcePerson(u32),
    Band(u32),
    Card(u32),
    CardPart(u32, CardPart),
    Preview,
    Modal,
    MonthForm,
    MonthTitle,
    MonthPrev,
    MonthNext,
    MonthCell(usize),
    EditForm,
    EditHeading,
    EditField(EditField),
    EditMessage,
}

impl Target {
    pub fn to_dom(self) -> Id {
        let surface = || Vec::from([(Tag::Main, None), (Tag::Section, None)]);
        let nav = |n| [(Tag::Header, None), (Tag::Nav, Some(n))];
        let modal_form = |n| [(Tag::Modal, None), (Tag::Form, Some(n))];
        let list = |n, item| {
            let mut path = surface();
            path.extend([(Tag::Ol, Some(n)), (Tag::Li, item)]);
            path
        };
        let join = |mut path: Vec<(Tag, Option<u32>)>, tail: &[(Tag, Option<u32>)]| {
            path.extend_from_slice(tail);
            path
        };
        let path = match self {
            Self::Title => Vec::from([(Tag::Header, None), (Tag::H2, None)]),
            Self::ViewButton(view) => {
                let n = VIEWS.iter().position(|v| *v == view).map_or(0, |i| i as u32 + 1);
                join(nav(1).into(), &[(Tag::Div, Some(1)), (Tag::Button, Some(n))])
            }
            Self::Step(n) => join(nav(1).into(), &[(Tag::Div, Some(2)), (Tag::Button, Some(n))]),
            Self::Zoom => join(nav(2).into(), &[(Tag::Input, None)]),
            Self::Reload => join(nav(2).into(), &[(Tag::Button, Some(RELOAD_BUTTON))]),
            Self::MonthOpen => join(nav(2).into(), &[(Tag::Button, Some(MONTH_BUTTON))]),
            Self::Save => join(nav(2).into(), &[(Tag::Button, Some(SAVE_BUTTON))]),
            Self::TimeAxis(n) => Vec::from([(Tag::Main, None), (Tag::Ol, Some(n))]),
            Self::Surface => surface(),
            Self::DayList => join(surface(), &[(Tag::Ol, Some(1))]),
            Self::Day(n) => list(1, Some(n)),
            Self::DayTitle(n) => join(list(1, Some(n)), &[(Tag::H3, None)]),
            Self::Resource(n) => list(2, Some(n)),
            Self::ResourceName(n) => join(list(2, Some(n)), &[(Tag::Strong, None)]),
            Self::ResourcePerson(n) => join(list(2, Some(n)), &[(Tag::Span, None)]),
            Self::Band(n) => list(3, Some(n)),
            Self::Card(n) => list(4, Some(n)),
            Self::CardPart(n, part) => join(
                list(4, Some(n)),
                match part {
                    CardPart::Status => &[(Tag::P, Some(1)), (Tag::Span, Some(1))],
                    CardPart::Time => &[(Tag::P, Some(1)), (Tag::Span, Some(2))],
                    CardPart::Title => &[(Tag::H3, None)],
                    CardPart::Category => &[(Tag::P, Some(2))],
                    CardPart::Note => &[(Tag::P, Some(3))],
                },
            ),
            Self::Preview => list(5, None),
            Self::Modal => Vec::from([(Tag::Modal, None)]),
            Self::MonthForm => modal_form(1).into(),
            Self::MonthTitle => join(modal_form(1).into(), &[(Tag::Header, None), (Tag::H3, None)]),
            Self::MonthPrev => {
                join(modal_form(1).into(), &[(Tag::Header, None), (Tag::Button, Some(1))])
            }
            Self::MonthNext => {
                join(modal_form(1).into(), &[(Tag::Header, None), (Tag::Button, Some(2))])
            }
            Self::MonthCell(index) => {
                let (row, column) =
                    (index as u32 / MONTH_COLUMNS + 1, index as u32 % MONTH_COLUMNS + 1);
                join(
                    modal_form(1).into(),
                    &[
                        (Tag::Table, None),
                        (Tag::Tbody, None),
                        (Tag::Tr, Some(row)),
                        (Tag::Td, Some(column)),
                        (Tag::Button, None),
                    ],
                )
            }
            Self::EditForm => modal_form(2).into(),
            Self::EditHeading => {
                join(modal_form(2).into(), &[(Tag::Header, None), (Tag::H3, None)])
            }
            Self::EditField(field) => {
                let n = FIELDS.iter().position(|f| *f == field).map_or(0, |i| i as u32 + 1);
                let tag = match field {
                    EditField::Status | EditField::Resource => Tag::Select,
                    EditField::Note => Tag::Textarea,
                    _ => Tag::Input,
                };
                join(modal_form(2).into(), &[(Tag::Dl, None), (Tag::Dd, Some(n)), (tag, None)])
            }
            Self::EditMessage => join(modal_form(2).into(), &[(Tag::Output, None)]),
        };
        Id::new(&path)
    }

    pub fn from_dom(id: &Id) -> Option<Self> {
        let path: Vec<(&Tag, Option<u32>)> = id.0.iter().map(|s| (&s.tag, s.n)).collect();
        Some(match path.as_slice() {
            [(Tag::Header, None), (Tag::H2, None)] => Self::Title,
            [
                (Tag::Header, None),
                (Tag::Nav, Some(1)),
                (Tag::Div, Some(1)),
                (Tag::Button, Some(n)),
            ] => Self::ViewButton(*VIEWS.get(n.checked_sub(1)? as usize)?),
            [
                (Tag::Header, None),
                (Tag::Nav, Some(1)),
                (Tag::Div, Some(2)),
                (Tag::Button, Some(n)),
            ] => Self::Step(*n),
            [(Tag::Header, None), (Tag::Nav, Some(2)), (Tag::Input, None)] => Self::Zoom,
            [(Tag::Header, None), (Tag::Nav, Some(2)), (Tag::Button, Some(n))] => match *n {
                RELOAD_BUTTON => Self::Reload,
                MONTH_BUTTON => Self::MonthOpen,
                SAVE_BUTTON => Self::Save,
                _ => return None,
            },
            [(Tag::Main, None), (Tag::Ol, Some(n))] => Self::TimeAxis(*n),
            [(Tag::Main, None), (Tag::Section, None), rest @ ..] => match rest {
                [] => Self::Surface,
                [(Tag::Ol, Some(1))] => Self::DayList,
                [(Tag::Ol, Some(1)), (Tag::Li, Some(n))] => Self::Day(*n),
                [(Tag::Ol, Some(1)), (Tag::Li, Some(n)), (Tag::H3, None)] => Self::DayTitle(*n),
                [(Tag::Ol, Some(2)), (Tag::Li, Some(n))] => Self::Resource(*n),
                [(Tag::Ol, Some(2)), (Tag::Li, Some(n)), (Tag::Strong, None)] => {
                    Self::ResourceName(*n)
                }
                [(Tag::Ol, Some(2)), (Tag::Li, Some(n)), (Tag::Span, None)] => {
                    Self::ResourcePerson(*n)
                }
                [(Tag::Ol, Some(3)), (Tag::Li, Some(n))] => Self::Band(*n),
                [(Tag::Ol, Some(4)), (Tag::Li, Some(n)), tail @ ..] => match tail {
                    [(Tag::P, Some(1)), (Tag::Span, Some(1))] => {
                        Self::CardPart(*n, CardPart::Status)
                    }
                    [(Tag::P, Some(1)), (Tag::Span, Some(2))] => Self::CardPart(*n, CardPart::Time),
                    [(Tag::H3, None)] => Self::CardPart(*n, CardPart::Title),
                    [(Tag::P, Some(2))] => Self::CardPart(*n, CardPart::Category),
                    [(Tag::P, Some(3))] => Self::CardPart(*n, CardPart::Note),
                    _ => Self::Card(*n),
                },
                [(Tag::Ol, Some(5)), (Tag::Li, None)] => Self::Preview,
                _ => Self::Surface,
            },
            [(Tag::Modal, None), rest @ ..] => match rest {
                [] => Self::Modal,
                [(Tag::Form, Some(1))] => Self::MonthForm,
                [(Tag::Form, Some(1)), (Tag::Header, None), (Tag::H3, None)] => Self::MonthTitle,
                [(Tag::Form, Some(1)), (Tag::Header, None), (Tag::Button, Some(1))] => {
                    Self::MonthPrev
                }
                [(Tag::Form, Some(1)), (Tag::Header, None), (Tag::Button, Some(2))] => {
                    Self::MonthNext
                }
                [
                    (Tag::Form, Some(1)),
                    (Tag::Table, None),
                    (Tag::Tbody, None),
                    (Tag::Tr, Some(row)),
                    (Tag::Td, Some(column)),
                    (Tag::Button, None),
                ] if (1..=MONTH_ROWS).contains(row) && (1..=MONTH_COLUMNS).contains(column) => {
                    Self::MonthCell(((row - 1) * MONTH_COLUMNS + column - 1) as usize)
                }
                [(Tag::Form, Some(2))] => Self::EditForm,
                [(Tag::Form, Some(2)), (Tag::Header, None), (Tag::H3, None)] => Self::EditHeading,
                [(Tag::Form, Some(2)), (Tag::Output, None)] => Self::EditMessage,
                [(Tag::Form, Some(2)), (Tag::Dl, None), (Tag::Dd, Some(n)), (tag, None)] => {
                    let field = *FIELDS.get(n.checked_sub(1)? as usize)?;
                    let expected = Self::EditField(field).to_dom();
                    (expected.0.last()?.tag == **tag).then_some(Self::EditField(field))?
                }
                _ => return None,
            },
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
            Target::Title,
            Target::Zoom,
            Target::Reload,
            Target::MonthOpen,
            Target::Save,
            Target::Surface,
            Target::DayList,
            Target::Preview,
            Target::Modal,
            Target::MonthForm,
            Target::MonthTitle,
            Target::MonthPrev,
            Target::MonthNext,
            Target::EditForm,
            Target::EditHeading,
            Target::EditMessage,
        ]);
        targets.extend(VIEWS.map(Target::ViewButton));
        targets.extend(FIELDS.map(Target::EditField));
        targets.extend((1..=5).map(Target::Step));
        targets.extend((1..=2).map(Target::TimeAxis));
        targets.extend((0..42).map(Target::MonthCell));
        for n in 1..=7 {
            targets.extend([Target::Day(n), Target::DayTitle(n), Target::Resource(n)]);
            targets.extend([Target::ResourceName(n), Target::ResourcePerson(n), Target::Band(n)]);
        }
        for n in 1..=3 {
            targets.push(Target::Card(n));
            targets.extend(
                [
                    CardPart::Status,
                    CardPart::Time,
                    CardPart::Title,
                    CardPart::Category,
                    CardPart::Note,
                ]
                .map(|part| Target::CardPart(n, part)),
            );
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
        let html = fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/calendar/index.html"
        ))
        .unwrap();
        let present = |id: &str| html.contains(&format!("id=\"{id}\""));
        for target in every_target() {
            let id = html_id(&target.to_dom());
            let pooled = matches!(
                target,
                Target::Card(_) | Target::CardPart(..) | Target::Band(_) | Target::Preview
            );
            assert!(pooled || present(&id), "{target:?} -> {id}");
        }
    }

    #[test]
    fn descendants_of_a_card_resolve_to_the_card_and_other_surface_content_to_the_surface() {
        let inside_card = Id::new(&[
            (Tag::Main, None),
            (Tag::Section, None),
            (Tag::Ol, Some(4)),
            (Tag::Li, Some(2)),
            (Tag::P, Some(1)),
        ]);
        assert_eq!(Target::from_dom(&inside_card), Some(Target::Card(2)));
        let elsewhere = Id::new(&[(Tag::Main, None), (Tag::Section, None), (Tag::Ol, Some(9))]);
        assert_eq!(Target::from_dom(&elsewhere), Some(Target::Surface));
    }

    #[test]
    fn unknown_ids_resolve_to_nothing() {
        let unknown = [
            Id::new(&[(Tag::Footer, None)]),
            Id::new(&[
                (Tag::Header, None),
                (Tag::Nav, Some(1)),
                (Tag::Div, Some(1)),
                (Tag::Button, Some(4)),
            ]),
            Id::new(&[(Tag::Header, None), (Tag::Nav, Some(2)), (Tag::Button, Some(9))]),
            Id::new(&[(Tag::Modal, None), (Tag::Form, Some(3))]),
        ];
        for id in unknown {
            assert_eq!(Target::from_dom(&id), None);
        }
    }
}
