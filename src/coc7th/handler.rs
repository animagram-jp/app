use alloc::{string::String, vec, vec::Vec};
use core::{
    matches,
    option::Option::{self, None, Some},
    primitive::{f64, i32, u8, u32},
};

use arbitrary_int::u2;

#[cfg(feature = "file_store")]
use crate::{
    Error,
    event::Opened,
    file_store::{Backend, FileStore, StoreId},
};
use crate::{
    Lang,
    coc7th::character::SCHEMA_SIZE,
    data_struct::DataStruct,
    event::{Event, Response},
    js_client::{
        Attribute, CanvasEvent, Command, EventType, FullscreenEvent, Gesture, Pointer,
        VisibilityState, dom,
    },
};

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

#[cfg(feature = "file_store")]
const CHARACTER_STORE: StoreId = StoreId { name: "characters", version: "0.0" };

pub struct Handler {
    character_sheet: CharacterSheet,
    dialog:          Dialog,
    lang:            Lang,
    last_toast:      u2,
    character:       DataStruct,
    pub now:         f64,
    #[cfg(feature = "file_store")]
    characters:      Option<Backend>,
    #[cfg(feature = "file_store")]
    lost:            Vec<(u32, Option<Vec<u8>>)>,
    #[cfg(feature = "file_store")]
    startup:         Vec<Error>,
    logs:            Vec<Log>,
}

impl Handler {
    pub async fn ready(
        _viewport_width: f64,
        _viewport_height: f64,
        _rem_in_px: f64,
        _now: f64,
        _timezone_offset_minutes: i32,
    ) -> Self {
        #[cfg_attr(not(feature = "file_store"), allow(unused_mut))]
        let mut handler = Self {
            character_sheet: CharacterSheet::Immutable,
            dialog: Dialog::None,
            lang: Lang::Ja,
            last_toast: u2::new(1),
            character: DataStruct::new(0, 0.0, SCHEMA_SIZE),
            now: 0.0,
            #[cfg(feature = "file_store")]
            characters: None,
            #[cfg(feature = "file_store")]
            lost: Vec::new(),
            #[cfg(feature = "file_store")]
            startup: Vec::new(),
            logs: Vec::new(),
        };
        #[cfg(feature = "file_store")]
        match Backend::open(CHARACTER_STORE, true).await.and_then(Backend::new) {
            Ok(store) => handler.characters = Some(store),
            Err(error) => handler.startup.push(Error::FileStore(error)),
        }
        handler
    }

    pub fn close(&self) -> Vec<Command> {
        #[cfg(feature = "file_store")]
        if let Some(store) = &self.characters {
            store.close();
        }
        vec![]
    }

    #[cfg(feature = "file_store")]
    pub fn save(&mut self) -> (Vec<Event>, Vec<Command>) {
        let Some(store) = self.characters.as_mut() else {
            return (vec![], vec![]);
        };
        match store.save() {
            Ok(()) => (vec![], vec![]),
            Err(e @ crate::file_store::FileStoreError::InvalidState(_)) => (
                vec![Event::StoreLost(CHARACTER_STORE)],
                vec![Command::Error { error: Error::FileStore(e) }],
            ),
            Err(e) => (vec![], vec![Command::Error { error: Error::FileStore(e) }]),
        }
    }

    #[cfg(feature = "file_store")]
    pub fn process_lost(&mut self) -> (Vec<Event>, Vec<Command>) {
        if let Some(store) = self.characters.take() {
            self.lost = store.pending();
            store.close();
        }
        (vec![], vec![])
    }

    #[cfg(feature = "file_store")]
    pub fn process_opened(&mut self, opened: Opened) -> (Vec<Event>, Vec<Command>) {
        match opened.and_then(Backend::new) {
            Ok(mut store) => {
                store.replay(core::mem::take(&mut self.lost));
                self.characters = Some(store);
                (vec![], vec![])
            }
            Err(error) => {
                (vec![], vec![Command::Error { error: Error::FileStore(error) }, Command::Reload])
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

    pub fn process_fullscreen(&mut self, _event: FullscreenEvent) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    pub fn process_fetched(&mut self, _response: &Response) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    pub fn initial_draw(&mut self) -> (Vec<Event>, Vec<Command>) {
        let mut commands = Vec::new();
        #[cfg(feature = "file_store")]
        commands.extend(self.startup.drain(..).map(|error| Command::Error { error }));
        commands.push(Command::RemoveAttribute {
            id:        dom::Id::new(&[(dom::Tag::Body, None)]),
            attribute: Attribute::Hidden,
        });
        (vec![], commands)
    }

    pub fn process_canvas(
        &mut self,
        event: &CanvasEvent,
        _pointer: &Pointer,
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
        _pointer: &Pointer,
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

    const CLICK: u8 = 3;

    #[wasm_bindgen_test]
    async fn process_event_emits_one_frame_per_command() {
        assert_eq!(EventType::from_u8(CLICK), EventType::Click);
        initialize();
        App::init(false, 0.0, 0.0, 16.0, 0.0, 0).await;
        while ARENA.read_ring(COMMAND_RING).is_some() {
            ARENA.advance_ring(COMMAND_RING);
        }

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

#[cfg(test)]
mod toggle_tests {
    use alloc::{string::String, vec::Vec};

    use super::*;
    use crate::{block_on, js_client::KeyName};

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
        handler.process_canvas(&event, &Pointer::default()).1
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
    fn the_initial_draw_reveals_the_body() {
        let mut handler = block_on(Handler::ready(0.0, 0.0, 16.0, 0.0, 0));
        let (events, commands) = handler.initial_draw();
        assert!(events.is_empty());
        assert_eq!(
            commands.iter().map(hidden_change).collect::<Vec<_>>(),
            [(false, dom::Id::new(&[(dom::Tag::Body, None)]))]
        );
    }

    #[test]
    fn other_targets_and_events_change_nothing() {
        let mut handler = block_on(Handler::ready(0.0, 0.0, 16.0, 0.0, 0));
        assert!(click(&mut handler, section(1)).is_empty());
        let first: Vec<_> =
            click(&mut handler, toggle_button()).iter().map(hidden_change).collect();
        assert_eq!(first, [(false, section(2)), (true, section(1))]);
    }

    #[cfg(feature = "file_store")]
    #[test]
    fn closing_the_handler_releases_the_store() {
        let mut handler = block_on(Handler::ready(0.0, 0.0, 16.0, 0.0, 0));
        assert!(handler.save().1.is_empty());
        assert!(handler.close().is_empty());
        let (events, commands) = handler.save();
        assert!(matches!(events.as_slice(), [Event::StoreLost(_)]));
        assert!(matches!(
            commands.as_slice(),
            [Command::Error {
                error: Error::FileStore(crate::file_store::FileStoreError::InvalidState(_)),
            }]
        ));
    }

    #[cfg(feature = "file_store")]
    #[test]
    fn a_lost_store_is_replaced_and_the_pending_change_survives() {
        let mut handler = block_on(Handler::ready(0.0, 0.0, 16.0, 0.0, 0));
        handler.characters.as_mut().unwrap().set(1, b"kept".to_vec());
        let disk = block_on(Backend::open(CHARACTER_STORE, true)).unwrap();

        handler.close();
        let (events, _) = handler.save();
        assert!(matches!(events.as_slice(), [Event::StoreLost(_)]));
        handler.process_lost();
        handler.process_opened(Ok(disk.clone()));
        assert_eq!(handler.characters.as_ref().unwrap().get(1), Some(&b"kept"[..]));
        assert!(handler.save().1.is_empty());
        assert_eq!(Backend::new(disk).unwrap().get(1), Some(&b"kept"[..]));
    }
}
