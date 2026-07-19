use core::{option::Option::{self, Some, None}, marker::Copy, clone::Clone, todo};
use alloc::{vec::Vec, vec};
use arbitrary_int::u2;
use crate::Lang;
use crate::js_client::{Command, EventType, Gesture, dom::{Id, Tag}, CanvasEvent, PointerState};
use crate::file_store::FileStore;
use crate::data_struct::DataStruct;
use crate::object::{
    Dice, dice,
    Character, Profile, Characteristic, SecondaryAttribute, Skill, Possession, Backstory, Memo,
    LanguageOwn, ArtAndCraft, Fighting, Firearms, Pilot, Science, Survival,
    HitPoints, MagicPoints, Luck, Sanity, Build, DamageBonus, MoveRate, OccupationSkillPoints, InterestSkillPoints,
    ArtAndCraftCustom, FightingCustom, FirearmsCustom, LanguageOther, PilotCustom, ScienceCustom, SurvivalCustom,
};

// ============================================================
// Event
// ============================================================

pub enum Event {
    Ready,
    Canvas(CanvasEvent),
    Gesture(Gesture),
}

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
    Input  { step: u8, value: u32 }, // #main_modal 入力UI表示状態
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
    pub fn initial_draw(&self) -> (Vec<Event>, Vec<Command>) {
        (Vec::new(), Vec::new())
    }
    pub fn process(&mut self, event: &CanvasEvent, _pointer_state: &PointerState) -> (Vec<Event>, Vec<Command>) {
        let id = &event.id;
        let commands = if matches!(event.event_type, EventType::Click)
            && id == &Id::new(&[(Tag::Header, None), (Tag::Button, Some(3))]) {
            match self.character_sheet {
                CharacterSheet::Immutable => {
                    self.character_sheet = CharacterSheet::Editable;
                    vec![
                        Command::RemoveClass {
                            id:    Id::new(&[(Tag::Main, None), (Tag::Section, Some(1))]).encode(),
                            value: "hidden".to_string(),
                        },
                        Command::AddClass {
                            id:    Id::new(&[(Tag::Main, None), (Tag::Section, Some(2))]).encode(),
                            value: "hidden".to_string(),
                        },
                    ]
                }
                CharacterSheet::Editable => {
                    self.character_sheet = CharacterSheet::Immutable;
                    vec![
                        Command::RemoveClass {
                            id:    Id::new(&[(Tag::Main, None), (Tag::Section, Some(2))]).encode(),
                            value: "hidden".to_string(),
                        },
                        Command::AddClass {
                            id:    Id::new(&[(Tag::Main, None), (Tag::Section, Some(1))]).encode(),
                            value: "hidden".to_string(),
                        },
                    ]
                }
            }
        } else {
            Vec::new()
        };
        (Vec::new(), commands)
    }
    pub fn process_gesture(&mut self, _gesture: &Gesture, _pointer_state: &PointerState) -> (Vec<Event>, Vec<Command>) {
        (Vec::new(), Vec::new())
    }
}

// ============================================================
// internal helper
// ============================================================

/// mapping object::{Objects}::read() <-> dom::Id
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

/// 各SkillのCustomスロット(indirect id list)を走査し、既に使用中のschema_idを収集する。
/// Customへ新規idを割り当てる前にHandlerが一度だけ実行し、使用状況を把握するために使う。
/// 使用状況を別途メタデータとして持たず、都度character自体を走査して求める。
fn used_custom_ids(character: &DataStruct) -> Vec<u32> {
    const LIST_IDS: [u32; 7] = [
        ArtAndCraftCustom::list_id(),
        FightingCustom::list_id(),
        FirearmsCustom::list_id(),
        LanguageOther::list_id(),
        PilotCustom::list_id(),
        ScienceCustom::list_id(),
        SurvivalCustom::list_id(),
    ];
    let mut used = Vec::new();
    for list_id in LIST_IDS {
        let Ok(bytes) = character.get(list_id) else { continue; };
        let count = bytes.len() / 8; // (numeric_id, name_id) = u32 * 2 ペアごと
        for i in 0..count {
            if let [Some(ids)] = character.get_indirect::<1, 2>(list_id, [i]) {
                used.extend(ids.into_iter().filter(|&id| id != 0));
            }
        }
    }
    used
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
            Command::SetText  { id: span.encode(),    value: self.icon().to_string() },
            Command::SetText  { id: p.encode(),        value: self.label(state.lang).to_string() },
            Command::AddClass { id: article.encode(),  value: self.css_class().to_string() },
            Command::JsFn     { id: article.encode(),  name: "show".to_string() },
        ]
    }
}
