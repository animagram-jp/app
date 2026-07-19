use core::{primitive::{f64, bool}, option::Option::{Some, None}};
use alloc::vec::Vec;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use serde::Serialize;
use serde_wasm_bindgen::Serializer;
use crate::js_client::{Command, EventType, Device, detect_device, PointerState, detect_gesture, CanvasEvent};
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
            Some(gesture) => self.events.push(Event::Gesture(gesture)),
            None => match &canvas_event.event_type {
                EventType::PointerDown => self.events.push(Event::Canvas(canvas_event)),
                EventType::PointerMove |
                EventType::PointerUp   | EventType::PointerCancel => {},
                _ => self.events.push(Event::Canvas(canvas_event)),
            },
        }
        while let Some(event) = self.events.pop() {
            let (new_events, new_commands) = self.dispatch(event);
            self.events.extend(new_events);
            commands.extend(new_commands);
        }
        let serializer = Serializer::new().serialize_maps_as_objects(true);
        commands.serialize(&serializer).unwrap_or(JsValue::NULL)
    }

    fn dispatch(&mut self, event: Event) -> (Vec<Event>, Vec<Command>) {
        let Self { handler, pointer_state, .. } = self;
        match event {
            Event::Ready             => handler.initial_draw(),
            Event::Canvas(e)         => handler.process(&e, pointer_state),
            Event::Gesture(g)        => handler.process_gesture(&g, pointer_state),
        }
    }
}
