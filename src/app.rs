use core::option::Option::{self, Some, None};
use alloc::vec::Vec;
use wasm_bindgen::prelude::{wasm_bindgen, *};
use wasm_bindgen::JsValue;
use serde_wasm_bindgen::to_value;
use crate::js_client::{Command, get_js_str, get_js_f64, EventType, Device, detect_device, Gesture, PointerState, detect_gesture, dom, CanvasEvent};
use crate::event::{Handler, Event};

// ============================================================
// App
// ============================================================

#[wasm_bindgen]
pub struct App {
    device:        Device,
    pointer_state: PointerState,
    events:        Vec<Event>,
    handler:       Handler,
}

#[wasm_bindgen]
impl App {
    pub async fn init(pointer_coarse: bool, viewport_width: f64, viewport_height: f64) -> App {
        let device = detect_device(pointer_coarse);

        let mut app = App {
            device,
            pointer_state: PointerState::default(),
            events:        Vec::new(),
            handler:       Handler::ready(viewport_width, viewport_height).await,
        };

        app.events.push(Event::Ready);
        app
    }

    pub fn close(&self) {
        self.handler.close();
    }

    pub fn process(&mut self, payload: JsValue) -> JsValue {
        let mut commands = Vec::new();
        let canvas_event = CanvasEvent::decode(&payload);
        let prev_state = self.pointer_state;
        self.pointer_state = self.pointer_state.update(
            &canvas_event.event_type,
            canvas_event.x, canvas_event.y, canvas_event.time,
        );
        match detect_gesture(&mut self.pointer_state, &prev_state, &canvas_event.event_type, canvas_event.time) {
            Some(gesture) => {
                self.events.push(Event::Gesture(gesture));
            }
            None => match &canvas_event.event_type {
                EventType::PointerDown => self.events.push(Event::Canvas(canvas_event)),
                EventType::PointerMove |
                EventType::PointerUp   | EventType::PointerCancel => {},
                _ => self.events.push(Event::Canvas(canvas_event)),
            },
        }
        while let Some(event) = self.events.pop() {
            commands.extend(self.dispatch(event));
        }
        to_value(&commands).unwrap_or(JsValue::NULL)
    }

    fn dispatch(&mut self, event: Event) -> Vec<Command> {
        match event {
            Event::Ready => {
                self.handler.initial_draw()
            }
            Event::Canvas(canvas_event) => {
                self.handler.process(&canvas_event)
            }
            Event::Gesture(gesture) => {
                self.handler.process_gesture(&gesture)
            }
        }
    }
}
