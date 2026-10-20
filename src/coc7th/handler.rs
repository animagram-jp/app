use alloc::{string::String, vec, vec::Vec};
#[cfg(feature = "file_store")]
use core::option::Option::None;
use core::{
    matches,
    option::Option::Some,
    primitive::{f64, i32, u8, u32},
};

use arbitrary_int::u2;

#[cfg(feature = "file_store")]
use crate::{
    Error,
    event::{Opened, store},
    file_store::{Backend, FileStore, StoreId},
};
use crate::{
    Lang,
    coc7th::{
        character::SCHEMA_SIZE,
        target::{Sheet, Target},
    },
    data_struct::DataStruct,
    event::{Event, gesture::Pointer},
    host::{Attribute, CanvasEvent, Command, EventType},
};

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
    character_sheet: Sheet,
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
    #[cfg(feature = "file_store")]
    pub fn save(&mut self) -> (Vec<Event>, Vec<Command>) {
        let Some(characters) = self.characters.as_mut() else {
            return (vec![], vec![]);
        };
        match characters.save() {
            Ok(()) => (vec![], vec![]),
            Err(error) => store::failed(CHARACTER_STORE, error),
        }
    }
}

impl crate::event::Handler for Handler {
    async fn ready(
        _viewport_width: f64,
        _viewport_height: f64,
        _rem_in_px: f64,
        _now: f64,
        _timezone_offset_minutes: i32,
    ) -> Self {
        #[cfg_attr(not(feature = "file_store"), allow(unused_mut))]
        let mut handler = Self {
            character_sheet: Sheet::Immutable,
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
        match store::open(CHARACTER_STORE).await {
            Ok(store) => handler.characters = Some(store),
            Err(error) => handler.startup.push(Error::FileStore(error)),
        }
        handler
    }

    fn close(&self) -> Vec<Command> {
        #[cfg(feature = "file_store")]
        if let Some(store) = &self.characters {
            store.close();
        }
        vec![]
    }

    #[cfg(feature = "file_store")]
    fn process_lost(&mut self) -> (Vec<Event>, Vec<Command>) {
        self.lost = store::lost(&mut self.characters);
        (vec![], vec![])
    }

    #[cfg(feature = "file_store")]
    fn process_opened(&mut self, opened: Opened) -> (Vec<Event>, Vec<Command>) {
        store::opened(&mut self.characters, &mut self.lost, opened)
    }

    fn initial_draw(&mut self) -> (Vec<Event>, Vec<Command>) {
        let mut commands = Vec::new();
        #[cfg(feature = "file_store")]
        commands.extend(self.startup.drain(..).map(|error| Command::Error { error }));
        commands.push(Command::RemoveAttribute {
            id:        Target::Body.to_dom(),
            attribute: Attribute::Hidden,
        });
        (vec![], commands)
    }

    fn process_canvas(
        &mut self,
        event: &CanvasEvent,
        _pointer: &Pointer,
    ) -> (Vec<Event>, Vec<Command>) {
        let target = Target::from_dom(&event.id);
        if !matches!(event.event_type, EventType::Click) || target != Some(Target::Edit) {
            return (vec![], vec![]);
        }

        let hidden = self.character_sheet;
        let shown = match hidden {
            Sheet::Immutable => Sheet::Editable,
            Sheet::Editable => Sheet::Immutable,
        };
        self.character_sheet = shown;
        let commands = vec![
            Command::RemoveAttribute {
                id:        Target::Sheet(shown).to_dom(),
                attribute: Attribute::Hidden,
            },
            Command::SetAttribute {
                id:        Target::Sheet(hidden).to_dom(),
                attribute: Attribute::Hidden,
                value:     String::new(),
            },
        ];
        (vec![], commands)
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use alloc::vec::Vec;

    use wasm_bindgen_test::*;

    use crate::{
        app::App,
        arena::{ARENA, COMMAND_RING, EVENT_RING, initialize, process_event},
        coc7th::target::Target,
        host::{
            EVENT_CANVAS, EventType, OPERATION_REMOVE_ATTRIBUTE, OPERATION_SET_ATTRIBUTE, Output,
        },
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
        Target::Edit.to_dom().encode(&mut frame);
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
    use crate::{
        block_on,
        event::Handler as _,
        host::{KeyName, dom},
    };

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
        Target::Edit.to_dom()
    }

    fn hidden_change(command: &Command) -> (bool, dom::Id) {
        match command {
            Command::SetAttribute { id, attribute: Attribute::Hidden, .. } => (true, id.clone()),
            Command::RemoveAttribute { id, attribute: Attribute::Hidden } => (false, id.clone()),
            _ => panic!("unexpected command"),
        }
    }

    fn sheet(sheet: Sheet) -> dom::Id {
        Target::Sheet(sheet).to_dom()
    }

    #[test]
    fn the_first_click_shows_the_edit_section_and_the_second_returns_to_the_view() {
        let mut handler = block_on(Handler::ready(0.0, 0.0, 16.0, 0.0, 0));

        let first: Vec<_> =
            click(&mut handler, toggle_button()).iter().map(hidden_change).collect();
        assert_eq!(first, [(false, sheet(Sheet::Editable)), (true, sheet(Sheet::Immutable))]);

        let second: Vec<_> =
            click(&mut handler, toggle_button()).iter().map(hidden_change).collect();
        assert_eq!(second, [(false, sheet(Sheet::Immutable)), (true, sheet(Sheet::Editable))]);
    }

    #[test]
    fn the_initial_draw_reveals_the_body() {
        let mut handler = block_on(Handler::ready(0.0, 0.0, 16.0, 0.0, 0));
        let (events, commands) = handler.initial_draw();
        assert!(events.is_empty());
        assert_eq!(
            commands.iter().map(hidden_change).collect::<Vec<_>>(),
            [(false, Target::Body.to_dom())]
        );
    }

    #[test]
    fn other_targets_and_events_change_nothing() {
        let mut handler = block_on(Handler::ready(0.0, 0.0, 16.0, 0.0, 0));
        assert!(click(&mut handler, sheet(Sheet::Immutable)).is_empty());
        let first: Vec<_> =
            click(&mut handler, toggle_button()).iter().map(hidden_change).collect();
        assert_eq!(first, [(false, sheet(Sheet::Editable)), (true, sheet(Sheet::Immutable))]);
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
