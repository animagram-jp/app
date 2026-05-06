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
    overlay: Overlay,
    dialog:  Dialog,
}

impl CanvasState {
    fn on_click(&mut self, id: &dom::Id, _key: KeyName) -> Vec<DomCmd> {
        // backdrop click: segments が1つ = dialog要素そのものへのclick
        let is_backdrop = id.0.len() == 1;

        match (self.dialog, id.last_tag(), is_backdrop) {
            // --- modal backdrop → close ---
            (Dialog::Modal, Some(dom::Tag::Modal), true) => {
                self.dialog = Dialog::None;
                vec![DomCmd::new(Operation::CloseModal, "modal", None, None)]
            }
            // --- drawer backdrop → close ---
            (Dialog::Drawer, Some(dom::Tag::Drawer), true) => {
                self.dialog = Dialog::None;
                vec![DomCmd::new(Operation::CloseModal, "drawer", None, None)]
            }
            // --- modal内部イベント: overlay close ---
            (Dialog::Modal, _, _) => {
                // todo: modal内キャラシ編集操作
                vec![]
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
                vec![DomCmd::new(Operation::OpenModal, "modal", None, None)]
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
                        return vec![DomCmd::new(Operation::CloseModal, "modal", None, None)];
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
            // todo: ArrowUp/Down, Enter (overlay内選択)
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
                // todo: textarea "/" トリガー
                vec![]
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
