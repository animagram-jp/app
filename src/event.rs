use arbitrary_int::u2;
use crate::Lang;
use crate::js_client::{CanvasCmd, Operation, EventType, Gesture, dom::{Id, Tag}, CanvasEvent};
use crate::store::DiskStore;
use crate::data_struct::DataStruct;
use crate::character::{
    Character, Profile, Characteristic, Skill,
    ArtCraftSpec, FightingSpec, FirearmsSpec, PilotSpec, ScienceSpec, SurvivalSpec,
};

// ============================================================
// canvas state
// ============================================================

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Dialog {
    #[default]
    None,
    Modal,
    Drawer,
    Select { step: u8, index: usize }, // dialog id="main_div_modal" aria-label="overlay" のセレクトUI表示状態
    Input  { step: u8, value: u32 },   // dialog id="main_div_modal" aria-label="overlay" の入力UI表示状態
}

pub struct CanvasState { // dialog + lang + editing + dom map(static)  -> canvas commands
    pub dialog:      Dialog,
    pub lang:        Lang,
    pub editing:     DataStruct,
    pub last_toast:  u2,
}

impl CanvasState {
    pub fn new() -> Self {
        Self {
            dialog:     Dialog::default(),
            lang:       Lang::Ja,
            editing:    DataStruct::new(0, 0.0, 256),
            last_toast: u2::new(1),
        }
    }
}

// ============================================================
// global pub fn
// ============================================================

pub fn initial_draw(_handler: &Coc7th) -> Vec<CanvasCmd> {
    todo!()
}

pub fn handle_gesture(gesture: Gesture, state: &mut CanvasState, handler: &mut Coc7th) -> Vec<CanvasCmd> {
    todo!("ジェスチャー処理")
}

pub fn output_commands(_canvas_state: &CanvasState) {
    todo!()
}



// ============================================================
// event handlers
// ============================================================

const CHARACTER_SCHEMA_NAME: &str = "characters";

pub struct Log;

pub struct Coc7th {
    characters: DiskStore,
    log_stack:  Vec<Log>,
}

impl Coc7th {
    pub async fn ready() -> Self {
        Self {
            characters: DiskStore::new(CHARACTER_SCHEMA_NAME).await
                .unwrap_or_else(|e| panic!("DiskStore::new failed: {}", e)),
            log_stack: Vec::new(),
        }
    }
}

// ============================================================
// handle event
// ============================================================

pub fn handle(state: &mut CanvasState, ev: &CanvasEvent, handler: &mut Coc7th) -> Vec<CanvasCmd> {
    match (&ev.event_type, state.dialog) {
        (EventType::Click,   Dialog::None)  => todo!("normal click"),
        (EventType::Click,   Dialog::Modal) => todo!("dialog click"),
        (EventType::KeyDown, _)             => todo!("keydown"),
        (EventType::Input,   _)             => todo!("input"),
        (EventType::Change,  _)             => todo!("change"),
        (EventType::Blur,    _)             => todo!("blur"),
        (EventType::Submit,  _)             => todo!("submit"),
        _                                   => vec![],
    }
}

// ============================================================
// map item to Id
// ============================================================

fn map_id(item: &Character, parent: &Id, n: u32) -> Vec<Id> {
    match parent {
        p if p == &Id::new(&[(Tag::Main, None)]) => {
            let section_n = match item {
                Character::Profile        => 1,
                Character::Characteristic => 2,
                Character::Skill          => 3,
                Character::Possession     => todo!(),
                Character::Backstory      => todo!(),
                Character::Memo           => todo!(),
                Character::OtherAttribute => todo!(),
            };
            let base: Vec<(Tag, Option<u32>)> = vec![
                (Tag::Main,    None),
                (Tag::Div,     None),
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
                Character::OtherAttribute => 4,
                Character::Possession     => 5,
                Character::Backstory      => 6,
                Character::Memo           => 7,
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

// Characteristic: input-1(初期値) + input-2(変動値) + input-3(補正値) → span(合計) をリアルタイム更新
pub fn on_characteristic_input(_row: usize, base: i32, delta: i32, bonus: i32) -> Vec<CanvasCmd> {
    let _total = (base + delta + bonus).max(1);
    todo!()
}

// Skill: 専門分野(td-1_input)が変わったら th のテキストを更新する
pub fn on_skill_spec_input(_row: usize, _skill: &Skill, _spec: &str) -> Vec<CanvasCmd> {
    todo!()
}

// Skill: 職業pt(input-1) か 興味pt(input-2) か 補正値(input-3) が変わったら合計spanを更新する
pub fn on_skill_input(_row: usize, _base: u16, _occ_pt: u16, _int_pt: u16, _bonus: i32) -> Vec<CanvasCmd> {
    todo!()
}

// fieldset-2 の1行: ロール値をキャッシュに書き込み、input-1とspanをSetValue/SetTextで更新
pub fn roll_characteristic(_row: usize, _char_data: &mut DataStruct) -> Vec<CanvasCmd> {
    todo!()
}

// legend button: 全Characteristicを一括ロール
pub fn roll_all_characteristics(_char_data: &mut DataStruct) -> Vec<CanvasCmd> {
    todo!()
}

pub fn restore_modal(ds: &DataStruct) -> Vec<CanvasCmd> {
    let mut commands = Vec::new();
    // todo
    commands
}

pub fn open_modal() -> Vec<CanvasCmd> {
    let mut commands = Vec::new();
    // todo
    commands
}

pub fn update_character_view(ds: &DataStruct) -> Vec<CanvasCmd> {
    let mut commands = Vec::new();
    commands
}

pub fn update_select(_list: &[(u32, String)], _selected_id: Option<u32>) -> Vec<CanvasCmd> {
    todo!()
}

pub fn reset_modal() -> Vec<CanvasCmd> {
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

    pub fn commands(&self, state: &mut CanvasState) -> Vec<CanvasCmd> {
        let n = if state.last_toast == u2::new(1) { u2::new(2) } else { u2::new(1) };
        state.last_toast = n;
        let article = Id::new(&[(Tag::Output, None), (Tag::Article, Some(n.value() as u32))]);
        let span    = Id::new(&[(Tag::Output, None), (Tag::Article, Some(n.value() as u32)), (Tag::Span, None)]);
        let p       = Id::new(&[(Tag::Output, None), (Tag::Article, Some(n.value() as u32)), (Tag::P,    None)]);
        vec![
            CanvasCmd::new(Operation::SetText,  &span.encode(),    None, Some(self.icon())),
            CanvasCmd::new(Operation::SetText,  &p.encode(),       None, Some(self.label(state.lang))),
            CanvasCmd::new(Operation::AddClass, &article.encode(), None, Some(self.css_class())),
            CanvasCmd::new(Operation::JsClass,  &article.encode(), None, Some("show")),
        ]
    }
}
