use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use std::collections::BTreeMap;
use crate::js_client::{
    CanvasCmd, Operation,
    get_js_str, get_js_f64,
    EventType, KeyName,
    Gesture, PointerState, detect_gesture,
    Device, detect_device,
    dom,
};
use crate::character::{Character, Characteristic, Skill, Profile};
use crate::Lang;
const LANG: Lang = Lang::Ja;
use crate::data_struct::{DataStruct, WAL_NAME};
use crate::wal::WalStore;

// ============================================================
// canvas state
// ============================================================

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Dialog {
    #[default]
    None,
    Modal,
    Drawer,
    Select { step: u8, index: usize },
    Input  { step: u8, value: u32 },
}

struct CanvasState {
    dialog:  Dialog,
    buf:     DataStruct,               // 編集中バッファ。buf.identity=0は未保存
    pool:    BTreeMap<u32, DataStruct>, // char_id → 全フィールド
    next_id: u32,
    wal:     WalStore,
}

impl CanvasState {
    fn new(wal: WalStore) -> Self {
        Self {
            dialog:  Dialog::default(),
            buf:     DataStruct::new(),
            pool:    BTreeMap::new(),
            next_id: 0,
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

    fn on_click(&mut self, id: &dom::Id, _key: KeyName) -> Vec<CanvasCmd> {
        let is_backdrop = id.0.len() == 1;

        match (self.dialog, id.last_tag(), is_backdrop) {
            (Dialog::Modal, Some(dom::Tag::Modal), true) => {
                self.dialog = Dialog::None;
                self.discard_buffer();
                let mut cmds = crate::event::reset_modal();
                cmds.push(CanvasCmd::new(Operation::CloseModal, "modal", None, None));
                cmds
            }
            (Dialog::Drawer, Some(dom::Tag::Drawer), true) => {
                self.dialog = Dialog::None;
                vec![CanvasCmd::new(Operation::CloseModal, "drawer", None, None)]
            }
            (Dialog::Modal,  _, _) => self.on_click_normal(id, id.last_tag()),
            (Dialog::Drawer, _, _) => vec![],
            (Dialog::None, last_tag, _) => self.on_click_normal(id, last_tag),
            (Dialog::Select { .. } | Dialog::Input { .. }, _, _) => vec![],
        }
    }

    fn on_click_normal(&mut self, id: &dom::Id, last_tag: Option<&dom::Tag>) -> Vec<CanvasCmd> {
        match last_tag {
            Some(dom::Tag::Button) if id.encode() == "main_header_button" => {
                self.dialog = Dialog::Modal;
                self.load_saved();
                let mut cmds = crate::event::open_modal();
                cmds.extend(crate::event::restore_modal(&self.buf));
                cmds.push(CanvasCmd::new(Operation::OpenModal, "modal", None, None));
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
                cmds.push(CanvasCmd::new(Operation::CloseModal, "modal", None, None));
                cmds
            }
            _ => vec![],
        }
    }

    fn on_keydown(&mut self, _id: &dom::Id, key: KeyName) -> Vec<CanvasCmd> {
        match key {
            KeyName::Escape => {
                match self.dialog {
                    Dialog::Modal => {
                        self.dialog = Dialog::None;
                        self.discard_buffer();
                        let mut cmds = crate::event::reset_modal();
                        cmds.push(CanvasCmd::new(Operation::CloseModal, "modal", None, None));
                        return cmds;
                    }
                    Dialog::Drawer => {
                        self.dialog = Dialog::None;
                        return vec![CanvasCmd::new(Operation::CloseModal, "drawer", None, None)];
                    }
                    Dialog::None => {}
                    Dialog::Select { .. } | Dialog::Input { .. } => {
                        self.dialog = Dialog::None;
                    }
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
                    cmds.push(CanvasCmd::new(Operation::CloseModal, "modal", None, None));
                    return cmds;
                }
                vec![]
            }
            _ => vec![],
        }
    }

    fn on_input(&mut self, id: &dom::Id, value: &str) -> Vec<CanvasCmd> {
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
            if row == 0 || row > Skill::list().len() { return vec![]; }
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
            3 if row <= Skill::list().len() => {
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

    fn on_change(&mut self, id: &dom::Id, value: &str) -> Vec<CanvasCmd> {
        if id.encode() == "main_div_section-1_section-1_select" {
            let char_id: u32 = value.parse().unwrap_or(0);
            if char_id == 0 || !self.pool.contains_key(&char_id) { return vec![]; }
            self.buf.identity = char_id;
            self.load_saved();
            let mut cmds = crate::event::update_character_view(&self.buf);
            cmds.extend(crate::event::update_debug_select(&self.saved_name_list(), self.buf.identity_opt()));
            return cmds;
        }

        let segs = &id.0;

        // "modal_fieldset-3_table_tr-{row}_td-1_select"
        if segs.len() == 5
            && segs[0].tag == dom::Tag::Modal
            && segs[1].tag == dom::Tag::Fieldset && segs[1].n == Some(3)
            && segs[3].tag == dom::Tag::Tr
            && segs[4].tag == dom::Tag::Select
        {
            let row    = segs[3].n.unwrap_or(0) as usize;
            let inp_id = format!("modal_fieldset-3_table_tr-{}_td-1_input", row);
            if value == "custom" {
                // 自由記入モード: inputをshow+focus、specをクリア
                let field = Character::Skill(Skill::list().remove(row - 1));
                let (occ, int, bonus, _) = self.buf.get(&field)
                    .map(Skill::decode).unwrap_or((0, 0, 0, String::new()));
                let _ = self.buf.set(&field, &Skill::encode(occ, int, bonus, Some("")));
                return vec![
                    CanvasCmd::new(Operation::RemoveClass, &inp_id, None, Some("hidden")),
                    CanvasCmd::new(Operation::SetValue,    &inp_id, None, Some("")),
                    CanvasCmd::new(Operation::Focus,       &inp_id, None, None),
                ];
            } else {
                // 固定variant選択: inputをhide、specをvalueで保存
                let field = Character::Skill(Skill::list().remove(row - 1));
                let (occ, int, bonus, _) = self.buf.get(&field)
                    .map(Skill::decode).unwrap_or((0, 0, 0, String::new()));
                let _ = self.buf.set(&field, &Skill::encode(occ, int, bonus, Some(value)));
                let skill  = Skill::list().remove(row - 1);
                let th_id  = format!("modal_fieldset-3_table_tr-{}_th", row);
                let base_id = format!("modal_fieldset-3_table_tr-{}_span-1", row);
                let mut cmds = vec![
                    CanvasCmd::new(Operation::AddClass, &inp_id, None, Some("hidden")),
                    CanvasCmd::new(Operation::SetText,  &th_id,  None, Some(&skill.label_with_spec(LANG, value))),
                    CanvasCmd::new(Operation::SetText,  &base_id, None, Some(&skill.base_value().to_string())),
                ];
                // 合計も更新
                let (occ2, int2, bonus2, _) = self.buf.get(&field)
                    .map(Skill::decode).unwrap_or((0, 0, 0, String::new()));
                cmds.extend(crate::event::on_skill_input(row, skill.base_value(), occ2, int2, bonus2));
                return cmds;
            }
        }

        vec![]
    }

    fn on_blur(&mut self, id: &dom::Id, value: &str) -> Vec<CanvasCmd> {
        let segs = &id.0;
        // "modal_fieldset-3_table_tr-{row}_td-1_input" + 空 → selectに戻す
        if segs.len() == 6
            && segs[0].tag == dom::Tag::Modal
            && segs[1].tag == dom::Tag::Fieldset && segs[1].n == Some(3)
            && segs[3].tag == dom::Tag::Tr
            && segs[4].tag == dom::Tag::Td && segs[4].n == Some(1)
            && segs[5].tag == dom::Tag::Input
            && value.is_empty()
        {
            let row    = segs[3].n.unwrap_or(0) as usize;
            let inp_id = format!("modal_fieldset-3_table_tr-{}_td-1_input", row);
            let sel_id = format!("modal_fieldset-3_table_tr-{}_td-1_select", row);
            return vec![
                CanvasCmd::new(Operation::AddClass,    &inp_id, None, Some("hidden")),
                CanvasCmd::new(Operation::RemoveClass, &sel_id, None, Some("hidden")),
            ];
        }
        vec![]
    }

    fn on_gesture(&mut self, _gesture: Gesture, _id: &dom::Id) -> Vec<CanvasCmd> {
        vec![]
    }
}

// ============================================================
// App
// ============================================================

/// JS から受け取った生ペイロードをデコードした入力イベント。
/// ドメイン知識を持たない汎用の構造体として定義する。
struct CanvasEvent {
    event_type: EventType,
    id:         dom::Id,
    key:        KeyName,
    value:      String,
    x:          f64,
    y:          f64,
    time:       f64,
}

impl CanvasEvent {
    fn decode(payload: &JsValue) -> Self {
        todo!("JsValue から各フィールドをデコードして CanvasEvent を返す")
    }
}

#[wasm_bindgen]
pub struct App {
    device:        Device,
    pointer_state: PointerState,
    canvas_state:  CanvasState,
    events:         Vec<CanvasEvent>,  // 受信キュー
    cmds:      Vec<CanvasCmd>,      // 送信キュー
    pub(crate) log_stack: Vec<crate::event::LogStack>,
}

#[wasm_bindgen]
impl App {
    pub async fn init(screen_width: u32, pointer_coarse: bool) -> App {
        let mut wal = WalStore::new();
        let _ = wal.open(WAL_NAME).await;
        let mut canvas_state = CanvasState::new(wal);
        canvas_state.load_all_from_wal();

        let mut cmds = Vec::new();
        let name_list = canvas_state.saved_name_list();
        if !name_list.is_empty() {
            canvas_state.buf.identity = name_list[0].0;
            canvas_state.load_saved();
            cmds.extend(crate::event::update_character_view(&canvas_state.buf));
        }
        cmds.extend(crate::event::update_debug_select(&name_list, canvas_state.buf.identity_opt()));

        App {
            device:        detect_device(screen_width, pointer_coarse),
            pointer_state: PointerState::default(),
            canvas_state,
            events: Vec::new(),
            cmds,
            log_stack: Vec::new(),
        }
    }

    pub fn flush(&mut self) -> JsValue {
        let out = serde_wasm_bindgen::to_value(&self.cmds).unwrap_or(JsValue::NULL);
        self.cmds.clear();
        out
    }

    /// JS 側からイベントを受け取り、受信キューに積んでループを回す。
    pub fn event(&mut self, payload: JsValue) {
        self.events.push(CanvasEvent::decode(&payload));
        while let Some(ev) = self.events.pop() {
            let cmds = self.dispatch(ev);
            self.cmds.extend(cmds);
            // todo: cmd 間の整合処理（必要なら events に追加イベントを積む）
        }
    }

    /// CanvasEvent をディスパッチして CanvasCmd のリストを返す。
    fn dispatch(&mut self, ev: CanvasEvent) -> Vec<CanvasCmd> {
        todo!("ev.event_type に応じて canvas_state.on_* を呼び分け、CanvasCmd を返す")
    }
}
