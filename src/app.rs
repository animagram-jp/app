use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use serde_wasm_bindgen::to_value;
use crate::js_client::{CanvasCmd, get_js_str, get_js_f64, EventType, Device, detect_device, Gesture, PointerState, detect_gesture, dom, CanvasEvent};
use crate::event::{CanvasState, Coc7th};

// ============================================================
// Event
// ============================================================

pub enum Event {
    Ready,
    Canvas(CanvasEvent),
    Gesture(Gesture),
}

// ============================================================
// App
// ============================================================

#[wasm_bindgen]
pub struct App {
    device:        Device,
    pointer_state: PointerState,
    canvas_state:  CanvasState,
    handler:       Coc7th,
    events:        Vec<Event>,
    commands:      Vec<CanvasCmd>,
}

#[wasm_bindgen]
impl App {
    pub async fn init(screen_width: u32, pointer_coarse: bool) -> App {
        let device = detect_device(screen_width, pointer_coarse);

        let mut app = App {
            device,
            pointer_state: PointerState::default(),
            canvas_state:  CanvasState::new(),
            handler:       Coc7th::ready().await,
            events:        Vec::new(),
            commands:          Vec::new(),
        };

        app.events.push(Event::Ready);
        app
    }

    pub fn process(&mut self, payload: JsValue) -> JsValue {
        let canvas_event = CanvasEvent::decode(&payload);
        self.pointer_state = self.pointer_state.update(
            &canvas_event.event_type,
            canvas_event.x, canvas_event.y, canvas_event.time,
        );
        match detect_gesture(&self.pointer_state, &canvas_event.event_type, canvas_event.time) {
            Some(gesture) => self.events.push(Event::Gesture(gesture)),
            None => match &canvas_event.event_type {
                EventType::PointerDown | EventType::PointerMove |
                EventType::PointerUp   | EventType::PointerCancel => {}, // 正しいのか要確認
                _ => self.events.push(Event::Canvas(canvas_event)),
            },
        }
        while let Some(event) = self.events.pop() { // 必要そうならtimeoutやlimitを設ける
            let commands = self.dispatch(event);
            self.commands.extend(commands);
        }
        let out = to_value(&self.commands).unwrap_or(JsValue::NULL);
        self.commands.clear();
        out
    }

    fn dispatch(&mut self, event: Event) -> Vec<CanvasCmd> {
        match event {
            Event::Ready => {
                // event::initial_draw(&mut self.canvas_state, &self.handler)
            }
            Event::Canvas(canvas_event) => {
                event::handle(&mut self.canvas_state, &canvas_event, &mut self.handler)
            }
            Event::Gesture(gesture) => {
                event::handle_gesture(gesture, &mut self.canvas_state, &mut self.handler)
            }
        }
    }
}
