use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use crate::js_client::{
    CanvasCmd, Operation,
    get_js_str, get_js_f64,
    EventType, KeyName,
    Gesture, PointerState, detect_gesture,
    Device, detect_device,
    dom,
};
use crate::Lang;
const LANG: Lang = Lang::Ja;
use crate::data_struct::{DataStruct, Id};
use crate::wal::WalStore;
use crate::character::SCHEMA_NAME;
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
    dialog: Dialog,
    buf:    DataStruct,  // 編集中バッファ。buf.identity=0は未保存
}

impl CanvasState {
    fn new() -> Self {
        Self { dialog: Dialog::default(), buf: DataStruct::new() }
    }

    // bufのフィールドをクリアしつつidentityは保持する
    fn discard_buffer(&mut self) {
        let id = self.buf.identity;
        self.buf = DataStruct::new();
        self.buf.identity = id;
    }

    fn on_click(&mut self, id: &dom::Id, _key: KeyName) -> Vec<CanvasCmd> {
        todo!()
    }

    fn on_click_normal(&mut self, id: &dom::Id, last_tag: Option<&dom::Tag>) -> Vec<CanvasCmd> {
        todo!()
    }

    fn on_keydown(&mut self, _id: &dom::Id, key: KeyName) -> Vec<CanvasCmd> {
        todo!()
    }

    fn on_input(&mut self, id: &dom::Id, value: &str) -> Vec<CanvasCmd> {
        todo!()
    }

    fn on_change(&mut self, id: &dom::Id, value: &str) -> Vec<CanvasCmd> {
        todo!()
    }

    fn on_blur(&mut self, id: &dom::Id, value: &str) -> Vec<CanvasCmd> {
        todo!()
    }

    fn on_gesture(&mut self, _gesture: Gesture, _id: &dom::Id) -> Vec<CanvasCmd> {
        todo!()
    }
}

// ============================================================
// App
// ============================================================

/// JS から受け取った生ペイロードをデコードした入力イベント。
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
        if let Some(raw) = self.characters.get(id as u64) {
            self.canvas_state.buf = DataStruct::from_bytes(&raw);
            self.canvas_state.buf.identity = Id(id);
        }
    }

    fn save(&mut self) {
        if self.canvas_state.buf.identity.get().is_none() {
            self.canvas_state.buf.identity = Id(self.characters.alloc() as u32);
        }
        let id = self.canvas_state.buf.identity;
        self.characters.set(id.0 as u64, &self.canvas_state.buf.to_bytes());
    }

    fn character_list(&self) -> Vec<(u32, String)> {
        use crate::character::{Character, Profile};
        self.characters.get_all().into_iter().map(|(id, raw)| {
            let id = id as u32;
            let ds = DataStruct::from_bytes(&raw);
            let name = ds.get(&Character::Profile(Profile::Name)).ok()
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| format!("#{}", id));
            (id, name)
        }).collect()
    }
}

#[wasm_bindgen]
impl App {
    pub async fn init(screen_width: u32, pointer_coarse: bool) -> App {
        let characters = WalStore::open(SCHEMA_NAME).await
            .unwrap_or_else(|e| panic!("WalStore::open failed: {}", e));

        let mut app = App {
            device:        detect_device(screen_width, pointer_coarse),
            pointer_state: PointerState::default(),
            characters,
            canvas_state:  CanvasState::new(),
            events:        Vec::new(),
            cmds:          Vec::new(),
            log_stack:     Vec::new(),
        };

        let list = app.character_list();
        if !list.is_empty() {
            app.load(list[0].0);
            app.cmds.extend(event::update_character_view(&app.canvas_state.buf));
        }
        app.cmds.extend(event::update_debug_select(&list, app.canvas_state.buf.identity.get()));
        app
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
        }
    }

    /// CanvasEvent をディスパッチして CanvasCmd のリストを返す。
    fn dispatch(&mut self, ev: CanvasEvent) -> Vec<CanvasCmd> {
        todo!("ev.event_type に応じて canvas_state.on_* を呼び分け、CanvasCmd を返す")
    }
}
