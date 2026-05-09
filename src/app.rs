use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsValue, to_value};
use crate::js_client::{
    CanvasCmd, Operation,
    get_js_str, get_js_f64,
    EventType, KeyName,
    Gesture, PointerState, detect_gesture,
    Device, detect_device,
    dom,
};
use crate::Lang;
use crate::data_struct::{DataStruct, Id};
use crate::wal::WalStore;
use crate::character::CHARACTER_SCHEMA_NAME;
use crate::event;

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
    dialog:    Dialog,
    lang:      Lang,
    character: DataStruct,  // 編集中バッファ。character.identity=0は未保存
}

impl CanvasState {
    fn new() -> Self {
        Self { dialog: Dialog::default(), lang: Lang::Ja, character: DataStruct::new() }
    }
}

// ============================================================
// app
// ============================================================

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
    characters:    WalStore,
    canvas_state:  CanvasState,
    events:        Vec<CanvasEvent>,
    cmds:          Vec<CanvasCmd>,
    log_stack:     Vec<event::LogStack>,
}

impl App {
    fn load(&mut self, id: u32) {

    }

    fn save(&mut self) {

    }
}

#[wasm_bindgen]
impl App {
    pub async fn init(screen_width: u32, pointer_coarse: bool) -> (App, JsValue) {
        let device = detect_device(screen_width, pointer_coarse);
        let characters = WalStore::open(CHARACTER_SCHEMA_NAME).await
            .unwrap_or_else(|e| panic!("WalStore::open failed: {}", e));

        let mut app = App {
            device,
            pointer_state: PointerState::default(),
            characters,
            canvas_state:  CanvasState::new(),
            events:        Vec::new(),
            cmds:          Vec::new(),
            log_stack:     Vec::new(),
        };

        let cmds = to_value(&app.cmds).unwrap_or(JsValue::NULL);
        app.cmds.clear();
        (app, cmds)
    }

    /// JS 側からイベントを受け取り、CanvasCmd のリストを返す。
    pub fn event(&mut self, payload: JsValue) -> JsValue {
        self.events.push(CanvasEvent::decode(&payload));
        while let Some(canvas_event) = self.events.pop() {
            let cmds = self.dispatch(canvas_event);
            self.cmds.extend(cmds);
        }
        let out = to_value(&self.cmds).unwrap_or(JsValue::NULL);
        self.cmds.clear();
        out
    }

    /// CanvasEvent をディスパッチして CanvasCmd のリストを返す。
    fn dispatch(&mut self, canvas_event: CanvasEvent) -> Vec<CanvasCmd> {
        todo!("ev.event_type に応じて canvas_state.on_* を呼び分け、CanvasCmd を返す")
    }
}
