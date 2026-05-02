use wasm_bindgen::prelude::*;
use crate::{Lang, dice::{self, ResultLevel}};
use crate::table::Roll;
use crate::character::{Instance, Model, schema};

const OP_SET_ATTR: u32 = 0b001;
const OP_SET_TEXT: u32 = 0b010;
const OP_FOCUS:    u32 = 0b100;

const EVENT_CLICK:    u32 = 0b001;
const EVENT_SUBMIT:   u32 = 0b010;
const EVENT_INPUT:    u32 = 0b011;
const EVENT_KEYDOWN:  u32 = 0b100;
const EVENT_FOCUS:    u32 = 0b110;

const SELECTOR_ITEMS: &[&str] = &[
    "roll-dice",
    "roll-skill",
    "roll-char",
    "roll-sanity",
    "roll-madness-rt",
    "roll-madness-sum",
    "roll-combine",
    "roll-pushed",
    "roll-dev-check",
    "roll-autofire",
    "roll-cast-minor",
    "roll-cast-major",
    "roll-phobia",
    "roll-mania",
];

const DICE_SIDES: &[u32] = &[2, 3, 4, 5, 6, 8, 10, 12, 20, 100];

#[derive(Clone, Copy, PartialEq)]
enum DicePhase { Count, Sides, Modifier }

#[derive(Clone)]
struct DiceInput {
    phase: DicePhase,
    count: u32,
    sides_idx: usize,
    modifier: i32,
}

const SKILL_ID_MAP: &[(&str, Model)] = &[
    ("accounting",            Model::Accounting),
    ("anthropology",          Model::Anthropology),
    ("archaeology",           Model::Archaeology),
    ("appraise",              Model::Appraise),
    ("art-craft",             Model::ArtCraft),
    ("charm",                 Model::Charm),
    ("climb",                 Model::Climb),
    ("computer-use",          Model::ComputerUse),
    ("credit-rating",         Model::CreditRating),
    ("cthulhu-mythos",        Model::CthulhuMythos),
    ("disguise",              Model::Disguise),
    ("drive-auto",            Model::DriveAuto),
    ("elec-repair",           Model::ElecRepair),
    ("electronics",           Model::Electronics),
    ("fast-talk",             Model::FastTalk),
    ("fighting-brawl",        Model::FightingBrawl),
    ("fighting-other",        Model::FightingOther),
    ("firearms-handgun",      Model::FirearmsHandgun),
    ("firearms-rifle-shotgun",Model::FirearmsRifleShotgun),
    ("firearms-other",        Model::FirearmsOther),
    ("first-aid",             Model::FirstAid),
    ("history",               Model::History),
    ("intimidate",            Model::Intimidate),
    ("jump",                  Model::Jump),
    ("language-other",        Model::LanguageOther),
    ("language-own",          Model::LanguageOwn),
    ("law",                   Model::Law),
    ("library-use",           Model::LibraryUse),
    ("listen",                Model::Listen),
    ("locksmith",             Model::Locksmith),
    ("mech-repair",           Model::MechRepair),
    ("medicine",              Model::Medicine),
    ("natural-world",         Model::NaturalWorld),
    ("navigate",              Model::Navigate),
    ("occult",                Model::Occult),
    ("persuade",              Model::Persuade),
    ("pilot",                 Model::Pilot),
    ("psychoanalysis",        Model::Psychoanalysis),
    ("psychology",            Model::Psychology),
    ("ride",                  Model::Ride),
    ("science",               Model::Science),
    ("sleight-of-hand",       Model::SleightOfHand),
    ("spot-hidden",           Model::SpotHidden),
    ("stealth",               Model::Stealth),
    ("survival",              Model::Survival),
    ("swim",                  Model::Swim),
    ("throw",                 Model::Throw),
    ("track",                 Model::Track),
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

#[derive(Clone, Copy, PartialEq)]
enum SkillSelectorMode { Roll, Push, DevCheck }

#[wasm_bindgen]
pub struct App {
    selector_open: bool,
    selector_idx: usize,
    char_selector_open: bool,
    char_selector_idx: usize,
    skill_selector_mode: Option<SkillSelectorMode>,
    skill_selector_idx: usize,
    dice_input: Option<DiceInput>,
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
            skill_selector_mode: None,
            skill_selector_idx: 0,
            dice_input: None,
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
            EVENT_CLICK if target_id == "dice-input-overlay" => {
                self.close_dice_input()
            }
            EVENT_CLICK if target_id.starts_with("charroll-") => {
                let key = target_id.strip_prefix("charroll-").unwrap_or("");
                self.handle_char_selector(key)
            }
            EVENT_CLICK if target_id == "skill-selector-overlay" => {
                self.close_skill_selector()
            }
            EVENT_CLICK if target_id.starts_with("skillroll-") => {
                let key = target_id.strip_prefix("skillroll-").unwrap_or("");
                self.handle_skill_selector(key)
            }
            EVENT_CLICK if target_id == "char-roll" => {
                self.handle_char_roll()
            }
            EVENT_SUBMIT if target_id == "char-edit-form" => {
                let fields = js_get_field(&payload, "fields");
                self.handle_char_edit_save(&fields)
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
            EVENT_FOCUS if target_id.starts_with("skillroll-") => {
                let mode = self.skill_selector_mode.unwrap_or(SkillSelectorMode::Roll);
                let items = self.skill_selector_candidates(mode);
                if let Some(idx) = items.iter().position(|s| s == &target_id) {
                    self.skill_selector_idx = idx;
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
        if let Some(ref di) = self.dice_input.clone() {
            return self.handle_dice_keydown(key, di.clone());
        }
        if self.skill_selector_mode.is_some() {
            let mode = self.skill_selector_mode.unwrap_or(SkillSelectorMode::Roll);
            let items = self.skill_selector_candidates(mode);
            let len = items.len();
            if len == 0 { return self.close_skill_selector(); }
            return match key {
                "ArrowDown" => {
                    self.skill_selector_idx = (self.skill_selector_idx + 1) % len;
                    vec![focus(&items[self.skill_selector_idx])]
                }
                "ArrowUp" => {
                    self.skill_selector_idx = (self.skill_selector_idx + len - 1) % len;
                    vec![focus(&items[self.skill_selector_idx])]
                }
                "Enter" => {
                    let id = items[self.skill_selector_idx].clone();
                    let k = id.strip_prefix("skillroll-").unwrap_or("").to_string();
                    self.handle_skill_selector(&k)
                }
                "Escape" => self.close_skill_selector(),
                _ => vec![],
            };
        }
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
            "dice" => {
                self.selector_open = false;
                self.selector_idx = 0;
                return self.open_dice_input();
            }
            "skill"       => {
                self.selector_open = false;
                self.selector_idx = 0;
                return self.open_skill_selector(SkillSelectorMode::Roll, "技能判定");
            }
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
            "pushed"      => {
                self.selector_open = false;
                self.selector_idx = 0;
                return self.open_skill_selector(SkillSelectorMode::Push, "プッシュロール");
            }
            "dev-check"   => {
                self.selector_open = false;
                self.selector_idx = 0;
                return self.open_skill_selector(SkillSelectorMode::DevCheck, "上達チェック");
            }
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
        if schema::roll_characteristics(&mut self.character).is_err() {
            return vec![];
        }
        // モーダルinputとメインviewを両方更新
        self.stat_view_cmds()
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
        let (label, field) = match key {
            "str" => ("STR",  Model::Strength),
            "con" => ("CON",  Model::Constitution),
            "siz" => ("SIZ",  Model::Size),
            "dex" => ("DEX",  Model::Dexterity),
            "app" => ("APP",  Model::Appearance),
            "int" => ("INT",  Model::Intelligence),
            "pow" => ("POW",  Model::Power),
            "edu" => ("EDU",  Model::Education),
            "luk" => ("幸運", Model::Luck),
            _ => return self.close_char_selector(),
        };
        let value = schema::get(&self.character, field);
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

    fn close_skill_selector(&mut self) -> Vec<DomCmd> {
        self.skill_selector_mode = None;
        self.skill_selector_idx = 0;
        vec![
            set_attr("skill-selector", "hidden", "true"),
            set_attr("skill-selector", "inert", "true"),
            focus("chat-input"),
        ]
    }

    fn open_skill_selector(&mut self, mode: SkillSelectorMode, title: &str) -> Vec<DomCmd> {
        let candidates = self.skill_selector_candidates(mode);
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

        self.skill_selector_mode = Some(mode);
        self.skill_selector_idx = 0;

        let mut cmds = vec![
            set_attr("selector", "hidden", "true"),
            set_attr("selector", "inert", "true"),
            set_text("skill-selector-title", title),
            set_attr("skill-selector", "hidden", ""),
            set_attr("skill-selector", "inert", ""),
        ];

        // 全ボタンをいったん非表示にしてから候補のみ表示
        for &(name, _) in SKILL_ID_MAP {
            let id = format!("skillroll-{}", name);
            if candidates.iter().any(|c| c == &id) {
                cmds.push(set_attr(&id, "hidden", ""));
                cmds.push(set_attr(&id, "inert", ""));
            } else {
                cmds.push(set_attr(&id, "hidden", "true"));
                cmds.push(set_attr(&id, "inert", "true"));
            }
        }

        if !candidates.is_empty() {
            cmds.push(focus(&candidates[0]));
        }
        cmds
    }

    fn skill_selector_candidates(&self, mode: SkillSelectorMode) -> Vec<String> {
        match mode {
            SkillSelectorMode::Roll => {
                SKILL_ID_MAP.iter()
                    .filter(|&&(_, field)| schema::skill::get(&self.character, field).is_ok())
                    .map(|&(name, _)| format!("skillroll-{}", name))
                    .collect()
            }
            SkillSelectorMode::Push => {
                // 直近のSkillログのうち、Failure以下でpushed:falseのもの
                self.roll_log.iter().rev()
                    .find_map(|entry| {
                        if let RollLog::Skill { field, level, pushed: false, .. } = entry {
                            let is_failure = match level {
                                Some(ResultLevel::Failure) | Some(ResultLevel::Fumble) | None => true,
                                _ => false,
                            };
                            if is_failure {
                                return SKILL_ID_MAP.iter()
                                    .find(|&&(_, f)| f == *field)
                                    .map(|&(name, _)| vec![format!("skillroll-{}", name)]);
                            }
                        }
                        None
                    })
                    .unwrap_or_default()
            }
            SkillSelectorMode::DevCheck => {
                // bonus <= 0 (常に0) かつ Regular以上の成功がある技能
                let mut eligible: Vec<Model> = Vec::new();
                for entry in &self.roll_log {
                    if let RollLog::Skill { field, level, pushed: false, .. } = entry {
                        let is_success = matches!(level,
                            Some(ResultLevel::Regular) |
                            Some(ResultLevel::Hard) |
                            Some(ResultLevel::Extreme) |
                            Some(ResultLevel::Critical)
                        );
                        if is_success && !eligible.contains(field) {
                            eligible.push(*field);
                        }
                    }
                }
                SKILL_ID_MAP.iter()
                    .filter(|&&(_, field)| eligible.contains(&field))
                    .map(|&(name, _)| format!("skillroll-{}", name))
                    .collect()
            }
        }
    }

    fn handle_skill_selector(&mut self, name: &str) -> Vec<DomCmd> {
        let mode = match self.skill_selector_mode {
            Some(m) => m,
            None => return self.close_skill_selector(),
        };
        let field = match SKILL_ID_MAP.iter().find(|&&(n, _)| n == name) {
            Some(&(_, f)) => f,
            None => return self.close_skill_selector(),
        };

        match mode {
            SkillSelectorMode::Roll => self.do_skill_roll(field, false),
            SkillSelectorMode::Push => {
                // 直近の同技能ログをpushed: trueにマーク
                for entry in self.roll_log.iter_mut().rev() {
                    if let RollLog::Skill { field: f, pushed, .. } = entry {
                        if *f == field && !*pushed {
                            *pushed = true;
                            break;
                        }
                    }
                }
                self.do_skill_roll(field, true)
            }
            SkillSelectorMode::DevCheck => self.do_dev_check(field),
        }
    }

    fn do_skill_roll(&mut self, field: Model, pushed: bool) -> Vec<DomCmd> {
        let difficulty = match schema::skill::get(&self.character, field) {
            Ok(v) => v,
            Err(_) => return self.close_skill_selector(),
        };
        let label = schema::label(field, crate::Lang::Ja);
        let result = dice::skill_roll(0, Some(difficulty as u32), dice::DifficultySpec::None).unwrap();
        let entry = RollLog::Skill { field, label, difficulty, total: result.total, level: result.level, pushed };
        let log_cmd = self.push_log(entry);
        let mut cmds = self.close_skill_selector();
        cmds.push(log_cmd);
        cmds
    }

    fn do_dev_check(&mut self, field: Model) -> Vec<DomCmd> {
        let current = match schema::skill::get(&self.character, field) {
            Ok(v) => v,
            Err(_) => return self.close_skill_selector(),
        };
        let label = schema::label(field, crate::Lang::Ja);
        let roll = crate::n_d_n(1, 100);
        if roll > current as u32 {
            // 成功: 1d10上昇
            let gain = crate::n_d_n(1, 10) as u16;
            let new_val = current.saturating_add(gain);
            let _ = schema::skill::set(&mut self.character, field, new_val);
            let name = SKILL_ID_MAP.iter().find(|&&(_, f)| f == field).map(|&(n, _)| n).unwrap_or("");
            let msg = format!("[上達チェック: {}] 出目: {} > {} → 成功! +{} → {}", label, roll, current, gain, new_val);
            let log_cmd = self.push_log(RollLog::Message(msg));
            let skill_val_id = format!("skill-val-{}", name);
            let mut cmds = self.close_skill_selector();
            cmds.push(log_cmd);
            cmds.push(set_text(&skill_val_id, &new_val.to_string()));
            cmds
        } else {
            let msg = format!("[上達チェック: {}] 出目: {} ≤ {} → 失敗", label, roll, current);
            let log_cmd = self.push_log(RollLog::Message(msg));
            let mut cmds = self.close_skill_selector();
            cmds.push(log_cmd);
            cmds
        }
    }

    fn handle_char_edit_save(&mut self, fields: &JsValue) -> Vec<DomCmd> {
        // 能力値
        let stat_map: &[(&str, Model)] = &[
            ("stat-str", Model::Strength),
            ("stat-con", Model::Constitution),
            ("stat-siz", Model::Size),
            ("stat-dex", Model::Dexterity),
            ("stat-app", Model::Appearance),
            ("stat-int", Model::Intelligence),
            ("stat-pow", Model::Power),
            ("stat-edu", Model::Education),
            ("stat-luk", Model::Luck),
        ];
        for &(name, field) in stat_map {
            let s = js_get_str(fields, name);
            if !s.is_empty() {
                let v: u16 = s.trim().parse().unwrap_or(0);
                let _ = schema::set(&mut self.character, field, v);
            }
        }
        // スキル
        for &(name, field) in SKILL_ID_MAP {
            let s = js_get_str(fields, name);
            if !s.is_empty() {
                let v: u16 = s.trim().parse().unwrap_or(0);
                let _ = schema::skill::set(&mut self.character, field, v);
            }
        }
        self.stat_view_cmds()
    }

    // 能力値・導出値・スキルのメインビューを全更新するDomCmdを生成
    fn stat_view_cmds(&self) -> Vec<DomCmd> {
        let ch = &self.character;
        let mut cmds = vec![];

        let stat_pairs: &[(&str, &str, Model)] = &[
            ("char-view-str",  "char-val-str",  Model::Strength),
            ("char-view-con",  "char-val-con",  Model::Constitution),
            ("char-view-siz",  "char-val-siz",  Model::Size),
            ("char-view-dex",  "char-val-dex",  Model::Dexterity),
            ("char-view-app",  "char-val-app",  Model::Appearance),
            ("char-view-int",  "char-val-int",  Model::Intelligence),
            ("char-view-pow",  "char-val-pow",  Model::Power),
            ("char-view-edu",  "char-val-edu",  Model::Education),
            ("char-view-luk",  "char-val-luk",  Model::Luck),
        ];
        for &(view_id, val_id, field) in stat_pairs {
            if let Ok(v) = schema::get(ch, field) {
                cmds.push(set_attr(view_id, "hidden", ""));
                cmds.push(set_text(val_id, &v.to_string()));
            }
        }

        // モーダルのinputにも反映（ダイスロール後に値が見える）
        let modal_pairs: &[(&str, Model)] = &[
            ("edit-str", Model::Strength),
            ("edit-con", Model::Constitution),
            ("edit-siz", Model::Size),
            ("edit-dex", Model::Dexterity),
            ("edit-app", Model::Appearance),
            ("edit-int", Model::Intelligence),
            ("edit-pow", Model::Power),
            ("edit-edu", Model::Education),
            ("edit-luk", Model::Luck),
        ];
        for &(id, field) in modal_pairs {
            if let Ok(v) = schema::get(ch, field) {
                cmds.push(set_attr(id, "value", &v.to_string()));
            }
        }

        // 導出値
        let derived: &[(&str, &str, Model)] = &[
            ("char-view-hp",    "char-hp",    Model::HitPoints),
            ("char-view-mp",    "char-mp",    Model::MagicPoints),
            ("char-view-san",   "char-san",   Model::Sanity),
            ("char-view-dodge", "char-dodge", Model::Dodge),
        ];
        for &(view_id, val_id, field) in derived {
            if let Ok(v) = schema::get(ch, field) {
                cmds.push(set_attr(view_id, "hidden", ""));
                cmds.push(set_text(val_id, &v.to_string()));
            }
        }

        // スキル（保存済みのみ表示）
        for &(name, field) in SKILL_ID_MAP {
            if let Ok(v) = schema::skill::get(ch, field) {
                cmds.push(set_attr(&format!("skill-view-{}", name), "hidden", ""));
                cmds.push(set_text(&format!("skill-val-{}", name), &v.to_string()));
            }
        }

        cmds
    }

    fn open_dice_input(&mut self) -> Vec<DomCmd> {
        self.dice_input = Some(DiceInput { phase: DicePhase::Count, count: 1, sides_idx: 4, modifier: 0 });
        let mut cmds = vec![
            set_attr("selector", "hidden", "true"),
            set_attr("selector", "inert", "true"),
        ];
        cmds.extend(self.render_dice_input());
        cmds
    }

    fn close_dice_input(&mut self) -> Vec<DomCmd> {
        self.dice_input = None;
        vec![
            set_attr("dice-input", "hidden", "true"),
            set_attr("dice-input", "inert", "true"),
            focus("chat-input"),
        ]
    }

    fn render_dice_input(&self) -> Vec<DomCmd> {
        let di = match &self.dice_input {
            Some(d) => d,
            None => return vec![],
        };
        let sides = DICE_SIDES[di.sides_idx];
        let modifier_str = if di.modifier == 0 {
            "0".to_string()
        } else if di.modifier > 0 {
            format!("+{}", di.modifier)
        } else {
            di.modifier.to_string()
        };
        let hint = match di.phase {
            DicePhase::Count    => format!("{}個 → Enter で次へ", di.count),
            DicePhase::Sides    => format!("{}個 × {}面 → Enter で次へ", di.count, sides),
            DicePhase::Modifier => {
                let mod_part = if di.modifier != 0 { format!(" {}", modifier_str) } else { String::new() };
                format!("{}個 × {}面{} → Enter でロール", di.count, sides, mod_part)
            }
        };
        let (show_count, show_sides, show_mod) = match di.phase {
            DicePhase::Count    => ("", "true", "true"),
            DicePhase::Sides    => ("true", "", "true"),
            DicePhase::Modifier => ("true", "true", ""),
        };
        vec![
            set_attr("dice-input", "hidden", ""),
            set_attr("dice-input", "inert", ""),
            set_attr("dice-count-row",    "hidden", show_count),
            set_attr("dice-sides-row",    "hidden", show_sides),
            set_attr("dice-modifier-row", "hidden", show_mod),
            set_text("dice-count-val",    &di.count.to_string()),
            set_text("dice-sides-val",    &format!("{}面", sides)),
            set_text("dice-modifier-val", &modifier_str),
            set_text("dice-hint",         &hint),
            focus("dice-input-focus"),
        ]
    }

    fn handle_dice_keydown(&mut self, key: &str, di: DiceInput) -> Vec<DomCmd> {
        match key {
            "Escape" => return self.close_dice_input(),
            "Enter" => {
                match di.phase {
                    DicePhase::Count => {
                        if let Some(d) = self.dice_input.as_mut() { d.phase = DicePhase::Sides; }
                        return self.render_dice_input();
                    }
                    DicePhase::Sides => {
                        if let Some(d) = self.dice_input.as_mut() { d.phase = DicePhase::Modifier; }
                        return self.render_dice_input();
                    }
                    DicePhase::Modifier => {
                        return self.execute_dice_roll();
                    }
                }
            }
            "ArrowUp" | "ArrowDown" => {
                let up = key == "ArrowUp";
                match di.phase {
                    DicePhase::Count => {
                        if let Some(d) = self.dice_input.as_mut() {
                            if up { d.count = d.count.saturating_add(1).min(99); }
                            else  { d.count = d.count.saturating_sub(1).max(1); }
                        }
                    }
                    DicePhase::Sides => {
                        let len = DICE_SIDES.len();
                        if let Some(d) = self.dice_input.as_mut() {
                            if up { d.sides_idx = (d.sides_idx + 1) % len; }
                            else  { d.sides_idx = (d.sides_idx + len - 1) % len; }
                        }
                    }
                    DicePhase::Modifier => {
                        if let Some(d) = self.dice_input.as_mut() {
                            if up { d.modifier = d.modifier.saturating_add(1); }
                            else  { d.modifier = d.modifier.saturating_sub(1); }
                        }
                    }
                }
                return self.render_dice_input();
            }
            _ => {}
        }
        vec![]
    }

    fn execute_dice_roll(&mut self) -> Vec<DomCmd> {
        let di = match self.dice_input.take() {
            Some(d) => d,
            None => return vec![],
        };
        let sides = DICE_SIDES[di.sides_idx];
        let raw: u32 = crate::n_d_n(di.count, sides);
        let total = (raw as i32 + di.modifier).max(0) as u32;
        let modifier_str = if di.modifier > 0 {
            format!("+{}", di.modifier)
        } else if di.modifier < 0 {
            di.modifier.to_string()
        } else {
            String::new()
        };
        let expr = format!("{}d{}{}", di.count, sides, modifier_str);
        let msg = format!("[ダイスロール: {}] 出目: {} → 合計: {}", expr, raw, total);
        let log_cmd = self.push_log(RollLog::Message(msg));
        let mut cmds = self.close_dice_input();
        cmds.push(log_cmd);
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
                let kind = if *pushed { "プッシュロール" } else { "技能判定" };
                let result = match level {
                    Some(l) => l.label(Lang::Ja),
                    None    => "出目のみ",
                };
                write!(f, "[{}: {}={}] 出目: {}  結果: {}", kind, label, difficulty, total, result)
            }
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
