use wasm_bindgen::prelude::*;
use crate::{Lang, dice::{self, ResultLevel}};
use crate::table::Roll;
use crate::character::{Instance, schema};

const OP_SET_ATTR: u32 = 0b001;
const OP_SET_TEXT: u32 = 0b010;
const OP_FOCUS:    u32 = 0b100;

const EVENT_CLICK:    u32 = 0b001;
const EVENT_SUBMIT:   u32 = 0b010;
const EVENT_INPUT:    u32 = 0b011;
const EVENT_KEYDOWN:  u32 = 0b100;
const EVENT_CHANGE:   u32 = 0b101;
const EVENT_FOCUS:    u32 = 0b110;

const SELECTOR_ITEMS: &[&str] = &[
    "roll-skill",
    "roll-char",
    "roll-sanity",
    "roll-madness-rt",
    "roll-madness-sum",
    "roll-combine",
    "roll-pushed",
    "roll-autofire",
    "roll-cast-minor",
    "roll-cast-major",
    "roll-phobia",
    "roll-mania",
];

const CHAR_SELECTOR_ITEMS: &[&str] = &[
    "charroll-str",
    "charroll-con",
    "charroll-siz",
    "charroll-dex",
    "charroll-app",
    "charroll-int",
    "charroll-pow",
    "charroll-edu",
    "charroll-luk",
];

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
    selector_idx: usize,
    char_selector_open: bool,
    char_selector_idx: usize,
    roll_log: Vec<RollLog>,
    character: Instance,
}

#[wasm_bindgen]
impl App {
    pub fn init() -> App {
        App {
            selector_open: false,
            selector_idx: 0,
            char_selector_open: false,
            char_selector_idx: 0,
            roll_log: Vec::new(),
            character: Instance::new(),
        }
    }

    pub fn event(&mut self, payload: JsValue) -> JsValue {
        let event_type = js_get_u32(&payload, "event_type");
        let target_id  = js_get_str(&payload, "target_id");
        let key        = js_get_str(&payload, "key");

        let cmds: Vec<DomCmd> = match event_type {
            EVENT_SUBMIT if target_id == "chat-form" => {
                let fields = js_get_field(&payload, "fields");
                let text = js_get_str(&fields, "text");
                self.handle_submit(&text)
            }
            EVENT_INPUT if target_id == "chat-input" => {
                let value = js_get_str(&payload, "value");
                self.handle_input(&value)
            }
            EVENT_KEYDOWN => {
                self.handle_keydown(&key)
            }
            EVENT_CLICK if target_id.starts_with("roll-") => {
                let roll_key = target_id.strip_prefix("roll-").unwrap_or("");
                self.handle_roll_select(roll_key)
            }
            EVENT_CLICK if target_id == "selector-overlay" => {
                self.close_selector()
            }
            EVENT_CLICK if target_id == "char-selector-overlay" => {
                self.close_char_selector()
            }
            EVENT_CLICK if target_id.starts_with("charroll-") => {
                let key = target_id.strip_prefix("charroll-").unwrap_or("");
                self.handle_char_selector(key)
            }
            EVENT_CLICK if target_id == "char-roll" => {
                self.handle_char_roll()
            }
            EVENT_CHANGE if target_id.starts_with("char-") => {
                let value_str = js_get_str(&payload, "value");
                let value: u16 = value_str.trim().parse().unwrap_or(0);
                self.handle_char_change(&target_id, value)
            }
            EVENT_FOCUS if target_id.starts_with("roll-") => {
                if let Some(idx) = SELECTOR_ITEMS.iter().position(|&s| s == target_id) {
                    self.selector_idx = idx;
                }
                vec![]
            }
            EVENT_FOCUS if target_id.starts_with("charroll-") => {
                if let Some(idx) = CHAR_SELECTOR_ITEMS.iter().position(|&s| s == target_id) {
                    self.char_selector_idx = idx;
                }
                vec![]
            }
            _ => vec![],
        };

        serde_wasm_bindgen::to_value(&cmds).unwrap_or(JsValue::NULL)
    }

    fn handle_input(&mut self, value: &str) -> Vec<DomCmd> {
        if value == "/" {
            self.selector_open = true;
            self.selector_idx = 0;
            return vec![
                set_attr("chat-input", "value", ""),
                set_attr("selector", "hidden", ""),
                set_attr("selector", "inert", ""),  // removeAttribute → フォーカス可
                focus(SELECTOR_ITEMS[0]),
            ];
        }
        vec![]
    }

    fn handle_keydown(&mut self, key: &str) -> Vec<DomCmd> {
        if self.char_selector_open {
            let len = CHAR_SELECTOR_ITEMS.len();
            return match key {
                "ArrowDown" => {
                    self.char_selector_idx = (self.char_selector_idx + 1) % len;
                    vec![focus(CHAR_SELECTOR_ITEMS[self.char_selector_idx])]
                }
                "ArrowUp" => {
                    self.char_selector_idx = (self.char_selector_idx + len - 1) % len;
                    vec![focus(CHAR_SELECTOR_ITEMS[self.char_selector_idx])]
                }
                "Enter" => {
                    let k = CHAR_SELECTOR_ITEMS[self.char_selector_idx]
                        .strip_prefix("charroll-").unwrap_or("");
                    self.handle_char_selector(k)
                }
                "Escape" => self.close_char_selector(),
                _ => vec![],
            };
        }
        if !self.selector_open { return vec![]; }
        let len = SELECTOR_ITEMS.len();
        match key {
            "ArrowDown" => {
                self.selector_idx = (self.selector_idx + 1) % len;
                vec![focus(SELECTOR_ITEMS[self.selector_idx])]
            }
            "ArrowUp" => {
                self.selector_idx = (self.selector_idx + len - 1) % len;
                vec![focus(SELECTOR_ITEMS[self.selector_idx])]
            }
            "Enter" => {
                let roll_key = SELECTOR_ITEMS[self.selector_idx]
                    .strip_prefix("roll-").unwrap_or("");
                self.handle_roll_select(roll_key)
            }
            "Escape" => {
                self.close_selector()
            }
            _ => vec![],
        }
    }

    fn render_log(&self) -> String {
        self.roll_log.iter().map(|e| format!("{}\n", e)).collect()
    }

    fn push_log(&mut self, entry: RollLog) -> DomCmd {
        self.roll_log.push(entry);
        set_text("chat-log", &self.render_log())
    }

    fn handle_submit(&mut self, text: &str) -> Vec<DomCmd> {
        let trimmed = text.trim();
        if trimmed.is_empty() { return vec![]; }
        let cmd = self.push_log(RollLog::Message(trimmed.to_string()));
        vec![cmd, set_attr("chat-input", "value", "")]
    }

    fn handle_roll_select(&mut self, key: &str) -> Vec<DomCmd> {
        let roll = match key {
            "skill"       => Roll::SkillRoll,
            "char"        => {
                self.selector_open = false;
                self.selector_idx = 0;
                self.char_selector_open = true;
                self.char_selector_idx = 0;
                return vec![
                    set_attr("selector", "hidden", "true"),
                    set_attr("selector", "inert", "true"),
                    set_attr("char-selector", "hidden", ""),
                    set_attr("char-selector", "inert", ""),
                    focus(CHAR_SELECTOR_ITEMS[0]),
                ];
            }
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
        self.close_selector_into(roll)
    }

    fn handle_char_roll(&mut self) -> Vec<DomCmd> {
        if schema::characteristic::roll_all(&mut self.character).is_err() {
            return vec![];
        }
        let ch = &self.character;
        let mut cmds = vec![];
        let fields = [
            ("char-str", schema::strength::get(ch)),
            ("char-con", schema::constitution::get(ch)),
            ("char-siz", schema::size::get(ch)),
            ("char-dex", schema::dexterity::get(ch)),
            ("char-app", schema::appearance::get(ch)),
            ("char-int", schema::intelligence::get(ch)),
            ("char-pow", schema::power::get(ch)),
            ("char-edu", schema::education::get(ch)),
            ("char-luk", schema::luck::get(ch)),
        ];
        for (id, result) in fields {
            if let Ok(v) = result {
                cmds.push(set_attr(id, "value", &v.to_string()));
            }
        }
        for (id, result) in [
            ("char-hp",    schema::hit_points::get(ch)),
            ("char-mp",    schema::magic_points::get(ch)),
            ("char-san",   schema::sanity::get(ch)),
            ("char-dodge", schema::dodge::get(ch)),
        ] {
            if let Ok(v) = result {
                cmds.push(set_text(id, &v.to_string()));
            }
        }
        cmds
    }

    fn close_char_selector(&mut self) -> Vec<DomCmd> {
        self.char_selector_open = false;
        self.char_selector_idx = 0;
        vec![
            set_attr("char-selector", "hidden", "true"),
            set_attr("char-selector", "inert", "true"),
            focus("chat-input"),
        ]
    }

    fn handle_char_selector(&mut self, key: &str) -> Vec<DomCmd> {
        let (label, value) = match key {
            "str" => ("STR",  schema::strength::get(&self.character)),
            "con" => ("CON",  schema::constitution::get(&self.character)),
            "siz" => ("SIZ",  schema::size::get(&self.character)),
            "dex" => ("DEX",  schema::dexterity::get(&self.character)),
            "app" => ("APP",  schema::appearance::get(&self.character)),
            "int" => ("INT",  schema::intelligence::get(&self.character)),
            "pow" => ("POW",  schema::power::get(&self.character)),
            "edu" => ("EDU",  schema::education::get(&self.character)),
            "luk" => ("幸運", schema::luck::get(&self.character)),
            _ => return self.close_char_selector(),
        };
        let difficulty = match value {
            Ok(v) => v,
            Err(_) => {
                let log_cmd = self.push_log(RollLog::Message(format!("[能力値判定: {}] 未入力", label)));
                let mut cmds = self.close_char_selector();
                cmds.push(log_cmd);
                return cmds;
            }
        };
        let result = dice::skill_roll(0, Some(difficulty as u32), dice::DifficultySpec::None).unwrap();
        let entry = RollLog::Characteristic {
            label,
            difficulty,
            total: result.total,
            level: result.level,
        };
        let log_cmd = self.push_log(entry);
        let mut cmds = self.close_char_selector();
        cmds.push(log_cmd);
        cmds
    }

    fn handle_char_change(&mut self, target_id: &str, value: u16) -> Vec<DomCmd> {
        let ch = &mut self.character;
        match target_id {
            "char-str" => { let _ = schema::strength::set(ch, value); }
            "char-con" => { let _ = schema::constitution::set(ch, value); }
            "char-siz" => { let _ = schema::size::set(ch, value); }
            "char-dex" => { let _ = schema::dexterity::set(ch, value); }
            "char-app" => { let _ = schema::appearance::set(ch, value); }
            "char-int" => { let _ = schema::intelligence::set(ch, value); }
            "char-pow" => { let _ = schema::power::set(ch, value); }
            "char-edu" => { let _ = schema::education::set(ch, value); }
            "char-luk" => { let _ = schema::luck::set(ch, value); }
            _ => {}
        }
        // 導出値を再計算して返す
        let mut cmds = vec![];
        if let Ok(v) = schema::hit_points::derive(ch) {
            let _ = schema::hit_points::set(ch);
            cmds.push(set_text("char-hp", &v.to_string()));
        }
        if let Ok(v) = schema::magic_points::derive(ch) {
            let _ = schema::magic_points::set(ch);
            cmds.push(set_text("char-mp", &v.to_string()));
        }
        if let Ok(v) = schema::sanity::derive(ch) {
            let _ = schema::sanity::set(ch);
            cmds.push(set_text("char-san", &v.to_string()));
        }
        if let Ok(v) = schema::dodge::derive(ch) {
            let _ = schema::dodge::set(ch);
            cmds.push(set_text("char-dodge", &v.to_string()));
        }
        cmds
    }

    fn close_selector(&mut self) -> Vec<DomCmd> {
        self.selector_open = false;
        self.selector_idx = 0;
        vec![
            set_attr("selector", "hidden", "true"),
            set_attr("selector", "inert", "true"),
            focus("chat-input"),
        ]
    }

    fn close_selector_into(&mut self, roll: Roll) -> Vec<DomCmd> {
        self.selector_open = false;
        self.selector_idx = 0;
        let entry = make_roll_log(roll);
        let log_cmd = self.push_log(entry);
        vec![
            set_attr("selector", "hidden", "true"),
            set_attr("selector", "inert", "true"),
            log_cmd,
            focus("chat-input"),
        ]
    }
}

fn make_roll_log(roll: Roll) -> RollLog {
    match roll {
        Roll::BoutOfMadnessRealTime => {
            let r = dice::roll_madness_realtime();
            RollLog::Table { kind: r.roll_type.label(Lang::Ja), roll: r.roll, label: r.label }
        }
        Roll::BoutOfMadnessSummary => {
            let r = dice::roll_madness_summary();
            RollLog::Table { kind: r.roll_type.label(Lang::Ja), roll: r.roll, label: r.label }
        }
        Roll::FailedCastingMinor => {
            let r = dice::roll_failed_casting_minor();
            RollLog::Table { kind: r.roll_type.label(Lang::Ja), roll: r.roll, label: r.label }
        }
        Roll::FailedCastingMajor => {
            let r = dice::roll_failed_casting_major();
            RollLog::Table { kind: r.roll_type.label(Lang::Ja), roll: r.roll, label: r.label }
        }
        Roll::PhobiaTable => {
            let r = dice::roll_phobia();
            RollLog::Table { kind: r.roll_type.label(Lang::Ja), roll: r.roll, label: r.label }
        }
        Roll::ManiaTable => {
            let r = dice::roll_mania();
            RollLog::Table { kind: r.roll_type.label(Lang::Ja), roll: r.roll, label: r.label }
        }
        r => RollLog::Simple { kind: r.label(Lang::Ja) },
    }
}

// ============================================================
// ロール履歴
// ============================================================

enum RollLog {
    Characteristic { label: &'static str, difficulty: u16, total: u32, level: Option<ResultLevel> },
    Table          { kind: &'static str, roll: u32, label: &'static str },
    Simple         { kind: &'static str },
    Message        (String),
}

impl std::fmt::Display for RollLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Characteristic { label, difficulty, total, level } => {
                let result = match level {
                    Some(l) => l.label(Lang::Ja),
                    None    => "出目のみ",
                };
                write!(f, "[能力値判定: {}={}] 出目: {}  結果: {}", label, difficulty, total, result)
            }
            Self::Table { kind, roll, label } => {
                write!(f, "[{}] {} → {}", kind, roll, label)
            }
            Self::Simple { kind } => {
                write!(f, "[{}] (パラメータ入力UI未実装)", kind)
            }
            Self::Message(s) => f.write_str(s),
        }
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

fn focus(id: &str) -> DomCmd {
    DomCmd { op: OP_FOCUS, id: id.to_string(), attr: None, value: String::new() }
}
