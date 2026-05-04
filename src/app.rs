use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use crate::{Lang, dice::{self, ResultLevel}};
use crate::table::Roll;
use crate::character::{Instance, Model, schema};
use crate::js_client::{
    Operation, DomCmd,
    get_js_str, get_js_field, get_js_f64,
    EventType, KeyName,
    Gesture, PointerState, detect_gesture,
    Dom,
};

// ============================================================
// canvas state
// ============================================================

#[derive(Clone, Copy, PartialEq, Default)]
enum Overlay {
    #[default]
    None,
    Select { step: u8, index: usize }, // 上下方向のボタンリストとfocusで構成するセレクターUI
    Input  { step: u8, value: u32 },   // 左端からlabel, up, down, value, nextのワンライナーUI)
}

#[derive(Default)]
struct CanvasState {
    overlay: Overlay,
    modal: bool,      // true = open, false = close
    drawer: bool,     // true = open, false = close
}

impl CanvasState {
    fn update(&mut self, gesture: Gesture, dom: Dom::Tag, key: KeyName) -> Vec<DomCmd> {
        match dom {
            Dom::ModalOpen  => { self.modal = true;  vec![DomCmd::new(Operation::OpenModal,  "modal",  None, None)] }
            Dom::ModalClose => { self.modal = false; vec![DomCmd::new(Operation::CloseModal, "modal",  None, None)] }
            Dom::DrawerOpen  => { self.drawer = true;  vec![DomCmd::new(Operation::OpenModal,  "drawer", None, None)] }
            Dom::DrawerClose => { self.drawer = false; vec![DomCmd::new(Operation::CloseModal, "drawer", None, None)] }
            // overlay 遷移はアプリ固有 — TODO
            _ => vec![],
        }
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
    log_stack:     Vec<Log>,
    character:     Instance,
}

#[wasm_bindgen]
impl App {
    pub fn init() -> App {
        App {
            pointer_state: PointerState::default(),
            canvas_state:  CanvasState::default(),
            dom_cmds:      Vec::new(),
            log_stack:     Vec::new(),
            character:     Instance::new(),
        }
    }

    pub fn flush(&mut self) -> JsValue {
        let out = serde_wasm_bindgen::to_value(&self.dom_cmds).unwrap_or(JsValue::NULL);
        self.dom_cmds.clear();
        out
    }

    pub fn event(&mut self, payload: JsValue) {
        let event_type = EventType::decode(&get_js_str(&payload, "event_type").unwrap_or_default());
        let id  = Dom::Id::decode(&get_js_str(&payload, "target_id").unwrap_or_default());
        let dom = id.last_tag().cloned().unwrap_or(Dom::Tag::Other);
        let key        = KeyName::decode(&get_js_str(&payload, "key").unwrap_or_default());
        let x          = get_js_f64(&payload, "x").unwrap_or(0.0);
        let y          = get_js_f64(&payload, "y").unwrap_or(0.0);
        let time       = get_js_f64(&payload, "time").unwrap_or(0.0);

        self.pointer_state = self.pointer_state.update(&event_type, x, y, time);
        let gesture = detect_gesture(&self.pointer_state, time);

        let dom_cmds: Vec<DomCmd> = match event_type {
            EventType::Submit if matches!(dom, Dom::ChatForm) => {
                let fields = get_js_field(&payload, "value").unwrap_or(JsValue::NULL);
                let text = get_js_str(&fields, "text").unwrap_or_default();
                self.on_chat_submit(&text)
            }
            EventType::Submit if matches!(dom, Dom::CharEditForm) => {
                let fields = get_js_field(&payload, "value").unwrap_or(JsValue::NULL);
                self.on_char_edit_save(&fields)
            }
            EventType::Input if matches!(dom, Dom::ChatInput) => {
                let value = get_js_str(&payload, "value").unwrap_or_default();
                self.on_chat_input(&value)
            }
            EventType::KeyDown => self.on_keydown(key),
            EventType::Click   => self.on_click(dom),
            EventType::FocusIn => { self.on_focus(dom); vec![] }
            EventType::PointerDown
            | EventType::PointerMove
            | EventType::PointerUp
            | EventType::PointerCancel => self.on_gesture(gesture, dom, key),
            _ => vec![],
        };

        self.dom_cmds.extend(dom_cmds);
    }

    fn on_gesture(&mut self, gesture: Option<Gesture>, dom: Dom::Tag, key: KeyName) -> Vec<DomCmd> {
        match gesture {
            Some(g) => self.canvas_state.update(g, dom, key),
            None    => vec![],
        }
    }
}