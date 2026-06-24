use core::option::Option::{self, Some, None};
use alloc::vec::Vec;
use wasm_bindgen::prelude::{wasm_bindgen, *};
use wasm_bindgen::JsValue;
use serde_wasm_bindgen::to_value;
use crate::js_client::{Command, get_js_str, get_js_f64, EventType, Device, detect_device, Gesture, PointerState, detect_gesture, dom, CanvasEvent};
use crate::event::Handler;

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
    events:        Vec<Event>,
    commands:      Vec<Command>,
    handler:       Handler,
}

#[wasm_bindgen]
impl App {
    pub async fn init(screen_width: u32, pointer_coarse: bool) -> App {
        let device = detect_device(screen_width, pointer_coarse);

        let mut app = App {
            device,
            pointer_state: PointerState::default(),
            events:        Vec::new(),
            commands:      Vec::new(),
            handler:       Handler::ready().await,
        };

        app.events.push(Event::Ready);
        app
    }

    pub fn close(&self) {
        self.handler.close();
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

    fn dispatch(&mut self, event: Event) -> Vec<Command> {
        match event {
            Event::Ready => {
                Handler::initial_draw()
            }
            Event::Canvas(canvas_event) => {
                self.handler.process(&canvas_event)
            }
            Event::Gesture(gesture) => {
                Handler::process_gesture(&gesture)
            }
        }
    }
}
