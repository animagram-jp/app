// This file includes untranslated text (ja).

use wasm_bindgen::prelude::*;
use crate::dice;
use crate::static::Roll;

const OP_SET_ATTR: u32 = 0b01;
const OP_SET_TEXT: u32 = 0b10;

fn js_get_str(obj: &JsValue, key: &str) -> String {
    js_sys::Reflect::get(obj, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default()
}

fn js_get_u32(obj: &JsValue, key: &str) -> u32 {
    js_sys::Reflect::get(obj, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as u32
}

fn js_get_field(obj: &JsValue, key: &str) -> JsValue {
    js_sys::Reflect::get(obj, &JsValue::from_str(key))
        .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub struct App {
    selector_open: bool,
    chat_log: String,
}

#[wasm_bindgen]
impl App {
    pub fn init() -> App {
        App { selector_open: false, chat_log: String::new() }
    }

    pub fn event(&mut self, payload: JsValue) -> JsValue {
        let event_type = js_get_u32(&payload, "event_type");
        let target_id  = js_get_str(&payload, "target_id");

        let cmds: Vec<DomCmd> = match (event_type, target_id.as_str()) {
            (0b11, "chat-form") => {
                let fields = js_get_field(&payload, "fields");
                let text = js_get_str(&fields, "text");
                self.handle_submit(&text)
            }
            (0b01, id) if id.starts_with("roll-") => {
                let key = id.strip_prefix("roll-").unwrap_or("");
                self.handle_roll_select(key)
            }
            (0b01, "selector-overlay") => {
                self.selector_open = false;
                vec![set_attr("selector", "hidden", "true")]
            }
            _ => vec![],
        };

        serde_wasm_bindgen::to_value(&cmds).unwrap_or(JsValue::NULL)
    }

    fn handle_submit(&mut self, text: &str) -> Vec<DomCmd> {
        let trimmed = text.trim();
        if trimmed == "/" {
            self.selector_open = true;
            return vec![
                set_attr("selector", "hidden", ""),
                set_attr("chat-input", "value", ""),
            ];
        }
        if trimmed.is_empty() { return vec![]; }
        self.chat_log.push_str(trimmed);
        self.chat_log.push('\n');
        vec![
            set_text("chat-log", &self.chat_log),
            set_attr("chat-input", "value", ""),
        ]
    }

    fn handle_roll_select(&mut self, key: &str) -> Vec<DomCmd> {
        let roll = match key {
            "skill"       => Roll::SkillRoll,
            "char"        => Roll::CharacteristicRoll,
            "sanity"      => Roll::SanityRoll,
            "madness-rt"  => Roll::BoutOfMadnessRealTime,
            "madness-sum" => Roll::BoutOfMadnessSummary,
            "combine"     => Roll::CombinedSkillRoll,
            "pushed"      => Roll::PushedRoll,
            "autofire"    => Roll::AutoFireRoll,
            "cast-minor"  => Roll::FailedCastingMinor,
            "cast-major"  => Roll::FailedCastingMajor,
            "phobia"      => Roll::PhobiaTable,
            "mania"       => Roll::ManiaTable,
            _ => return vec![],
        };
        self.selector_open = false;
        let result = execute_roll(roll);
        self.chat_log.push_str(&result);
        vec![
            set_attr("selector", "hidden", "true"),
            set_text("chat-log", &self.chat_log),
        ]
    }
}

fn execute_roll(roll: Roll) -> String {
    match roll {
        Roll::BoutOfMadnessRealTime => {
            let r = dice::roll_madness_realtime();
            format!("【{}】{}\n", r.roll_type.label(), r)
        }
        Roll::BoutOfMadnessSummary => {
            let r = dice::roll_madness_summary();
            format!("【{}】{}\n", r.roll_type.label(), r)
        }
        Roll::FailedCastingMinor => {
            let r = dice::roll_failed_casting_minor();
            format!("【{}】{}\n", r.roll_type.label(), r)
        }
        Roll::FailedCastingMajor => {
            let r = dice::roll_failed_casting_major();
            format!("【{}】{}\n", r.roll_type.label(), r)
        }
        Roll::PhobiaTable => {
            let r = dice::roll_phobia();
            format!("【{}】{}\n", r.roll_type.label(), r)
        }
        Roll::ManiaTable => {
            let r = dice::roll_mania();
            format!("【{}】{}\n", r.roll_type.label(), r)
        }
        _ => format!("【{}】(パラメータ入力UI未実装)\n", roll.label()),
    }
}

// ============================================================
// DOM コマンド型
// ============================================================

#[derive(serde::Serialize)]
struct DomCmd {
    op: u32,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    attr: Option<String>,
    value: String,
}

fn set_text(id: &str, value: &str) -> DomCmd {
    DomCmd { op: OP_SET_TEXT, id: id.to_string(), attr: None, value: value.to_string() }
}

fn set_attr(id: &str, attr: &str, value: &str) -> DomCmd {
    DomCmd { op: OP_SET_ATTR, id: id.to_string(), attr: Some(attr.to_string()), value: value.to_string() }
}
