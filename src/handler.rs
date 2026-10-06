use alloc::{string::String, vec, vec::Vec};
use core::{
    matches,
    option::Option::{self, None, Some},
    primitive::{f64, i32, u8, u32},
};

use arbitrary_int::u2;

#[cfg(feature = "worker")]
use crate::{Error, file_store::FileStore};
use crate::{
    Lang,
    data_struct::DataStruct,
    event::{Event, Response},
    js_client::{
        Attribute, CanvasEvent, Command, EventType, Gesture, PointerState, VisibilityState, dom,
    },
};

#[cfg(feature = "worker")]
const RETRY_LIMIT: u8 = 3;

pub enum CharacterSheet {
    Immutable,
    Editable,
}

pub enum Dialog {
    None,
    Drawer,
    Select { step: u8, index: u32 },
    Input { step: u8, value: u32 },
}

pub struct Log;

#[cfg(feature = "worker")]
const CHARACTER_SCHEMA_NAME: &str = "characters";

pub struct Handler {
    character_sheet: CharacterSheet,
    dialog:          Dialog,
    lang:            Lang,
    last_toast:      u2,
    character:       DataStruct,
    #[cfg(feature = "worker")]
    characters:      FileStore,
    logs:            Vec<Log>,
    #[cfg(feature = "worker")]
    store_failures:  u8,
}

impl Handler {
    pub async fn ready(
        _viewport_width: f64,
        _viewport_height: f64,
        _rem_in_px: f64,
        _now: f64,
        _timezone_offset_minutes: i32,
    ) -> Self {
        Self {
            character_sheet: CharacterSheet::Immutable,
            dialog: Dialog::None,
            lang: Lang::Ja,
            last_toast: u2::new(1),
            character: DataStruct::new(0, 0.0, 256),
            #[cfg(feature = "worker")]
            characters: FileStore::new(CHARACTER_SCHEMA_NAME)
                .await
                .unwrap_or_else(|e| panic!("FileStore::new failed: {e}")),
            logs: Vec::new(),
            #[cfg(feature = "worker")]
            store_failures: 0,
        }
    }

    pub fn close(&self) -> Vec<Command> {
        #[cfg(feature = "worker")]
        self.characters.close();
        vec![]
    }

    #[cfg(feature = "worker")]
    pub fn save(&mut self) -> Vec<Command> {
        match self.characters.save() {
            Ok(()) => {
                self.store_failures = 0;
                vec![]
            }
            Err(_) if self.store_failures + 1 < RETRY_LIMIT => {
                self.store_failures += 1;
                vec![]
            }
            Err(e) => {
                self.store_failures = 0;
                vec![Command::Error { error: Error::FileStore(e) }]
            }
        }
    }

    pub fn process_resize(&mut self, _width: f64, _height: f64) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    pub fn process_scroll(&mut self, _x: f64, _y: f64) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    pub fn process_visibility(&mut self, _state: VisibilityState) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    pub fn process_fetched(&mut self, _response: &Response) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    pub fn initial_draw(&mut self) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    pub fn process_canvas(
        &mut self,
        event: &CanvasEvent,
        _state: &PointerState,
    ) -> (Vec<Event>, Vec<Command>) {
        let toggle = dom::Id::new(&[(dom::Tag::Header, None), (dom::Tag::Button, Some(3))]);
        if !matches!(event.event_type, EventType::Click) || event.id != toggle {
            return (vec![], vec![]);
        }

        let section = |n| dom::Id::new(&[(dom::Tag::Main, None), (dom::Tag::Section, Some(n))]);
        let (shown, hidden) = match self.character_sheet {
            CharacterSheet::Immutable => {
                self.character_sheet = CharacterSheet::Editable;
                (2, 1)
            }
            CharacterSheet::Editable => {
                self.character_sheet = CharacterSheet::Immutable;
                (1, 2)
            }
        };
        let commands = vec![
            Command::RemoveAttribute { id: section(shown), attribute: Attribute::Hidden },
            Command::SetAttribute {
                id:        section(hidden),
                attribute: Attribute::Hidden,
                value:     String::new(),
            },
        ];
        (vec![], commands)
    }

    pub fn process_gesture(
        &mut self,
        _gesture: &Gesture,
        _state: &PointerState,
        _origin: Option<&CanvasEvent>,
    ) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use alloc::vec::Vec;

    use wasm_bindgen_test::*;

    use crate::{
        app::App,
        arena::{ARENA, COMMAND_RING, EVENT_RING, initialize, process_event},
        event::EVENT_CANVAS,
        js_client::{EventType, OPERATION_REMOVE_ATTRIBUTE, OPERATION_SET_ATTRIBUTE, Output, dom},
    };

    const CLICK: u8 = 2;

    #[wasm_bindgen_test]
    async fn process_event_emits_one_frame_per_command() {
        assert_eq!(EventType::decode_u8(CLICK), EventType::Click);
        initialize();
        App::init(false, 0.0, 0.0, 16.0, 0.0, 0).await;

        let mut frame = Vec::new();
        frame.push(EVENT_CANVAS);
        frame.push(CLICK);
        dom::Id::new(&[(dom::Tag::Header, None), (dom::Tag::Button, Some(3))]).encode(&mut frame);
        frame.push(0);
        frame.push(0);
        str::encode("", &mut frame);
        0.0f32.encode(&mut frame);
        0.0f32.encode(&mut frame);
        0.0f32.encode(&mut frame);
        0.0f32.encode(&mut frame);
        frame.extend_from_slice(&0f64.to_le_bytes());
        frame.extend_from_slice(&0u32.to_le_bytes());
        assert!(ARENA.write_ring(EVENT_RING, &frame));

        process_event();

        let mut operations = Vec::new();
        while let Some(command) = ARENA.read_ring(COMMAND_RING) {
            operations.push(command[0]);
            ARENA.advance_ring(COMMAND_RING);
        }
        assert_eq!(operations, [OPERATION_REMOVE_ATTRIBUTE, OPERATION_SET_ATTRIBUTE]);
    }
}

#[cfg(all(test, not(feature = "worker")))]
mod toggle_tests {
    use alloc::{string::String, vec::Vec};
    use core::{
        future::Future,
        pin::pin,
        task::{Context, Poll, Waker},
    };

    use super::*;
    use crate::js_client::KeyName;

    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
                return output;
            }
        }
    }

    fn click(handler: &mut Handler, id: dom::Id) -> Vec<Command> {
        let event = CanvasEvent {
            event_type: EventType::Click,
            id,
            key: KeyName::Other,
            flags: 0,
            value: String::new(),
            x: 0.0,
            y: 0.0,
            local_x: 0.0,
            local_y: 0.0,
            time: 0.0,
            pointer_id: 0,
        };
        handler.process_canvas(&event, &PointerState::default()).1
    }

    fn toggle_button() -> dom::Id {
        dom::Id::new(&[(dom::Tag::Header, None), (dom::Tag::Button, Some(3))])
    }

    fn hidden_change(command: &Command) -> (bool, dom::Id) {
        match command {
            Command::SetAttribute { id, attribute: Attribute::Hidden, .. } => (true, id.clone()),
            Command::RemoveAttribute { id, attribute: Attribute::Hidden } => (false, id.clone()),
            _ => panic!("unexpected command"),
        }
    }

    fn section(n: u32) -> dom::Id {
        dom::Id::new(&[(dom::Tag::Main, None), (dom::Tag::Section, Some(n))])
    }

    #[test]
    fn the_first_click_shows_the_edit_section_and_the_second_returns_to_the_view() {
        let mut handler = block_on(Handler::ready(0.0, 0.0, 16.0, 0.0, 0));

        let first: Vec<_> =
            click(&mut handler, toggle_button()).iter().map(hidden_change).collect();
        assert_eq!(first, [(false, section(2)), (true, section(1))]);

        let second: Vec<_> =
            click(&mut handler, toggle_button()).iter().map(hidden_change).collect();
        assert_eq!(second, [(false, section(1)), (true, section(2))]);
    }

    #[test]
    fn other_targets_and_events_change_nothing() {
        let mut handler = block_on(Handler::ready(0.0, 0.0, 16.0, 0.0, 0));
        assert!(click(&mut handler, section(1)).is_empty());
        let first: Vec<_> =
            click(&mut handler, toggle_button()).iter().map(hidden_change).collect();
        assert_eq!(first, [(false, section(2)), (true, section(1))]);
    }
}
