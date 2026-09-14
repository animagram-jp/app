use alloc::{vec, vec::Vec};
use core::{
    matches,
    option::Option::{self, None, Some},
    primitive::{f64, u8, u32},
};

use arbitrary_int::u2;

#[cfg(feature = "worker")]
use crate::file_store::FileStore;
#[cfg(feature = "worker")]
use crate::js_client::CommandError;
use crate::{
    Lang,
    arena::Decoder,
    data_struct::DataStruct,
    js_client::{
        Attribute, CanvasEvent, ClassName, Command, EventType, Gesture, KeyName, PointerState, dom,
    },
};

#[cfg(feature = "worker")]
const RETRY_LIMIT: u8 = 3;

/// pointer / key / input / change / focus, etc.
pub const EVENT_CANVAS: u8 = 1;
pub const EVENT_RESIZE: u8 = 2;
pub const EVENT_SCROLL: u8 = 3;
pub const EVENT_SHUTDOWN: u8 = 8;

pub enum Event {
    /// Event originating from the DOM.
    Canvas(CanvasEvent),
    /// A recognized gesture. Pushed by `dispatch` itself.
    Gesture(Gesture),
    Resize { width: f64, height: f64 },
    Scroll { id: dom::Id, x: f64, y: f64 },
    Shutdown,
}

/// ```
/// # use app::event::{decode_event, Event, EVENT_SHUTDOWN};
/// assert!(matches!(decode_event(&[EVENT_SHUTDOWN]), Some(Event::Shutdown)));
/// assert!(decode_event(&[200]).is_none());
/// assert!(decode_event(&[]).is_none());
/// ```
pub fn decode_event(frame: &[u8]) -> Option<Event> {
    let mut decoder = Decoder::new(frame);
    let kind = decoder.u8()?;
    Some(match kind {
        EVENT_CANVAS => Event::Canvas(CanvasEvent {
            event_type: EventType::decode_u8(decoder.u8()?),
            id:         decoder.id()?,
            key:        KeyName::decode_u8(decoder.u8()?),
            value:      decoder.string()?,
            x:          decoder.f32()? as f64,
            y:          decoder.f32()? as f64,
            time:       decoder.f64()?,
            pointer_id: decoder.u32()?,
        }),
        EVENT_RESIZE => {
            Event::Resize { width: decoder.f32()? as f64, height: decoder.f32()? as f64 }
        }
        EVENT_SCROLL => Event::Scroll {
            id: decoder.id()?,
            x:  decoder.f32()? as f64,
            y:  decoder.f32()? as f64,
        },
        EVENT_SHUTDOWN => Event::Shutdown,
        _ => return None,
    })
}

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
    pub async fn ready(_viewport_width: f64, _viewport_height: f64) -> Self {
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
                vec![Command::Error { error: CommandError::FileStore(e) }]
            }
        }
    }

    pub fn process_viewport(&mut self, _width: f64, _height: f64) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    pub fn process_scroll(
        &mut self,
        _id: &dom::Id,
        _x: f64,
        _y: f64,
    ) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    pub fn initial_draw(&self) -> (Vec<Event>, Vec<Command>) {
        let commands = vec![Command::RemoveAttribute {
            id:        dom::Id::new(&[(dom::Tag::Body, None)]),
            attribute: Attribute::Hidden,
        }];
        (vec![], commands)
    }

    pub fn process(
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
                (1, 2)
            }
            CharacterSheet::Editable => {
                self.character_sheet = CharacterSheet::Immutable;
                (2, 1)
            }
        };
        let commands = vec![
            Command::RemoveClass { id: section(shown), value: ClassName::Hidden },
            Command::AddClass { id: section(hidden), value: ClassName::Hidden },
        ];
        (vec![], commands)
    }

    pub fn process_gesture(
        &mut self,
        _gesture: &Gesture,
        _state: &PointerState,
    ) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }
}
