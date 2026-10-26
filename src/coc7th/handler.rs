use alloc::{string::String, vec, vec::Vec};
use core::{
    matches,
    option::Option::{None, Some},
    primitive::{f64, i32, u8, u32, u64},
};

use arbitrary_int::u2;

use crate::{
    Error, Language,
    coc7th::{
        character::SCHEMA_SIZE,
        target::{SHEET_PARTS, Sheet, Target},
    },
    data_struct::DataStruct,
    event::{self, Event, Opened, gesture::Pointer, store},
    file_store::{FileStore, StoreId},
    host::{Attribute, CanvasEvent, Command, EventType},
};

pub enum Dialog {
    None,
    Drawer,
    Select { step: u8, index: u32 },
    Input { step: u8, value: u32 },
}

pub struct Log;
const CHARACTER_STORE: StoreId = StoreId { name: "characters", version: "0.0" };

pub struct Handler<S: FileStore> {
    character_sheet: Sheet,
    dialog:          Dialog,
    language:        Language,
    last_toast:      u2,
    character:       DataStruct,
    pub now_in_ms:   u64,
    characters:      Option<S>,
    lost:            Vec<(u32, Option<Vec<u8>>)>,
    startup:         Vec<Error>,
    logs:            Vec<Log>,
}

impl<S: FileStore> Handler<S> {
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

impl<S: FileStore> event::Handler for Handler<S> {
    type Store = S;

    async fn ready(
        _viewport_width_in_px: f64,
        _viewport_height_in_px: f64,
        _rem_in_px: f64,
        _now_in_ms: u64,
        _timezone_offset_minutes: i32,
    ) -> Self {
        let mut handler = Self {
            character_sheet: Sheet::Immutable,
            dialog:          Dialog::None,
            language:        Language::Ja,
            last_toast:      u2::new(1),
            character:       DataStruct::new(0, 0, SCHEMA_SIZE),
            now_in_ms:       0,
            characters:      None,
            lost:            Vec::new(),
            startup:         Vec::new(),
            logs:            Vec::new(),
        };
        match store::open::<S>(CHARACTER_STORE).await {
            Ok(store) => handler.characters = Some(store),
            Err(error) => handler.startup.push(Error::FileStore(error)),
        }
        handler
    }

    fn close(&self) -> Vec<Command> {
        if let Some(store) = &self.characters {
            store.close();
        }
        vec![]
    }
    fn process_lost(&mut self) -> (Vec<Event>, Vec<Command>) {
        self.lost = store::lost(&mut self.characters);
        (vec![], vec![])
    }
    fn process_opened(&mut self, opened: Opened<S>) -> (Vec<Event>, Vec<Command>) {
        store::opened(&mut self.characters, &mut self.lost, opened)
    }

    fn initial_draw(&mut self) -> (Vec<Event>, Vec<Command>) {
        let mut commands = Vec::new();
        commands.extend(self.startup.drain(..).map(|error| Command::Error { error }));
        commands.extend(language_commands(self.language));
        commands.extend(Language::list().iter().flat_map(|&l| option_commands(l)));
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
        if !matches!(event.event_type, EventType::Click) {
            return (vec![], vec![]);
        }
        if let Some(Target::LanguageOption(l)) = target {
            self.language = l;
            return (vec![], Vec::from(language_commands(l)));
        }
        if target != Some(Target::Edit) {
            return (vec![], vec![]);
        }

        let hidden = self.character_sheet;
        let shown = match hidden {
            Sheet::Immutable => Sheet::Editable,
            Sheet::Editable => Sheet::Immutable,
        };
        self.character_sheet = shown;
        let mut commands = Vec::new();
        commands.extend((1..=SHEET_PARTS).map(|part| Command::RemoveAttribute {
            id:        Target::Sheet(shown, part).to_dom(),
            attribute: Attribute::Hidden,
        }));
        commands.extend((1..=SHEET_PARTS).map(|part| Command::SetAttribute {
            id:        Target::Sheet(hidden, part).to_dom(),
            attribute: Attribute::Hidden,
            value:     String::new(),
        }));
        (vec![], commands)
    }
}

fn language_commands(language: Language) -> [Command; 2] {
    let (tag, label) = language.display();
    [
        Command::SetAttribute {
            id:        Target::Html.to_dom(),
            attribute: Attribute::Lang,
            value:     tag.into(),
        },
        Command::SetText { id: Target::Language.to_dom(), value: label.into() },
    ]
}

fn option_commands(language: Language) -> [Command; 2] {
    let (tag, label) = language.display();
    let id = Target::LanguageOption(language).to_dom();
    [
        Command::SetAttribute {
            id:        id.clone(),
            attribute: Attribute::Lang,
            value:     tag.into(),
        },
        Command::SetText { id, value: label.into() },
    ]
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use alloc::vec::Vec;
    use core::iter::repeat_n;

    use wasm_bindgen_test::*;

    use crate::{
        Output,
        app::App,
        arena::{ARENA, COMMAND_RING, EVENT_RING, initialize, process_event},
        coc7th::target::{SHEET_PARTS, Target},
        host::{EVENT_CANVAS, EventType, OPERATION_REMOVE_ATTRIBUTE, OPERATION_SET_ATTRIBUTE},
    };

    const CLICK: u8 = 3;

    #[wasm_bindgen_test]
    async fn process_event_emits_one_frame_per_command() {
        assert_eq!(EventType::from_u8(CLICK), EventType::Click);
        initialize();
        App::init(false, 0.0, 0.0, 16.0, 0, 0).await;
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
        let parts = SHEET_PARTS as usize;
        let expected: Vec<_> = repeat_n(OPERATION_REMOVE_ATTRIBUTE, parts)
            .chain(repeat_n(OPERATION_SET_ATTRIBUTE, parts))
            .collect();
        assert_eq!(operations, expected);
    }
}

#[cfg(test)]
mod toggle_tests {
    use alloc::{string::String, vec::Vec};

    use super::*;
    use crate::{
        En, block_on,
        event::Handler as _,
        file_store::{FileStoreError, MemoryStore},
        host::{KeyName, dom},
    };

    fn click(handler: &mut Handler<MemoryStore>, id: dom::Id) -> Vec<Command> {
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

    fn written(command: &Command) -> (dom::Id, Option<Attribute>, String) {
        match command {
            Command::SetText { id, value } => (id.clone(), None, value.clone()),
            Command::SetAttribute { id, attribute, value } => {
                (id.clone(), Some(*attribute), value.clone())
            }
            _ => panic!("unexpected command"),
        }
    }

    fn labelled(
        target: Target,
        root: Target,
        language: Language,
    ) -> Vec<(dom::Id, Option<Attribute>, String)> {
        let (tag, label) = language.display();
        Vec::from([
            (root.to_dom(), Some(Attribute::Lang), String::from(tag)),
            (target.to_dom(), None, String::from(label)),
        ])
    }

    fn toggled(shown: Sheet, hidden: Sheet) -> Vec<(bool, dom::Id)> {
        let change = |sheet, is_hidden| {
            (1..=SHEET_PARTS).map(move |part| (is_hidden, Target::Sheet(sheet, part).to_dom()))
        };
        change(shown, false).chain(change(hidden, true)).collect()
    }

    #[test]
    fn the_first_click_shows_the_edit_section_and_the_second_returns_to_the_view() {
        let mut handler = block_on(Handler::<MemoryStore>::ready(0.0, 0.0, 16.0, 0, 0));

        let first: Vec<_> =
            click(&mut handler, toggle_button()).iter().map(hidden_change).collect();
        assert_eq!(first, toggled(Sheet::Editable, Sheet::Immutable));

        let second: Vec<_> =
            click(&mut handler, toggle_button()).iter().map(hidden_change).collect();
        assert_eq!(second, toggled(Sheet::Immutable, Sheet::Editable));
    }

    #[test]
    fn the_initial_draw_labels_the_language_and_reveals_the_body() {
        let mut handler = block_on(Handler::<MemoryStore>::ready(0.0, 0.0, 16.0, 0, 0));
        let (events, commands) = handler.initial_draw();
        assert!(events.is_empty());
        let (texts, reveal) = commands.split_at(commands.len() - 1);
        let expected: Vec<_> =
            labelled(Target::Language, Target::Html, Language::Ja)
                .into_iter()
                .chain(Language::list().iter().flat_map(|&l| {
                    labelled(Target::LanguageOption(l), Target::LanguageOption(l), l)
                }))
                .collect();
        assert_eq!(texts.iter().map(written).collect::<Vec<_>>(), expected);
        assert_eq!(
            reveal.iter().map(hidden_change).collect::<Vec<_>>(),
            [(false, Target::Body.to_dom())]
        );
    }

    #[test]
    fn choosing_a_language_relabels_the_button() {
        let mut handler = block_on(Handler::<MemoryStore>::ready(0.0, 0.0, 16.0, 0, 0));
        let english = Language::En(En::Gb);
        let commands = click(&mut handler, Target::LanguageOption(english).to_dom());
        assert_eq!(
            commands.iter().map(written).collect::<Vec<_>>(),
            labelled(Target::Language, Target::Html, english)
        );
        assert_eq!(handler.language, english);
        let commands = click(&mut handler, Target::LanguageOption(Language::Ja).to_dom());
        assert_eq!(
            commands.iter().map(written).collect::<Vec<_>>(),
            labelled(Target::Language, Target::Html, Language::Ja)
        );
        assert_eq!(handler.language, Language::Ja);
    }

    #[test]
    fn other_targets_and_events_change_nothing() {
        let mut handler = block_on(Handler::<MemoryStore>::ready(0.0, 0.0, 16.0, 0, 0));
        assert!(click(&mut handler, Target::Sheet(Sheet::Immutable, 1).to_dom()).is_empty());
        let first: Vec<_> =
            click(&mut handler, toggle_button()).iter().map(hidden_change).collect();
        assert_eq!(first, toggled(Sheet::Editable, Sheet::Immutable));
    }
    #[test]
    fn closing_the_handler_releases_the_store() {
        let mut handler = block_on(Handler::<MemoryStore>::ready(0.0, 0.0, 16.0, 0, 0));
        assert!(handler.save().1.is_empty());
        assert!(handler.close().is_empty());
        let (events, commands) = handler.save();
        assert!(matches!(events.as_slice(), [Event::StoreLost(_)]));
        assert!(matches!(
            commands.as_slice(),
            [Command::Error { error: Error::FileStore(FileStoreError::InvalidState(_)) }]
        ));
    }
    #[test]
    fn a_lost_store_is_replaced_and_the_pending_change_survives() {
        let mut handler = block_on(Handler::<MemoryStore>::ready(0.0, 0.0, 16.0, 0, 0));
        handler.characters.as_mut().unwrap().set(1, b"kept".to_vec());
        let disk = block_on(MemoryStore::open(CHARACTER_STORE, true)).unwrap();

        handler.close();
        let (events, _) = handler.save();
        assert!(matches!(events.as_slice(), [Event::StoreLost(_)]));
        handler.process_lost();
        handler.process_opened(Ok(disk.clone()));
        assert_eq!(handler.characters.as_ref().unwrap().get(1), Some(&b"kept"[..]));
        assert!(handler.save().1.is_empty());
        assert_eq!(MemoryStore::new(disk).unwrap().get(1), Some(&b"kept"[..]));
    }
}
