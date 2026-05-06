use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use crate::Lang;
use crate::data_struct::DataStruct;
use crate::Roll;
use crate::js_client::{
    Operation, DomCmd,
    get_js_str, get_js_f64, 
    EventType, KeyName,
    Gesture, PointerState, detect_gesture,
    Dom,
};
use crate::event;

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
    drawer: bool,     // true = open, false = close
    modal: bool,      // true = open, false = close
}

impl CanvasState {
    fn update(&mut self, gesture: Gesture, dom: Dom::Id, key: KeyName) -> Vec<DomCmd> {
        if (self.modal) {
            // todo: modal open時のclose処理
        }
        if (self.drawer) {
            // todo: drawer open時のclose処理
        }
        match dom {
            // todo: overlay 遷移
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
        let id         = Dom::Id::decode(&get_js_str(&payload, "target_id").unwrap_or_default());
        let key        = KeyName::decode(&get_js_str(&payload, "key").unwrap_or_default());
        let x          = get_js_f64(&payload, "x").unwrap_or(0.0);
        let y          = get_js_f64(&payload, "y").unwrap_or(0.0);
        let time       = get_js_f64(&payload, "time").unwrap_or(0.0);

        self.pointer_state = self.pointer_state.update(&event_type, x, y, time);
        let gesture = detect_gesture(&self.pointer_state, time);

        match gesture {
            Some(g) => self.canvas_state.update(g, id, key),
            None    => vec![],
        }
    }
}