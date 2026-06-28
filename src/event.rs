use core::{option::Option::{self, Some, None}, marker::Copy, clone::Clone, todo};
use alloc::{vec::Vec, vec, string::String};
use arbitrary_int::u2;
use crate::Lang;
use crate::js_client::{Command, Operation, EventType, Gesture, dom::{Id, Tag}, CanvasEvent};
use crate::store::FileStore;
use crate::data_struct::DataStruct;
use crate::model::{
    Dice, dice,
    Character, Profile, Characteristic, SecondaryAttribute, Skill, Possession, Backstory, Memo,
    LanguageOwn, ArtAndCraft, Fighting, Firearms, Pilot, Science, Survival,
    HitPoints, MagicPoints, Luck, Sanity, Build, DamageBonus, MoveRate, OccupationSkillPoints, InterestSkillPoints,
};

// ============================================================
// viewport state
// ============================================================

pub enum CharacterSheet {
    Immutable,
    Editable,
}

#[derive(Clone, Copy)]
pub enum Dialog {
    None,
    Drawer, // #drawer
    Select { step: u8, index: u32 }, // #main_modal セレクトUI表示状態
    Input  { step: u8, value: u32 },   // #main_modal 入力UI表示状態
}

// ============================================================
// event handler
// ============================================================

const CHARACTER_SCHEMA_NAME: &str = "characters";

pub struct Log;

pub struct Handler {
    character_sheet: CharacterSheet,
    dialog:     Dialog,
    lang:       Lang,
    last_toast: u2,
    character:  DataStruct,
    characters: FileStore,
    logs:       Vec<Log>,
}

impl Handler {
    pub async fn ready(_viewport_width: f64, _viewport_height: f64) -> Self {
        Self {
            character_sheet: CharacterSheet::Immutable,
            dialog:     Dialog::None,
            lang:       Lang::Ja,
            last_toast: u2::new(1), // todo last_toastをnext_toastにrenameするか検討
            character:  DataStruct::new(0, 0.0, 256),
            characters: FileStore::new(CHARACTER_SCHEMA_NAME).await
                .unwrap_or_else(|e| panic!("FileStore::new failed: {}", e)),
            logs:       Vec::new(),
        }
    }
    pub fn close(&self) {
        self.characters.close();
    }
    pub fn initial_draw(&self) -> Vec<Command> {
        Vec::new()
    }
    pub fn process(&mut self, event: &CanvasEvent) -> Vec<Command> {
        let id = &event.id;
        if matches!(event.event_type, EventType::Click)
            && id == &Id::new(&[(Tag::Header, None), (Tag::Button, Some(3))]) {
            match self.character_sheet {
                CharacterSheet::Immutable => {
                    self.character_sheet = CharacterSheet::Editable;
                    vec![
                        Command::new(
                            Operation::RemoveClass,
                            &Id::new(&[(Tag::Main, None), (Tag::Section, Some(1))]).encode(),
                            None,
                            Some("hidden"),
                        ),
                        Command::new(
                            Operation::AddClass,
                            &Id::new(&[(Tag::Main, None), (Tag::Section, Some(2))]).encode(),
                            None,
                            Some("hidden"),
                        ),
                    ]
                }
                CharacterSheet::Editable => {
                    self.character_sheet = CharacterSheet::Immutable;
                    vec![
                        Command::new(
                            Operation::RemoveClass,
                            &Id::new(&[(Tag::Main, None), (Tag::Section, Some(2))]).encode(),
                            None,
                            Some("hidden"),
                        ),
                        Command::new(
                            Operation::AddClass,
                            &Id::new(&[(Tag::Main, None), (Tag::Section, Some(1))]).encode(),
                            None,
                            Some("hidden"),
                        ),
                    ]
                }
            }
        } else {
            vec![]
        }
    }
    pub fn process_gesture(&mut self, gesture: &Gesture) -> Vec<Command> {
        vec![]
    }
}

// ============================================================
// internal helper
// ============================================================

/// mapping model::{Models}::read() <-> dom::Id
fn map_id(item: &Character, parent: &Id, n: u32) -> Vec<Id> {
    match parent {
        p if p == &Id::new(&[(Tag::Section, Some(1))]) => {
            let section_n = match item {
                Character::Profile         => 2,
                Character::Characteristic  => 3,
                Character::SecondaryAttribute => 4,
                Character::Skill      => 5,
                Character::Possession => 6,
                Character::Backstory  => 7,
                Character::Memo       => 8,
            };
            let base: Vec<(Tag, Option<u32>)> = vec![
                (Tag::Main,    None),
                (Tag::Section, Some(1)),
                (Tag::Section, Some(section_n)),
                (Tag::Span,    Some(n)),
            ];
            vec![
                Id::new(&[base.as_slice(), &[(Tag::Span, Some(1))]].concat()),  // label
                Id::new(&[base.as_slice(), &[(Tag::Span, Some(2))]].concat()),  // value
            ]
        }
        p if p == &Id::new(&[(Tag::Section, Some(2))]) => {
            let fieldset_n = match item {
                Character::Profile         => 2,
                Character::Characteristic  => 3,
                Character::SecondaryAttribute => 4,
                Character::Skill      => 5,
                Character::Possession => 6,
                Character::Backstory  => 7,
                Character::Memo       => 8,
            };
            let tr: Vec<(Tag, Option<u32>)> = vec![
                (Tag::Modal,    None),
                (Tag::Fieldset, Some(fieldset_n)),
                (Tag::Table,    None),
                (Tag::Tr,       Some(n)),
            ];
            let s = tr.as_slice();
            match item {
                Character::Profile => vec![
                    // [0] th, [1] input
                    Id::new(&[s, &[(Tag::Th,    None   )]].concat()),
                    Id::new(&[s, &[(Tag::Input, None   )]].concat()),
                ],
                Character::Characteristic => vec![
                    // [0] th, [1] input-1(初期値), [2] input-2(変動), [3] input-3(補正), [4] span(合計)
                    Id::new(&[s, &[(Tag::Th,    None   )]].concat()),
                    Id::new(&[s, &[(Tag::Input, Some(1))]].concat()),
                    Id::new(&[s, &[(Tag::Input, Some(2))]].concat()),
                    Id::new(&[s, &[(Tag::Input, Some(3))]].concat()),
                    Id::new(&[s, &[(Tag::Span,  None   )]].concat()),
                ],
                Character::Skill => vec![
                    // [0] th, [1] span-1(base), [2] input-1(職業), [3] input-2(興味), [4] input-3(補正), [5] span-2(合計), [6] td-1_select
                    Id::new(&[s, &[(Tag::Th,     None   )]].concat()),
                    Id::new(&[s, &[(Tag::Span,   Some(1))]].concat()),
                    Id::new(&[s, &[(Tag::Input,  Some(1))]].concat()),
                    Id::new(&[s, &[(Tag::Input,  Some(2))]].concat()),
                    Id::new(&[s, &[(Tag::Input,  Some(3))]].concat()),
                    Id::new(&[s, &[(Tag::Span,   Some(2))]].concat()),
                    Id::new(&[s, &[(Tag::Td, Some(1)), (Tag::Select, None)]].concat()),
                ],
                _ => todo!(),
            }
        }
        _ => todo!(),
    }
}

// --- toast ---

pub enum Toast { Saved, Discarded, Synced }

impl Toast {
    fn icon(&self) -> &'static str {
        match self {
            Self::Saved     => "💾",
            Self::Discarded => "🗑️",
            Self::Synced    => "☁️",
        }
    }

    fn label(&self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::Saved,     Lang::En(_)) => "Saved",
            (Self::Saved,     Lang::Ja)    => "保存しました",
            (Self::Discarded, Lang::En(_)) => "Discarded",
            (Self::Discarded, Lang::Ja)    => "破棄しました",
            (Self::Synced,    Lang::En(_)) => "Synced",
            (Self::Synced,    Lang::Ja)    => "同期しました",
        }
    }

    fn css_class(&self) -> &'static str {
        match self {
            Self::Saved     => "success",
            Self::Discarded => "warning",
            Self::Synced    => "info"
        }
    }

    pub fn commands(&self, state: &mut Handler) -> Vec<Command> {
        let n = if state.last_toast == u2::new(1) { u2::new(2) } else { u2::new(1) };
        state.last_toast = n;
        let article = Id::new(&[(Tag::Output, None), (Tag::Article, Some(n.value() as u32))]);
        let span    = Id::new(&[(Tag::Output, None), (Tag::Article, Some(n.value() as u32)), (Tag::Span, None)]);
        let p       = Id::new(&[(Tag::Output, None), (Tag::Article, Some(n.value() as u32)), (Tag::P,    None)]);
        vec![
            Command::new(Operation::SetText,  &span.encode(),    None, Some(self.icon())),
            Command::new(Operation::SetText,  &p.encode(),       None, Some(self.label(state.lang))),
            Command::new(Operation::AddClass, &article.encode(), None, Some(self.css_class())),
            Command::new(Operation::JsFn,     &article.encode(), None, Some("show")),
        ]
    }
}
