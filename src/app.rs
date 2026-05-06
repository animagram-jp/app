use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use std::collections::BTreeMap;
use crate::js_client::{
    DomCmd, Operation,
    get_js_str, get_js_f64,
    EventType, KeyName,
    Gesture, PointerState, detect_gesture,
    dom,
};
use crate::character::{Character, Characteristic, Skill, Profile};
use crate::data_struct::{DataStruct, WAL_NAME};
use crate::wal::WalStore;

// ============================================================
// canvas state
// ============================================================

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Overlay {
    #[default]
    None,
    Select { step: u8, index: usize },
    Input  { step: u8, value: u32 },
}

#[derive(Clone, Copy, PartialEq, Default)]
enum Dialog {
    #[default]
    None,
    Modal,
    Drawer,
}

struct CanvasState {
    overlay:  Overlay,
    dialog:   Dialog,
    buf:      DataStruct,               // 編集中バッファ。buf.identity=0は未保存
    pool:     BTreeMap<u32, DataStruct>, // char_id → 全フィールド
    next_id:  u32,
    wal:      WalStore,
}

impl CanvasState {
    fn new(wal: WalStore) -> Self {
        Self {
            overlay:  Overlay::default(),
            dialog:   Dialog::default(),
            buf:      DataStruct::new(),
            pool:     BTreeMap::new(),
            next_id:  0,
            wal,
        }
    }

    fn alloc(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    fn load_all_from_wal(&mut self) {
        for (id, raw) in self.wal.get_all(WAL_NAME) {
            let id = id as u32;
            let mut ds = DataStruct::from_bytes(&raw);
            ds.identity = id;
            self.pool.insert(id, ds);
            if self.next_id < id { self.next_id = id; }
        }
    }

    // buf.identity が指すエントリをpoolから復元する。0なら何もしない。
    fn load_saved(&mut self) {
        let id = self.buf.identity;
        if id == 0 { return; }
        if let Some(src) = self.pool.get(&id) {
            self.buf = src.clone();
        }
    }

    fn save(&mut self) -> u32 {
        if self.buf.identity == 0 {
            self.buf.identity = self.alloc();
        }
        let id = self.buf.identity;
        self.pool.insert(id, self.buf.clone());
        self.wal.set(WAL_NAME, id as u64, &self.buf.to_bytes());
        id
    }

    // bufのフィールドをクリアしつつidentityは保持する
    fn discard_buffer(&mut self) {
        let id = self.buf.identity;
        self.buf = DataStruct::new();
        self.buf.identity = id;
    }

    fn saved_name_list(&self) -> Vec<(u32, String)> {
        self.pool.iter().map(|(&id, ds)| {
            let name = ds.get(&Character::Profile(Profile::Name)).ok()
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| format!("#{}", id));
            (id, name)
        }).collect()
    }

    fn on_click(&mut self, id: &dom::Id, _key: KeyName) -> Vec<DomCmd> {
        let is_backdrop = id.0.len() == 1;

        match (self.dialog, id.last_tag(), is_backdrop) {
            (Dialog::Modal, Some(dom::Tag::Modal), true) => {
                self.dialog = Dialog::None;
                self.discard_buffer();
                let mut cmds = crate::event::reset_modal();
                cmds.push(DomCmd::new(Operation::CloseModal, "modal", None, None));
                cmds
            }
            (Dialog::Drawer, Some(dom::Tag::Drawer), true) => {
                self.dialog = Dialog::None;
                vec![DomCmd::new(Operation::CloseModal, "drawer", None, None)]
            }
            (Dialog::Modal,  _, _) => self.on_click_normal(id, id.last_tag()),
            (Dialog::Drawer, _, _) => vec![],
            (Dialog::None, last_tag, _) => self.on_click_normal(id, last_tag),
        }
    }

    fn on_click_normal(&mut self, id: &dom::Id, last_tag: Option<&dom::Tag>) -> Vec<DomCmd> {
        match last_tag {
            Some(dom::Tag::Button) if id.encode() == "main_header_button" => {
                self.dialog = Dialog::Modal;
                self.load_saved();
                let mut cmds = crate::event::open_modal();
                cmds.extend(crate::event::restore_modal(&self.buf));
                cmds.push(DomCmd::new(Operation::OpenModal, "modal", None, None));
                cmds
            }
            Some(dom::Tag::Button) if {
                let s = &id.0;
                s.len() == 5
                && s[0].tag == dom::Tag::Modal
                && s[1].tag == dom::Tag::Fieldset && s[1].n == Some(2)
                && s[3].tag == dom::Tag::Tr
                && s[4].tag == dom::Tag::Button
            } => {
                let row = id.0[3].n.unwrap_or(0) as usize;
                crate::event::roll_characteristic(row, &mut self.buf)
            }
            Some(dom::Tag::Button) if id.encode() == "modal_fieldset-2_legend_button" => {
                crate::event::roll_all_characteristics(&mut self.buf)
            }
            Some(dom::Tag::Button) if id.encode() == "modal_footer_button" => {
                self.dialog = Dialog::None;
                self.save();
                let mut cmds = crate::event::toast_saved();
                cmds.extend(crate::event::update_debug_select(&self.saved_name_list(), self.buf.identity_opt()));
                cmds.extend(crate::event::update_character_view(&self.buf));
                cmds.push(DomCmd::new(Operation::CloseModal, "modal", None, None));
                cmds
            }
            _ => vec![],
        }
    }

    fn on_keydown(&mut self, _id: &dom::Id, key: KeyName) -> Vec<DomCmd> {
        match key {
            KeyName::Escape => {
                match self.dialog {
                    Dialog::Modal => {
                        self.dialog = Dialog::None;
                        self.discard_buffer();
                        let mut cmds = crate::event::reset_modal();
                        cmds.push(DomCmd::new(Operation::CloseModal, "modal", None, None));
                        return cmds;
                    }
                    Dialog::Drawer => {
                        self.dialog = Dialog::None;
                        return vec![DomCmd::new(Operation::CloseModal, "drawer", None, None)];
                    }
                    Dialog::None => {}
                }
                if self.overlay != Overlay::None {
                    self.overlay = Overlay::None;
                }
                vec![]
            }
            KeyName::Enter => {
                if self.dialog == Dialog::Modal {
                    self.dialog = Dialog::None;
                    self.save();
                    let mut cmds = crate::event::toast_saved();
                    cmds.extend(crate::event::update_debug_select(&self.saved_name_list(), self.buf.identity_opt()));
                    cmds.extend(crate::event::update_character_view(&self.buf));
                    cmds.push(DomCmd::new(Operation::CloseModal, "modal", None, None));
                    return cmds;
                }
                vec![]
            }
            _ => vec![],
        }
    }

    fn on_input(&mut self, id: &dom::Id, value: &str) -> Vec<DomCmd> {
        let segs = &id.0;

        // 専門分野: "modal_fieldset-3_table_tr-{row}_td-1_input"
        if segs.len() == 6
            && segs[0].tag == dom::Tag::Modal
            && segs[1].tag == dom::Tag::Fieldset && segs[1].n == Some(3)
            && segs[3].tag == dom::Tag::Tr
            && segs[4].tag == dom::Tag::Td && segs[4].n == Some(1)
            && segs[5].tag == dom::Tag::Input
        {
            let row = segs[3].n.unwrap_or(0) as usize;
            if row == 0 || row > 20 { return vec![]; }
            let skills = Skill::list();
            let skill  = &skills[row - 1];
            let field  = Character::Skill(Skill::list().remove(row - 1));
            let (occ, int, bonus, _) = self.buf.get(&field)
                .map(Skill::decode).unwrap_or((0, 0, 0, String::new()));
            let _ = self.buf.set(&field, &Skill::encode(occ, int, bonus, Some(value)));
            return crate::event::on_skill_spec_input(row, skill, value);
        }

        // "modal_fieldset-{fs}_table_tr-{row}_input-{col}"
        if segs.len() != 5 { return vec![]; }
        if segs[0].tag != dom::Tag::Modal    { return vec![]; }
        if segs[1].tag != dom::Tag::Fieldset { return vec![]; }
        if segs[3].tag != dom::Tag::Tr       { return vec![]; }
        if segs[4].tag != dom::Tag::Input    { return vec![]; }

        let fs  = segs[1].n.unwrap_or(0) as usize;
        let row = segs[3].n.unwrap_or(0) as usize;
        let col = segs[4].n.unwrap_or(0) as usize;

        if row == 0 { return vec![]; }

        match fs {
            1 if row <= 6 => {
                let profiles = [
                    Profile::Name, Profile::Birthpalce, Profile::Pronoun,
                    Profile::Occupation, Profile::Residence, Profile::Age,
                ];
                let field = Character::Profile(profiles[row - 1]);
                let _ = self.buf.set(&field, value.as_bytes());
                vec![]
            }
            2 if row <= 9 => {
                let field = Character::Characteristic(Characteristic::list()[row - 1]);
                let mut vals = self.buf.get(&field)
                    .map(Characteristic::decode)
                    .unwrap_or([0; 3]);
                vals[col - 1] = value.parse().unwrap_or(0);
                let _ = self.buf.set(&field, &Characteristic::encode(vals));
                let [base, delta, bonus] = vals;
                crate::event::on_characteristic_input(row, base, delta, bonus)
            }
            3 if row <= 20 => {
                let skills = Skill::list();
                let field  = Character::Skill(Skill::list().remove(row - 1));
                let (mut occ, mut int, mut bonus, spec) = self.buf.get(&field)
                    .map(Skill::decode)
                    .unwrap_or((0, 0, 0, String::new()));
                let v: i32 = value.parse().unwrap_or(0);
                match col {
                    1 => occ   = v.max(0) as u16,
                    2 => int   = v.max(0) as u16,
                    3 => bonus = v,
                    _ => {}
                }
                let _ = self.buf.set(&field, &Skill::encode(occ, int, bonus, Some(&spec)));
                let base = skills[row - 1].base_value();
                crate::event::on_skill_input(row, base, occ, int, bonus)
            }
            _ => vec![],
        }
    }

    fn on_change(&mut self, id: &dom::Id, value: &str) -> Vec<DomCmd> {
        if id.encode() == "main_div_section-1_section-1_select" {
            let char_id: u32 = value.parse().unwrap_or(0);
            if char_id == 0 || !self.pool.contains_key(&char_id) { return vec![]; }
            self.buf.identity = char_id;
            self.load_saved();
            let mut cmds = crate::event::update_character_view(&self.buf);
            cmds.extend(crate::event::update_debug_select(&self.saved_name_list(), self.buf.identity_opt()));
            return cmds;
        }
        vec![]
    }

    fn on_gesture(&mut self, _gesture: Gesture, _id: &dom::Id) -> Vec<DomCmd> {
        vec![]
    }
}

// ============================================================
// App
// ============================================================

#[wasm_bindgen]
pub struct App {
    pointer_state: PointerState,
    canvas_state:  CanvasState,
    dom_cmds:      Vec<DomCmd>,
    pub(crate) log_stack: Vec<crate::event::LogStack>,
}

#[wasm_bindgen]
impl App {
    pub async fn init() -> App {
        let mut wal = WalStore::new();
        let _ = wal.open(WAL_NAME).await;
        let mut canvas_state = CanvasState::new(wal);
        canvas_state.load_all_from_wal();

        let mut dom_cmds = Vec::new();
        let name_list = canvas_state.saved_name_list();
        if !name_list.is_empty() {
            canvas_state.buf.identity = name_list[0].0;
            canvas_state.load_saved();
            dom_cmds.extend(crate::event::update_character_view(&canvas_state.buf));
        }
        dom_cmds.extend(crate::event::update_debug_select(&name_list, canvas_state.buf.identity_opt()));

        App {
            pointer_state: PointerState::default(),
            canvas_state,
            dom_cmds,
            log_stack: Vec::new(),
        }
    }

    pub fn flush(&mut self) -> JsValue {
        let out = serde_wasm_bindgen::to_value(&self.dom_cmds).unwrap_or(JsValue::NULL);
        self.dom_cmds.clear();
        out
    }

    pub fn event(&mut self, payload: JsValue) {
        let event_type = EventType::decode(&get_js_str(&payload, "event_type").unwrap_or_default());
        let id         = dom::Id::decode(&get_js_str(&payload, "target_id").unwrap_or_default());
        let key        = KeyName::decode(&get_js_str(&payload, "key").unwrap_or_default());
        let x          = get_js_f64(&payload, "x").unwrap_or(0.0);
        let y          = get_js_f64(&payload, "y").unwrap_or(0.0);
        let time       = get_js_f64(&payload, "time").unwrap_or(0.0);

        let cmds = match event_type {
            EventType::Click    => self.canvas_state.on_click(&id, key),
            EventType::KeyDown  => self.canvas_state.on_keydown(&id, key),
            EventType::Change   => {
                let value = get_js_str(&payload, "value").unwrap_or_default();
                self.canvas_state.on_change(&id, &value)
            }
            EventType::Input    => {
                let value = get_js_str(&payload, "value").unwrap_or_default();
                self.canvas_state.on_input(&id, &value)
            }
            EventType::PointerDown | EventType::PointerMove
            | EventType::PointerUp | EventType::PointerCancel => {
                self.pointer_state = self.pointer_state.update(&event_type, x, y, time);
                if let Some(g) = detect_gesture(&self.pointer_state, time) {
                    self.canvas_state.on_gesture(g, &id)
                } else {
                    vec![]
                }
            }
            _ => vec![],
        };

        self.dom_cmds.extend(cmds);
    }
}
