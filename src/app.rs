use wasm_bindgen::prelude::*;
use crate::{Lang, dice::{self, ResultLevel}};
use crate::table::Roll;
use crate::character::{Instance, Model, schema};

// ============================================================
// DOM op codes
// ============================================================

const OP_SET_ATTR:    u32 = 0b0001;
const OP_SET_TEXT:    u32 = 0b0010;
const OP_FOCUS:       u32 = 0b0100;
const OP_SHOW_MODAL:  u32 = 0b1000;
const OP_CLOSE_MODAL: u32 = 0b1001;

// ============================================================
// JS → Rust 変換境界
// ============================================================

/// JSから届く event_type 数値
const EV_CLICK:   u32 = 0b001;
const EV_SUBMIT:  u32 = 0b010;
const EV_INPUT:   u32 = 0b011;
const EV_KEYDOWN: u32 = 0b100;
const EV_FOCUS:   u32 = 0b110;

#[derive(Debug)]
enum Key { Up, Down, Enter, Escape, Other }

impl Key {
    fn parse(s: &str) -> Self {
        match s {
            "ArrowUp"   => Self::Up,
            "ArrowDown" => Self::Down,
            "Enter"     => Self::Enter,
            "Escape"    => Self::Escape,
            _           => Self::Other,
        }
    }
}

/// JSの target_id 文字列から変換される、アプリが扱う全クリック対象
#[derive(Debug)]
enum ClickTarget {
    // ロール種セレクタ
    SelectorOverlay,
    RollItem(Roll),
    // 能力値判定セレクタ
    CharSelectorOverlay,
    CharRollItem(Model),
    // 技能セレクタ
    SkillSelectorOverlay,
    SkillRollItem(Model),
    // ダイス入力
    DiceInputOverlay,
    DiceUp,
    DiceDown,
    DiceNext,
    // キャラクター編集
    CharEditOpen,
    CharEditCancel,
    CharRoll,
    CharEditRoll(Model),
    // 不明
    Unknown,
}

impl ClickTarget {
    fn parse(id: &str) -> Self {
        match id {
            "selector_overlay"       => Self::SelectorOverlay,
            "char_selector_overlay"  => Self::CharSelectorOverlay,
            "skill_selector_overlay" => Self::SkillSelectorOverlay,
            "dice_input_overlay"     => Self::DiceInputOverlay,
            "dice_up"                => Self::DiceUp,
            "dice_down"              => Self::DiceDown,
            "dice_next"              => Self::DiceNext,
            "char_edit_open"         => Self::CharEditOpen,
            "char_edit_cancel"       => Self::CharEditCancel,
            "char_roll"              => Self::CharRoll,
            _ if id.starts_with("roll_") => {
                let key = id.strip_prefix("roll_").unwrap_or("");
                match Roll::all().iter().find(|r| r.dom_id() == key) {
                    Some(&roll) => Self::RollItem(roll),
                    None        => Self::Unknown,
                }
            }
            _ if id.starts_with("char_edit_roll_") => {
                let key = id.strip_prefix("char_edit_roll_").unwrap_or("");
                match schema::attribute(schema::Attribute::Characteristic).iter().find(|m| m.dom_id() == key) {
                    Some(&field) => Self::CharEditRoll(field),
                    None         => Self::Unknown,
                }
            }
            _ if id.starts_with("char_roll_") => {
                let key = id.strip_prefix("char_roll_").unwrap_or("");
                match schema::attribute(schema::Attribute::Characteristic).iter().find(|m| m.dom_id() == key) {
                    Some(&field) => Self::CharRollItem(field),
                    None         => Self::Unknown,
                }
            }
            _ if id.starts_with("skill_roll_") => {
                let key = id.strip_prefix("skill_roll_").unwrap_or("");
                match schema::attribute(schema::Attribute::Skill).iter().find(|m| m.dom_id() == key) {
                    Some(&field) => Self::SkillRollItem(field),
                    None         => Self::Unknown,
                }
            }
            _ => Self::Unknown,
        }
    }
}

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
// 定数テーブル
// ============================================================


// ============================================================
// JS helpers
// ============================================================

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

// ============================================================
// App
// ============================================================

#[wasm_bindgen]
pub struct App {
    state:     State,
    roll_log:  Vec<RollLog>,
    character: Instance,
}

#[wasm_bindgen]
impl App {
    pub fn init() -> App {
        App {
            state:     State::Idle,
            roll_log:  Vec::new(),
            character: Instance::new(),
        }
    }

    pub fn event(&mut self, payload: JsValue) -> JsValue {
        let ev_type   = js_get_u32(&payload, "event_type");
        let target_id = js_get_str(&payload, "target_id");
        let key_str   = js_get_str(&payload, "key");

        let cmds: Vec<DomCmd> = match ev_type {
            EV_SUBMIT if target_id == "chat_form" => {
                let fields = js_get_field(&payload, "fields");
                let text = js_get_str(&fields, "text");
                self.on_chat_submit(&text)
            }
            EV_SUBMIT if target_id == "char_edit_form" => {
                let fields = js_get_field(&payload, "fields");
                self.on_char_edit_save(&fields)
            }
            EV_INPUT if target_id == "chat_input" => {
                let value = js_get_str(&payload, "value");
                self.on_chat_input(&value)
            }
            EV_KEYDOWN => {
                self.on_keydown(Key::parse(&key_str))
            }
            EV_CLICK => {
                self.on_click(ClickTarget::parse(&target_id))
            }
            EV_FOCUS => {
                self.on_focus(&target_id);
                vec![]
            }
            _ => vec![],
        };

        serde_wasm_bindgen::to_value(&cmds).unwrap_or(JsValue::NULL)
    }

    // ----------------------------------------------------------
    // input / submit
    // ----------------------------------------------------------

    fn on_chat_submit(&mut self, text: &str) -> Vec<DomCmd> {
        let trimmed = text.trim();
        if trimmed.is_empty() { return vec![]; }
        let cmd = self.push_log(RollLog::Message(trimmed.to_string()));
        vec![cmd, set_attr("chat_input", "value", "")]
    }

    fn on_chat_input(&mut self, value: &str) -> Vec<DomCmd> {
        if value != "/" { return vec![]; }
        self.state = State::Selector { idx: 0 };
        let first_id = format!("roll_{}", Roll::all()[0].dom_id());
        vec![
            set_attr("chat_input", "value", ""),
            set_attr("selector", "hidden", ""),
            set_attr("selector", "inert", ""),
            focus(&first_id),
        ]
    }

    // ----------------------------------------------------------
    // keydown — state別に分岐
    // ----------------------------------------------------------

    fn on_keydown(&mut self, key: Key) -> Vec<DomCmd> {
        match &self.state {
            State::DiceInput { .. }     => self.dice_keydown(key),
            State::SkillSelector { .. } => self.skill_selector_keydown(key),
            State::CharSelector { .. }  => self.char_selector_keydown(key),
            State::Selector { .. }      => self.selector_keydown(key),
            State::Idle                 => vec![],
        }
    }

    fn selector_keydown(&mut self, key: Key) -> Vec<DomCmd> {
        let State::Selector { idx } = self.state else { return vec![]; };
        let all = Roll::all();
        let len = all.len();
        match key {
            Key::Down  => { self.state = State::Selector { idx: (idx + 1) % len };
                            vec![focus(&format!("roll_{}", all[idx_of(&self.state)].dom_id()))] }
            Key::Up    => { self.state = State::Selector { idx: (idx + len - 1) % len };
                            vec![focus(&format!("roll_{}", all[idx_of(&self.state)].dom_id()))] }
            Key::Enter => self.on_roll_select(all[idx]),
            Key::Escape => self.close_selector(),
            Key::Other  => vec![],
        }
    }

    fn char_selector_keydown(&mut self, key: Key) -> Vec<DomCmd> {
        let State::CharSelector { idx } = self.state else { return vec![]; };
        let chars = schema::attribute(schema::Attribute::Characteristic);
        let len = chars.len();
        match key {
            Key::Down  => { self.state = State::CharSelector { idx: (idx + 1) % len };
                            vec![focus(&format!("char_roll_{}", chars[idx_of(&self.state)].dom_id()))] }
            Key::Up    => { self.state = State::CharSelector { idx: (idx + len - 1) % len };
                            vec![focus(&format!("char_roll_{}", chars[idx_of(&self.state)].dom_id()))] }
            Key::Enter => self.do_char_roll(chars[idx]),
            Key::Escape => self.close_char_selector(),
            Key::Other  => vec![],
        }
    }

    fn skill_selector_keydown(&mut self, key: Key) -> Vec<DomCmd> {
        let State::SkillSelector { mode, idx } = self.state else { return vec![]; };
        let candidates = self.skill_candidates(mode);
        let len = candidates.len();
        if len == 0 { return self.close_skill_selector(); }
        match key {
            Key::Down  => { self.state = State::SkillSelector { mode, idx: (idx + 1) % len };
                            vec![focus(&candidates[idx_of(&self.state)])] }
            Key::Up    => { self.state = State::SkillSelector { mode, idx: (idx + len - 1) % len };
                            vec![focus(&candidates[idx_of(&self.state)])] }
            Key::Enter => { let id = candidates[idx].clone();
                            let key = id.strip_prefix("skill_roll_").unwrap_or("");
                            let field = schema::attribute(schema::Attribute::Skill).iter().find(|m| m.dom_id() == key).copied();
                            if let Some(f) = field { self.do_skill_action(mode, f) }
                            else { self.close_skill_selector() } }
            Key::Escape => self.close_skill_selector(),
            Key::Other  => vec![],
        }
    }

    fn dice_keydown(&mut self, key: Key) -> Vec<DomCmd> {
        match key {
            Key::Escape => self.close_dice_input(),
            Key::Enter  => self.dice_advance(),
            Key::Up     => self.dice_increment(true),
            Key::Down   => self.dice_increment(false),
            Key::Other  => vec![],
        }
    }

    // ----------------------------------------------------------
    // click — ClickTarget に変換済み
    // ----------------------------------------------------------

    fn on_click(&mut self, target: ClickTarget) -> Vec<DomCmd> {
        match target {
            ClickTarget::SelectorOverlay       => self.close_selector(),
            ClickTarget::CharSelectorOverlay   => self.close_char_selector(),
            ClickTarget::SkillSelectorOverlay  => self.close_skill_selector(),
            ClickTarget::DiceInputOverlay      => self.close_dice_input(),
            ClickTarget::DiceUp                => self.dice_increment(true),
            ClickTarget::DiceDown              => self.dice_increment(false),
            ClickTarget::DiceNext              => self.dice_advance(),
            ClickTarget::RollItem(roll)        => self.on_roll_select(roll),
            ClickTarget::CharRollItem(field)   => self.do_char_roll(field),
            ClickTarget::SkillRollItem(field)  => {
                let mode = if let State::SkillSelector { mode, .. } = self.state { mode }
                           else { return self.close_skill_selector(); };
                self.do_skill_action(mode, field)
            }
            ClickTarget::CharEditOpen          => self.open_char_edit(),
            ClickTarget::CharEditCancel        => vec![close_modal("char_edit")],
            ClickTarget::CharRoll              => self.on_char_roll_all(),
            ClickTarget::CharEditRoll(field)   => self.on_char_edit_roll(field),
            ClickTarget::Unknown               => vec![],
        }
    }

    fn on_focus(&mut self, target_id: &str) {
        if let State::Selector { ref mut idx } = self.state {
            let all = Roll::all();
            if let Some(i) = all.iter().position(|r| format!("roll_{}", r.dom_id()) == target_id) {
                *idx = i;
            }
        } else if let State::CharSelector { ref mut idx } = self.state {
            let chars = schema::attribute(schema::Attribute::Characteristic);
            if let Some(i) = chars.iter().position(|m| format!("char_roll_{}", m.dom_id()) == target_id) {
                *idx = i;
            }
        } else if let State::SkillSelector { mode, .. } = self.state {
            let candidates = self.skill_candidates(mode);
            if let Some(i) = candidates.iter().position(|s| s == target_id) {
                if let State::SkillSelector { ref mut idx, .. } = self.state {
                    *idx = i;
                }
            }
        }
    }

    // ----------------------------------------------------------
    // ロール種セレクタ
    // ----------------------------------------------------------

    fn on_roll_select(&mut self, roll: Roll) -> Vec<DomCmd> {
        match roll {
            Roll::DiceRoll => {
                self.state = State::DiceInput { phase: DicePhase::Count, count: 1, sides_idx: 4, modifier: 0 };
                let mut cmds = vec![
                    set_attr("selector", "hidden", "true"),
                    set_attr("selector", "inert", "true"),
                ];
                cmds.extend(self.render_dice_input());
                cmds
            }
            Roll::SkillRoll => {
                self.open_skill_selector(SkillSelectorMode::Roll, "技能判定")
            }
            Roll::CharacteristicRoll => {
                let first_id = format!("char_roll_{}", schema::attribute(schema::Attribute::Characteristic)[0].dom_id());
                self.state = State::CharSelector { idx: 0 };
                vec![
                    set_attr("selector", "hidden", "true"),
                    set_attr("selector", "inert", "true"),
                    set_attr("char_selector", "hidden", ""),
                    set_attr("char_selector", "inert", ""),
                    focus(&first_id),
                ]
            }
            Roll::PushedRoll      => self.open_skill_selector(SkillSelectorMode::Push, "プッシュロール"),
            Roll::DevelopmentCheck => self.open_skill_selector(SkillSelectorMode::DevCheck, "上達チェック"),
            roll => {
                self.state = State::Idle;
                let entry = make_roll_log(roll);
                let log_cmd = self.push_log(entry);
                vec![
                    set_attr("selector", "hidden", "true"),
                    set_attr("selector", "inert", "true"),
                    log_cmd,
                    focus("chat_input"),
                ]
            }
        }
    }

    // ----------------------------------------------------------
    // 能力値判定セレクタ
    // ----------------------------------------------------------

    fn do_char_roll(&mut self, field: Model) -> Vec<DomCmd> {
        let label = schema::label(field, Lang::Ja);
        let difficulty = match schema::get(&self.character, field) {
            Ok(v)  => v,
            Err(_) => {
                let log_cmd = self.push_log(RollLog::Message(format!("[能力値判定: {}] 未入力", label)));
                let mut cmds = self.close_char_selector();
                cmds.push(log_cmd);
                return cmds;
            }
        };
        let result = dice::skill_roll(0, Some(difficulty as u32), dice::DifficultySpec::None).unwrap();
        let entry = RollLog::Characteristic { label, difficulty, total: result.total, level: result.level };
        let log_cmd = self.push_log(entry);
        let mut cmds = self.close_char_selector();
        cmds.push(log_cmd);
        cmds
    }

    // ----------------------------------------------------------
    // 技能セレクタ
    // ----------------------------------------------------------

    fn open_skill_selector(&mut self, mode: SkillSelectorMode, title: &str) -> Vec<DomCmd> {
        let candidates = self.skill_candidates(mode);
        if candidates.is_empty() {
            let msg = match mode {
                SkillSelectorMode::Roll     => "技能が未登録です",
                SkillSelectorMode::Push     => "プッシュ可能なロールがありません",
                SkillSelectorMode::DevCheck => "上達チェック対象の技能がありません",
            };
            let log_cmd = self.push_log(RollLog::Message(msg.to_string()));
            let mut cmds = self.close_selector();
            cmds.push(log_cmd);
            return cmds;
        }
        self.state = State::SkillSelector { mode, idx: 0 };
        let mut cmds = vec![
            set_attr("selector", "hidden", "true"),
            set_attr("selector", "inert", "true"),
            set_text("skill_selector_title", title),
            set_attr("skill_selector", "hidden", ""),
            set_attr("skill_selector", "inert", ""),
        ];
        for &field in schema::attribute(schema::Attribute::Skill) {
            let id = format!("skill_roll_{}", field.dom_id());
            let visible = candidates.iter().any(|c| c == &id);
            cmds.push(set_attr(&id, "hidden", if visible { "" } else { "true" }));
            cmds.push(set_attr(&id, "inert",  if visible { "" } else { "true" }));
        }
        if !candidates.is_empty() { cmds.push(focus(&candidates[0])); }
        cmds
    }

    fn do_skill_action(&mut self, mode: SkillSelectorMode, field: Model) -> Vec<DomCmd> {
        match mode {
            SkillSelectorMode::Roll     => self.do_skill_roll(field, false),
            SkillSelectorMode::Push     => {
                for entry in self.roll_log.iter_mut().rev() {
                    if let RollLog::Skill { field: f, pushed, .. } = entry {
                        if *f == field && !*pushed { *pushed = true; break; }
                    }
                }
                self.do_skill_roll(field, true)
            }
            SkillSelectorMode::DevCheck => self.do_dev_check(field),
        }
    }

    fn do_skill_roll(&mut self, field: Model, pushed: bool) -> Vec<DomCmd> {
        let difficulty = match schema::skill::get(&self.character, field) {
            Ok(v)  => v,
            Err(_) => return self.close_skill_selector(),
        };
        let label = schema::label(field, Lang::Ja);
        let result = dice::skill_roll(0, Some(difficulty as u32), dice::DifficultySpec::None).unwrap();
        let entry = RollLog::Skill { field, label, difficulty, total: result.total, level: result.level, pushed };
        let log_cmd = self.push_log(entry);
        let mut cmds = self.close_skill_selector();
        cmds.push(log_cmd);
        cmds
    }

    fn do_dev_check(&mut self, field: Model) -> Vec<DomCmd> {
        let current = match schema::skill::get(&self.character, field) {
            Ok(v)  => v,
            Err(_) => return self.close_skill_selector(),
        };
        let label = schema::label(field, Lang::Ja);
        let roll = crate::n_d_n(1, 100);
        let mut cmds = self.close_skill_selector();
        if roll > current as u32 {
            let gain = crate::n_d_n(1, 10) as u16;
            let new_val = current.saturating_add(gain);
            let _ = schema::skill::set(&mut self.character, field, new_val);
            let msg = format!("[上達チェック: {}] 出目: {} > {} → 成功! +{} → {}", label, roll, current, gain, new_val);
            cmds.push(self.push_log(RollLog::Message(msg)));
            cmds.push(set_text(&format!("skill_val_{}", field.dom_id()), &new_val.to_string()));
        } else {
            let msg = format!("[上達チェック: {}] 出目: {} ≤ {} → 失敗", label, roll, current);
            cmds.push(self.push_log(RollLog::Message(msg)));
        }
        cmds
    }

    fn skill_candidates(&self, mode: SkillSelectorMode) -> Vec<String> {
        let skills = schema::attribute(schema::Attribute::Skill);
        match mode {
            SkillSelectorMode::Roll => {
                skills.iter()
                    .filter(|&&f| schema::skill::get(&self.character, f).is_ok())
                    .map(|f| format!("skill_roll_{}", f.dom_id()))
                    .collect()
            }
            SkillSelectorMode::Push => {
                self.roll_log.iter().rev()
                    .find_map(|entry| {
                        if let RollLog::Skill { field, level, pushed: false, .. } = entry {
                            let is_failure = matches!(level,
                                Some(ResultLevel::Failure) | Some(ResultLevel::Fumble) | None);
                            if is_failure {
                                return Some(vec![format!("skill_roll_{}", field.dom_id())]);
                            }
                        }
                        None
                    })
                    .unwrap_or_default()
            }
            SkillSelectorMode::DevCheck => {
                let mut eligible: Vec<Model> = Vec::new();
                for entry in &self.roll_log {
                    if let RollLog::Skill { field, level, pushed: false, .. } = entry {
                        if matches!(level,
                            Some(ResultLevel::Regular) | Some(ResultLevel::Hard) |
                            Some(ResultLevel::Extreme) | Some(ResultLevel::Critical))
                            && !eligible.contains(field)
                        {
                            eligible.push(*field);
                        }
                    }
                }
                skills.iter()
                    .filter(|f| eligible.contains(f))
                    .map(|f| format!("skill_roll_{}", f.dom_id()))
                    .collect()
            }
        }
    }

    // ----------------------------------------------------------
    // ダイス入力
    // ----------------------------------------------------------

    fn dice_advance(&mut self) -> Vec<DomCmd> {
        let State::DiceInput { phase, count, sides_idx, modifier } = self.state
            else { return vec![]; };
        match phase {
            DicePhase::Count    => { self.state = State::DiceInput { phase: DicePhase::Sides, count, sides_idx, modifier }; self.render_dice_input() }
            DicePhase::Sides    => { self.state = State::DiceInput { phase: DicePhase::Modifier, count, sides_idx, modifier }; self.render_dice_input() }
            DicePhase::Modifier => self.execute_dice_roll(),
        }
    }

    fn dice_increment(&mut self, up: bool) -> Vec<DomCmd> {
        let State::DiceInput { phase, count, sides_idx, modifier } = self.state
            else { return vec![]; };
        let len = DICE_SIDES.len();
        self.state = State::DiceInput {
            phase,
            count:     if phase == DicePhase::Count    { if up { count.saturating_add(1).min(99) } else { count.saturating_sub(1).max(1) } } else { count },
            sides_idx: if phase == DicePhase::Sides    { if up { (sides_idx + 1) % len } else { (sides_idx + len - 1) % len } } else { sides_idx },
            modifier:  if phase == DicePhase::Modifier { if up { modifier.saturating_add(1) } else { modifier.saturating_sub(1) } } else { modifier },
        };
        self.render_dice_input()
    }

    fn render_dice_input(&self) -> Vec<DomCmd> {
        let State::DiceInput { phase, count, sides_idx, modifier } = self.state
            else { return vec![]; };
        let sides = DICE_SIDES[sides_idx];
        let modifier_str = match modifier.cmp(&0) {
            std::cmp::Ordering::Greater => format!("+{}", modifier),
            std::cmp::Ordering::Less    => modifier.to_string(),
            std::cmp::Ordering::Equal   => "0".to_string(),
        };
        let next_label = if phase == DicePhase::Modifier { "ロール" } else { "次へ" };
        let hint = match phase {
            DicePhase::Count    => format!("個数: {}", count),
            DicePhase::Sides    => format!("{}個 × {}面", count, sides),
            DicePhase::Modifier => {
                let mod_part = if modifier != 0 { format!(" {}", modifier_str) } else { String::new() };
                format!("{}個 × {}面{}", count, sides, mod_part)
            }
        };
        let (h_count, h_sides, h_mod) = match phase {
            DicePhase::Count    => ("", "true", "true"),
            DicePhase::Sides    => ("true", "", "true"),
            DicePhase::Modifier => ("true", "true", ""),
        };
        vec![
            set_attr("dice_input", "hidden", ""),
            set_attr("dice_input", "inert", ""),
            set_attr("dice_count_row",    "hidden", h_count),
            set_attr("dice_sides_row",    "hidden", h_sides),
            set_attr("dice_modifier_row", "hidden", h_mod),
            set_text("dice_count_val",    &count.to_string()),
            set_text("dice_sides_val",    &format!("{}面", sides)),
            set_text("dice_modifier_val", &modifier_str),
            set_text("dice_hint",         &hint),
            set_text("dice_next",         next_label),
            focus("dice_input_focus"),
        ]
    }

    fn execute_dice_roll(&mut self) -> Vec<DomCmd> {
        let State::DiceInput { count, sides_idx, modifier, .. } = self.state
            else { return vec![]; };
        let sides = DICE_SIDES[sides_idx];
        let raw   = crate::n_d_n(count, sides);
        let total = (raw as i32 + modifier).max(0) as u32;
        let modifier_str = match modifier.cmp(&0) {
            std::cmp::Ordering::Greater => format!("+{}", modifier),
            std::cmp::Ordering::Less    => modifier.to_string(),
            std::cmp::Ordering::Equal   => String::new(),
        };
        let expr = format!("{}d{}{}", count, sides, modifier_str);
        let msg  = format!("[ダイスロール: {}] 出目: {} → 合計: {}", expr, raw, total);
        let log_cmd = self.push_log(RollLog::Message(msg));
        let mut cmds = self.close_dice_input();
        cmds.push(log_cmd);
        cmds
    }


    // ----------------------------------------------------------
    // キャラクター編集
    // ----------------------------------------------------------

    fn open_char_edit(&self) -> Vec<DomCmd> {
        let ch = &self.character;
        let mut cmds = vec![show_modal("char_edit")];
        for &field in schema::attribute(schema::Attribute::Characteristic) {
            if let Ok(v) = schema::get(ch, field) {
                cmds.push(set_attr(&format!("edit_{}", field.dom_id()), "value", &v.to_string()));
            }
        }
        for &field in schema::attribute(schema::Attribute::Skill) {
            let occ_id = format!("skill_occ_{}", field.dom_id());
            let int_id = format!("skill_int_{}", field.dom_id());
            if let Ok(v) = schema::skill::get(ch, field) {
                cmds.push(set_attr(&occ_id, "value", &v.to_string()));
            } else {
                cmds.push(set_attr(&occ_id, "value", ""));
            }
            cmds.push(set_attr(&int_id, "value", ""));
        }
        cmds
    }

    fn on_char_roll_all(&mut self) -> Vec<DomCmd> {
        if schema::roll_characteristics(&mut self.character).is_err() { return vec![]; }
        self.stat_view_cmds()
    }

    fn on_char_edit_roll(&mut self, field: Model) -> Vec<DomCmd> {
        let v = schema::roll_characteristic(field);
        let _ = schema::set(&mut self.character, field, v);
        let _ = schema::update(&mut self.character);
        let mut cmds = vec![set_attr(&format!("edit_{}", field.dom_id()), "value", &v.to_string())];
        cmds.extend(self.stat_view_cmds());
        cmds
    }

    fn on_char_edit_save(&mut self, fields: &JsValue) -> Vec<DomCmd> {
        for &field in schema::attribute(schema::Attribute::Characteristic) {
            let s = js_get_str(fields, &format!("stat_{}", field.dom_id()));
            if !s.is_empty() {
                let v: u16 = s.trim().parse().unwrap_or(0);
                let _ = schema::set(&mut self.character, field, v);
            }
        }
        for &field in schema::attribute(schema::Attribute::Skill) {
            let occ: u16 = js_get_str(fields, &format!("occ_{}", field.dom_id())).trim().parse().unwrap_or(0);
            let int: u16 = js_get_str(fields, &format!("int_{}", field.dom_id())).trim().parse().unwrap_or(0);
            if occ > 0 || int > 0 {
                let base  = schema::skill::base_value(field);
                let total = base.saturating_add(occ).saturating_add(int);
                let _ = schema::skill::set(&mut self.character, field, total);
            }
        }
        let _ = schema::update(&mut self.character);
        let mut cmds = vec![close_modal("char_edit")];
        cmds.extend(self.stat_view_cmds());
        cmds
    }

    fn stat_view_cmds(&self) -> Vec<DomCmd> {
        let ch = &self.character;
        let mut cmds = vec![];
        for &field in schema::attribute(schema::Attribute::Characteristic) {
            if let Ok(v) = schema::get(ch, field) {
                cmds.push(set_attr(&format!("char_view_{}", field.dom_id()), "hidden", ""));
                cmds.push(set_text(&format!("char_val_{}", field.dom_id()), &v.to_string()));
                cmds.push(set_attr(&format!("edit_{}", field.dom_id()), "value", &v.to_string()));
            }
        }
        for &field in schema::attribute(schema::Attribute::Derived) {
            if let Ok(v) = schema::get(ch, field) {
                cmds.push(set_attr(&format!("char_view_{}", field.dom_id()), "hidden", ""));
                cmds.push(set_text(&format!("char_val_{}", field.dom_id()), &v.to_string()));
            }
        }
        for &field in schema::attribute(schema::Attribute::Skill) {
            if let Ok(v) = schema::skill::get(ch, field) {
                cmds.push(set_attr(&format!("skill_view_{}", field.dom_id()), "hidden", ""));
                cmds.push(set_text(&format!("skill_val_{}", field.dom_id()), &v.to_string()));
            }
        }
        cmds
    }

    // ----------------------------------------------------------
    // close helpers
    // ----------------------------------------------------------

    fn close_selector(&mut self) -> Vec<DomCmd> {
        self.state = State::Idle;
        vec![
            set_attr("selector", "hidden", "true"),
            set_attr("selector", "inert", "true"),
            focus("chat_input"),
        ]
    }

    fn close_char_selector(&mut self) -> Vec<DomCmd> {
        self.state = State::Idle;
        vec![
            set_attr("char_selector", "hidden", "true"),
            set_attr("char_selector", "inert", "true"),
            focus("chat_input"),
        ]
    }

    fn close_skill_selector(&mut self) -> Vec<DomCmd> {
        self.state = State::Idle;
        vec![
            set_attr("skill_selector", "hidden", "true"),
            set_attr("skill_selector", "inert", "true"),
            focus("chat_input"),
        ]
    }

    fn close_dice_input(&mut self) -> Vec<DomCmd> {
        self.state = State::Idle;
        vec![
            set_attr("dice_input", "hidden", "true"),
            set_attr("dice_input", "inert", "true"),
            focus("chat_input"),
        ]
    }

    // ----------------------------------------------------------
    // log
    // ----------------------------------------------------------

    fn push_log(&mut self, entry: RollLog) -> DomCmd {
        self.roll_log.push(entry);
        let text: String = self.roll_log.iter().map(|e| format!("{}\n", e)).collect();
        set_text("chat_log", &text)
    }
}

// ============================================================
// State内idxを取り出すユーティリティ
// ============================================================

fn idx_of(state: &State) -> usize {
    match state {
        State::Selector      { idx } => *idx,
        State::CharSelector  { idx } => *idx,
        State::SkillSelector { idx, .. } => *idx,
        _ => 0,
    }
}

// ============================================================
// ロール表ヘルパー
// ============================================================

fn make_roll_log(roll: Roll) -> RollLog {
    match roll {
        Roll::BoutOfMadnessRealTime => { let r = dice::roll_madness_realtime();    RollLog::Table { kind: r.roll_type.label(Lang::Ja), roll: r.roll, label: r.label } }
        Roll::BoutOfMadnessSummary  => { let r = dice::roll_madness_summary();     RollLog::Table { kind: r.roll_type.label(Lang::Ja), roll: r.roll, label: r.label } }
        Roll::FailedCastingMinor    => { let r = dice::roll_failed_casting_minor(); RollLog::Table { kind: r.roll_type.label(Lang::Ja), roll: r.roll, label: r.label } }
        Roll::FailedCastingMajor    => { let r = dice::roll_failed_casting_major(); RollLog::Table { kind: r.roll_type.label(Lang::Ja), roll: r.roll, label: r.label } }
        Roll::PhobiaTable           => { let r = dice::roll_phobia();               RollLog::Table { kind: r.roll_type.label(Lang::Ja), roll: r.roll, label: r.label } }
        Roll::ManiaTable            => { let r = dice::roll_mania();                RollLog::Table { kind: r.roll_type.label(Lang::Ja), roll: r.roll, label: r.label } }
        r => RollLog::Simple { kind: r.label(Lang::Ja) },
    }
}

// ============================================================
// ロール履歴
// ============================================================

enum RollLog {
    Skill          { field: Model, label: &'static str, difficulty: u16, total: u32, level: Option<ResultLevel>, pushed: bool },
    Characteristic { label: &'static str, difficulty: u16, total: u32, level: Option<ResultLevel> },
    Table          { kind: &'static str, roll: u32, label: &'static str },
    Simple         { kind: &'static str },
    Message        (String),
}

impl std::fmt::Display for RollLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Skill { label, difficulty, total, level, pushed, .. } => {
                let kind   = if *pushed { "プッシュロール" } else { "技能判定" };
                let result = level.map_or("出目のみ", |l| l.label(Lang::Ja));
                write!(f, "[{}: {}={}] 出目: {}  結果: {}", kind, label, difficulty, total, result)
            }
            Self::Characteristic { label, difficulty, total, level } => {
                let result = level.map_or("出目のみ", |l| l.label(Lang::Ja));
                write!(f, "[能力値判定: {}={}] 出目: {}  結果: {}", label, difficulty, total, result)
            }
            Self::Table { kind, roll, label } => write!(f, "[{}] {} → {}", kind, roll, label),
            Self::Simple { kind }             => write!(f, "[{}] (パラメータ入力UI未実装)", kind),
            Self::Message(s)                  => f.write_str(s),
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

fn show_modal(id: &str) -> DomCmd {
    DomCmd { op: OP_SHOW_MODAL, id: id.to_string(), attr: None, value: String::new() }
}

fn close_modal(id: &str) -> DomCmd {
    DomCmd { op: OP_CLOSE_MODAL, id: id.to_string(), attr: None, value: String::new() }
}
