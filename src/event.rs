use core::{option::Option::{self, Some, None}, marker::Copy, clone::Clone, todo};
use alloc::{vec::Vec, vec, string::String};
use arbitrary_int::u2;
use crate::Lang;
use crate::js_client::{Command, Operation, EventType, Gesture, dom::{Id, Tag}, CanvasEvent};
use crate::store::FileStore;
use crate::data_struct::DataStruct;
use crate::model::{
    Character, Profile, Characteristic, Skill,
    ArtAndCraft, Fighting, Firearms, Pilot, Science, Survival,
};

// ============================================================
// Event Handler
// ============================================================

#[derive(Clone, Copy)]
pub enum Dialog {
    None,
    Modal,  // #modal
    Drawer, // #drawer
    Select { step: u8, index: u32 }, // #main_modal セレクトUI表示状態
    Input  { step: u8, value: u32 },   // #main_modal 入力UI表示状態
}

const CHARACTER_SCHEMA_NAME: &str = "characters";

pub struct Log;

pub struct Handler {
    dialog:     Dialog,
    lang:       Lang,
    last_toast: u2,
    character:  DataStruct,
    characters: FileStore,
    logs:       Vec<Log>,
}

impl Handler {
    pub async fn ready() -> Self {
        Self {
            dialog:     Dialog::None,
            lang:       Lang::Ja,
            last_toast: u2::new(1), // todo! 正しいか要確認
            character:  DataStruct::new(0, 0.0, 256),
            characters: FileStore::new(CHARACTER_SCHEMA_NAME).await
                .unwrap_or_else(|e| panic!("FileStore::new failed: {}", e)),
            logs: Vec::new(),
        }
    }
    pub fn close(&self) {
        self.characters.close();
    }

    pub fn initial_draw() -> Vec<Command> {
        Vec::new()
    }
    pub fn process(&mut self, event: &CanvasEvent) -> Vec<Command> {
        match (&event.event_type, self.dialog) {
            (EventType::Click, Dialog::None) => {
                match event.id.last_tag() {
                    // header_button-3: モーダルを開く
                    Some(Tag::Button) if event.id == Id::new(&[
                        (Tag::Header, None),
                        (Tag::Button, Some(3)),
                    ]) => {
                        self.dialog = Dialog::Modal;
                        open_modal()
                    }
                    _ => vec![],
                }
            }
            (EventType::Click, Dialog::Modal) => {
                if event.id == Id::new(&[(Tag::Modal, None)]) {
                    self.dialog = Dialog::None;
                    close_modal()
                } else {
                    vec![]
                }
            }
            (EventType::KeyDown, _)             => todo!("keydown"),
            (EventType::Input,   _)             => todo!("input"),
            (EventType::Change,  _)             => todo!("change"),
            (EventType::Blur,    _)             => todo!("blur"),
            (EventType::Submit,  _)             => todo!("submit"),
            _                                   => vec![],
        }
    }
    pub fn process_gesture(gesture: &Gesture) -> Vec<Command> {
        todo!()
    }
}

// ============================================================
// map item to dom::Id
// ============================================================

fn map_id(item: &Character, parent: &Id, n: u32) -> Vec<Id> {
    match parent {
        p if p == &Id::new(&[(Tag::Main, None)]) => {
            let section_n = match item {
                Character::Profile        => 2,
                Character::Characteristic => 3,
                Character::SecondaryAttribute => 4,
                Character::Skill          => 5,
                Character::Possession     => 6,
                Character::Backstory      => 7,
                Character::Memo           => 8,
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
        p if p == &Id::new(&[(Tag::Modal, None)]) => {
            let fieldset_n = match item {
                Character::Profile        => 1,
                Character::Characteristic => 2,
                Character::Skill          => 3,
                Character::Backstory      => 4,
                Character::Memo           => 5,
                Character::SecondaryAttribute => todo!("SecondaryAttributeのfieldsetはmodalに未実装"),
                Character::Possession     => todo!("Possessionのfieldsetはmodalに未実装"),
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

// ============================================================
// action
// ============================================================

pub fn close_modal() -> Vec<Command> {
    let modal = Id::new(&[(Tag::Modal, None)]);
    vec![Command::new(Operation::CloseModal, &modal.encode(), None, None)]
}

pub fn open_modal() -> Vec<Command> {
    let mut commands = Vec::new();

    let modal = Id::new(&[(Tag::Modal, None)]);
    commands.push(Command::new(Operation::OpenModal, &modal.encode(), None, None));

    commands
}

// Characteristic: input-1(初期値) + input-2(変動値) + input-3(補正値) → span(合計) をリアルタイム更新
pub fn on_characteristic_input(_row: usize, base: i32, delta: i32, bonus: i32) -> Vec<Command> {
    let _total = (base + delta + bonus).max(1);
    todo!()
}

// Skill: 専門分野(td-1_input)が変わったら th のテキストを更新する
pub fn on_skill_spec_input(_row: usize, _skill: &Skill, _spec: &str) -> Vec<Command> {
    todo!()
}

// Skill: 職業pt(input-1) か 興味pt(input-2) か 補正値(input-3) が変わったら合計spanを更新する
pub fn on_skill_input(_row: usize, _base: u16, _occ_pt: u16, _int_pt: u16, _bonus: i32) -> Vec<Command> {
    todo!()
}

// modal header button: 全Characteristicを一括ロール
pub fn roll_characteristics(_char_data: &mut DataStruct) -> Vec<Command> {
    todo!()
}

pub fn restore_modal(ds: &DataStruct) -> Vec<Command> {
    let mut commands = Vec::new();
    // todo
    commands
}



pub fn update_character_view(ds: &DataStruct) -> Vec<Command> {
    let mut commands = Vec::new();
    commands
}

pub fn update_select(_list: &[(u32, String)], _selected_id: Option<u32>) -> Vec<Command> {
    todo!()
}

pub fn reset_modal() -> Vec<Command> {
    let mut commands = Vec::new();
    commands
}

// ============================================================
// Toast
// ============================================================

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
            (Self::Saved,     Lang::En) => "Saved",
            (Self::Saved,     Lang::Ja) => "保存しました",
            (Self::Discarded, Lang::En) => "Discarded",
            (Self::Discarded, Lang::Ja) => "破棄しました",
            (Self::Synced,    Lang::En) => "Synced",
            (Self::Synced,    Lang::Ja) => "同期しました",
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
            Command::new(Operation::JsClass,  &article.encode(), None, Some("show")),
        ]
    }
}
