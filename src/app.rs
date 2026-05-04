use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use crate::{Lang, dice::{self, ResultLevel}};
use crate::table::Roll;
use crate::character::{Instance, Model, schema};
use crate::js_client::{
    DomCmd, Operation,
    EventType, KeyName,
    get_js_str, get_js_field,
};

// ============================================================
// UI State
// ============================================================

#[derive(Clone, Copy, PartialEq)]
pub enum SkillSelectorMode { Roll, Push, DevCheck }

#[derive(Clone, Copy, PartialEq)]
enum DicePhase { Count, Sides, Modifier }

enum State {
    Idle,
    Selector      { idx: usize },
    CharSelector  { idx: usize },
    SkillSelector { mode: SkillSelectorMode, idx: usize },
    DiceInput     { phase: DicePhase, count: u32, sides_idx: usize, modifier: i32 },
}

const DICE_SIDES: &[u32] = &[2, 3, 4, 5, 6, 8, 10, 12, 20, 100];

// ============================================================
// App
// ============================================================

#[wasm_bindgen]
pub struct App {
    state:     State,
    dom_cmds:  Vec<DomCmd>,
    roll_log:  Vec<RollLog>,
    character: Instance,
}

#[wasm_bindgen]
impl App {
    pub fn init() -> App {
        App {
            state:     State::Idle,
            dom_cmds:  Vec::new(),
            roll_log:  Vec::new(),
            character: Instance::new(),
        }
    }

    pub fn flush(&mut self) -> JsValue {
        let out = serde_wasm_bindgen::to_value(&self.dom_cmds).unwrap_or(JsValue::NULL);
        self.dom_cmds.clear();
        out
    }

    pub fn event(&mut self, payload: JsValue) {
        let event_type   = EventType::decode(&get_js_str(&payload, "event_type").unwrap_or_default());
        let target_id = get_js_str(&payload, "target_id").unwrap_or_default();
        let key_str   = get_js_str(&payload, "key").unwrap_or_default();

        let dom_cmds: Vec<DomCmd> = match event_type {
            EventType::Submit if target_id == "chat_form" => {
                let fields = get_js_field(&payload, "value").unwrap_or(JsValue::NULL);
                let text = get_js_str(&fields, "text").unwrap_or_default();
                self.on_chat_submit(&text)
            }
            EventType::Submit if target_id == "char_edit_form" => {
                let fields = get_js_field(&payload, "value").unwrap_or(JsValue::NULL);
                self.on_char_edit_save(&fields)
            }
            EventType::Input if target_id == "chat_input" => {
                let value = get_js_str(&payload, "value").unwrap_or_default();
                self.on_chat_input(&value)
            }
            EventType::KeyDown => {
                self.on_keydown(KeyName::decode(&key_str))
            }
            EventType::Click => {
                self.on_click(ClickTarget::parse(&target_id))
            }
            EventType::FocusIn => {
                self.on_focus(&target_id);
                vec![]
            }
            _ => vec![],
        };

        self.dom_cmds.extend(cmds);
    }
}