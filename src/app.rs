use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use crate::js_client::{
    DomCmd, Operation,
    get_js_str, get_js_f64,
    EventType, KeyName,
    Gesture, PointerState, detect_gesture,
    dom,
};

// ============================================================
// canvas state
// ============================================================

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Overlay {
    #[default]
    None,
    Select { step: u8, index: usize }, // 上下方向のボタンリストとfocusで構成するセレクターUI
    Input  { step: u8, value: u32 },   // 左端からlabel, up, down, value, nextのワンライナーUI
}

// modal/drawerは排他。同時に開かない。
#[derive(Clone, Copy, PartialEq, Default)]
enum Dialog {
    #[default]
    None,
    Modal,   // <dialog id="modal">  キャラシ編集
    Drawer,  // <dialog id="drawer"> サイドドロワー
}

#[derive(Default)]
struct CanvasState {
    overlay:    Overlay,
    dialog:     Dialog,
    char_vals:  [[i32; 3]; 9],   // [row-1] = [初期値, 変動値, 補正値]
    skill_pts:  [[i32; 3]; 20],  // [row-1] = [職業pt, 興味pt, 補正値]
}

impl CanvasState {
    fn on_click(&mut self, id: &dom::Id, _key: KeyName) -> Vec<DomCmd> {
        // backdrop click: segments が1つ = dialog要素そのものへのclick
        let is_backdrop = id.0.len() == 1;

        match (self.dialog, id.last_tag(), is_backdrop) {
            // --- modal backdrop → close + reset ---
            (Dialog::Modal, Some(dom::Tag::Modal), true) => {
                self.dialog = Dialog::None;
                let mut cmds = crate::event::reset_modal(&mut self.char_vals, &mut self.skill_pts);
                cmds.push(DomCmd::new(Operation::CloseModal, "modal", None, None));
                cmds
            }
            // --- drawer backdrop → close ---
            (Dialog::Drawer, Some(dom::Tag::Drawer), true) => {
                self.dialog = Dialog::None;
                vec![DomCmd::new(Operation::CloseModal, "drawer", None, None)]
            }
            // --- modal内部イベント ---
            (Dialog::Modal, _, _) => {
                self.on_click_normal(id, id.last_tag())
            }
            // --- drawer内部イベント ---
            (Dialog::Drawer, _, _) => {
                // todo: drawer内操作
                vec![]
            }
            // --- 通常状態 ---
            (Dialog::None, last_tag, _) => self.on_click_normal(id, last_tag),
        }
    }

    fn on_click_normal(&mut self, id: &dom::Id, last_tag: Option<&dom::Tag>) -> Vec<DomCmd> {
        match last_tag {
            // ✏️ ヘッダーボタン → modal open
            Some(dom::Tag::Button) if id.encode() == "main_header_button" => {
                self.dialog = Dialog::Modal;
                let mut cmds = crate::event::open_modal();
                cmds.push(DomCmd::new(Operation::OpenModal, "modal", None, None));
                cmds
            }
            // 🎲 fieldset-2 行サイコロ: "modal_fieldset-2_table_tr-{row}_button"
            Some(dom::Tag::Button) if {
                let s = &id.0;
                s.len() == 5
                && s[0].tag == dom::Tag::Modal
                && s[1].tag == dom::Tag::Fieldset && s[1].n == Some(2)
                && s[3].tag == dom::Tag::Tr
                && s[4].tag == dom::Tag::Button
            } => {
                let row = id.0[3].n.unwrap_or(0) as usize;
                crate::event::roll_characteristic(row, &mut self.char_vals)
            }
            // 🎲 fieldset-2 legend button: 能力値一括ロール
            Some(dom::Tag::Button) if id.encode() == "modal_fieldset-2_legend_button" => {
                crate::event::roll_all_characteristics(&mut self.char_vals)
            }
            // 💾 保存ボタン: キャッシュ保持のままclose + toast (todo: DataStructへの保存)
            Some(dom::Tag::Button) if id.encode() == "modal_footer_button" => {
                self.dialog = Dialog::None;
                let mut cmds = crate::event::toast_saved();
                cmds.push(DomCmd::new(Operation::CloseModal, "modal", None, None));
                cmds
            }
            // todo: overlay系ボタン、li選択、etc.
            _ => vec![],
        }
    }

    fn on_keydown(&mut self, id: &dom::Id, key: KeyName) -> Vec<DomCmd> {
        match key {
            // Escape: 開いているものを優先順で閉じる
            KeyName::Escape => {
                match self.dialog {
                    Dialog::Modal => {
                        self.dialog = Dialog::None;
                        let mut cmds = crate::event::reset_modal(&mut self.char_vals, &mut self.skill_pts);
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
                    // todo: overlay DOM非表示
                }
                vec![]
            }
            KeyName::Enter => {
                if self.dialog == Dialog::Modal {
                    self.dialog = Dialog::None;
                    let mut cmds = crate::event::toast_saved();
                    cmds.push(DomCmd::new(Operation::CloseModal, "modal", None, None));
                    return cmds;
                }
                vec![]
            }
            // todo: ArrowUp/Down (overlay内選択)
            _ => vec![],
        }
    }

    fn on_input(&mut self, id: &dom::Id, value: &str) -> Vec<DomCmd> {
        // 対象: "modal_fieldset-{fs}_table_tr-{row}_input-{col}"
        // segments: [modal, fieldset-N, table, tr-N, input-N]
        let segs = &id.0;
        if segs.len() != 5 { return vec![]; }
        if segs[0].tag != dom::Tag::Modal    { return vec![]; }
        if segs[1].tag != dom::Tag::Fieldset { return vec![]; }
        if segs[3].tag != dom::Tag::Tr       { return vec![]; }
        if segs[4].tag != dom::Tag::Input    { return vec![]; }

        let fs  = segs[1].n.unwrap_or(0) as usize;
        let row = segs[3].n.unwrap_or(0) as usize;
        let col = segs[4].n.unwrap_or(0) as usize; // 1, 2, or 3

        if row == 0 { return vec![]; }

        match fs {
            2 if row <= 9 => {
                let v: i32 = value.parse().unwrap_or(0);
                self.char_vals[row - 1][col - 1] = v;
                let [base, delta, bonus] = self.char_vals[row - 1];
                crate::event::on_characteristic_input(row, base, delta, bonus)
            }
            3 if row <= 20 => {
                let v: i32 = value.parse().unwrap_or(0);
                self.skill_pts[row - 1][col - 1] = v;
                let [occ, int, bonus] = self.skill_pts[row - 1];
                let skills = crate::character::Skill::default_rows();
                let base = skills.get(row - 1).map(|s| s.base_value()).unwrap_or(0);
                crate::event::on_skill_input(row, base, occ as u16, int as u16, bonus)
            }
            _ => vec![],
        }
    }

    fn on_gesture(&mut self, _gesture: Gesture, _id: &dom::Id) -> Vec<DomCmd> {
        // todo: swipe/longpress
        vec![]
    }
}

// ============================================================
// app
// ============================================================

#[wasm_bindgen]
pub struct App {
    pointer_state: PointerState,
    canvas_state:  CanvasState,
    dom_cmds:      Vec<DomCmd>,
    pub(crate) log_stack: Vec<crate::event::LogStack>,
    // todo: DataStruct を追加する
}

#[wasm_bindgen]
impl App {
    pub fn init() -> App {
        App {
            pointer_state: PointerState::default(),
            canvas_state:  CanvasState::default(),
            dom_cmds:      Vec::new(),
            log_stack:     Vec::new(),
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
            EventType::Click => {
                self.canvas_state.on_click(&id, key)
            }
            EventType::KeyDown => {
                self.canvas_state.on_keydown(&id, key)
            }
            EventType::Input => {
                let value = get_js_str(&payload, "value").unwrap_or_default();
                self.canvas_state.on_input(&id, &value)
            }
            // pointer系: gesture判定に委ねる
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
