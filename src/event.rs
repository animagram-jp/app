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
    data_struct::DataStruct,
    js_client::{
        CanvasEvent, ClassName, Command, EventType, Gesture, KeyName, PointerState,
        VisibilityState, dom, get_f32, get_f64, get_string, get_u8, get_u32,
    },
};

#[cfg(feature = "worker")]
const RETRY_LIMIT: u8 = 3;

/// pointer / key / input / change / focus, etc.
pub const EVENT_CANVAS: u8 = 1;
pub const EVENT_RESIZE: u8 = 2;
pub const EVENT_SCROLL: u8 = 3;
pub const EVENT_VISIBILITY: u8 = 4;
pub const EVENT_SHUTDOWN: u8 = 8;

pub enum Event {
    /// Event originating from the DOM.
    Canvas(CanvasEvent),
    /// A recognized gesture. Pushed by `dispatch` itself.
    Gesture(Gesture),
    Window(WindowEvent),
}

pub enum WindowEvent {
    Resize { width: f64, height: f64 },
    Scroll { x: f64, y: f64 },
    Shutdown,
    Visibility { state: VisibilityState },
}

/// ```
/// # use app::event::{decode_event, Event, WindowEvent, EVENT_SHUTDOWN};
/// assert!(matches!(decode_event(&[EVENT_SHUTDOWN]), Some(Event::Window(WindowEvent::Shutdown))));
/// assert!(decode_event(&[200]).is_none());
/// assert!(decode_event(&[]).is_none());
/// ```
pub fn decode_event(frame: &[u8]) -> Option<Event> {
    let mut input = frame;
    let kind = get_u8(&mut input)?;
    Some(match kind {
        EVENT_CANVAS => Event::Canvas(CanvasEvent {
            event_type: EventType::decode_u8(get_u8(&mut input)?),
            id:         dom::Id::decode(&mut input)?,
            key:        KeyName::decode_u8(get_u8(&mut input)?),
            flags:      get_u8(&mut input)?,
            value:      get_string(&mut input)?,
            x:          get_f32(&mut input)? as f64,
            y:          get_f32(&mut input)? as f64,
            time:       get_f64(&mut input)?,
            pointer_id: get_u32(&mut input)?,
        }),
        EVENT_RESIZE => Event::Window(WindowEvent::Resize {
            width:  get_f32(&mut input)? as f64,
            height: get_f32(&mut input)? as f64,
        }),
        EVENT_SCROLL => Event::Window(WindowEvent::Scroll {
            x: get_f32(&mut input)? as f64,
            y: get_f32(&mut input)? as f64,
        }),
        EVENT_VISIBILITY => Event::Window(WindowEvent::Visibility {
            state: VisibilityState::decode_u8(get_u8(&mut input)?),
        }),
        EVENT_SHUTDOWN => Event::Window(WindowEvent::Shutdown),
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

    pub fn process_resize(&mut self, _width: f64, _height: f64) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    pub fn process_scroll(&mut self, _x: f64, _y: f64) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    pub fn process_visibility(&mut self, _state: VisibilityState) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    pub fn initial_draw(&self) -> (Vec<Event>, Vec<Command>) {
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
        _origin: Option<&CanvasEvent>,
    ) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }
}

#[cfg(test)]
mod tests {
    use alloc::{format, vec::Vec};

    use super::*;
    use crate::js_client::{put_f32, put_str, put_u32};

    const INIT_JS: &str = include_str!("../distribution/init.js");

    const KEY_DOWN: u8 = 8;
    const ENTER: u8 = 37;
    const CTRL_REPEAT_SHIFT: u8 = (1 << 2) | (1 << 4) | (1 << 5);

    fn js_constant(name: &str) -> u8 {
        let head = format!("const {name} = ");
        let start = INIT_JS.find(&head).unwrap_or_else(|| panic!("{name} not found")) + head.len();
        INIT_JS[start..].split(';').next().unwrap().parse().unwrap()
    }

    fn section_2() -> dom::Id {
        dom::Id::new(&[(dom::Tag::Main, None), (dom::Tag::Section, Some(2))])
    }

    fn canvas_frame() -> Vec<u8> {
        let mut frame = Vec::new();
        frame.push(EVENT_CANVAS);
        frame.push(KEY_DOWN);
        section_2().encode(&mut frame);
        frame.push(ENTER);
        frame.push(CTRL_REPEAT_SHIFT);
        put_str(&mut frame, "a");
        put_f32(&mut frame, 1.5);
        put_f32(&mut frame, 2.5);
        frame.extend_from_slice(&3.0f64.to_le_bytes());
        put_u32(&mut frame, 9);
        frame
    }

    #[test]
    fn frame_kinds_match_init_js() {
        assert_eq!(js_constant("EVENT_CANVAS"), EVENT_CANVAS);
        assert_eq!(js_constant("EVENT_RESIZE"), EVENT_RESIZE);
        assert_eq!(js_constant("EVENT_SCROLL"), EVENT_SCROLL);
        assert_eq!(js_constant("EVENT_VISIBILITY"), EVENT_VISIBILITY);
        assert_eq!(js_constant("EVENT_SHUTDOWN"), EVENT_SHUTDOWN);
    }

    #[test]
    fn decodes_canvas_event() {
        assert_eq!(EventType::decode_u8(KEY_DOWN), EventType::KeyDown);
        assert_eq!(KeyName::decode_u8(ENTER), KeyName::Enter);

        let Some(Event::Canvas(event)) = decode_event(&canvas_frame()) else {
            panic!("not a canvas event");
        };
        assert_eq!(event.event_type, EventType::KeyDown);
        assert_eq!(event.id, section_2());
        assert_eq!(event.key, KeyName::Enter);
        assert_eq!(
            [event.alt(), event.ctrl(), event.meta(), event.repeat(), event.shift()],
            [false, true, false, true, true]
        );
        assert_eq!(event.value, "a");
        assert_eq!((event.x, event.y, event.time, event.pointer_id), (1.5, 2.5, 3.0, 9));
    }

    #[test]
    fn every_truncated_canvas_frame_is_rejected() {
        let frame = canvas_frame();
        for cut in 0..frame.len() {
            assert!(decode_event(&frame[..cut]).is_none(), "cut {cut}");
        }
    }

    #[test]
    fn decodes_window_events() {
        let mut resize = Vec::new();
        resize.push(EVENT_RESIZE);
        put_f32(&mut resize, 640.0);
        put_f32(&mut resize, 480.0);
        assert!(matches!(
            decode_event(&resize),
            Some(Event::Window(WindowEvent::Resize { width: 640.0, height: 480.0 }))
        ));

        let mut scroll = Vec::new();
        scroll.push(EVENT_SCROLL);
        put_f32(&mut scroll, 12.5);
        put_f32(&mut scroll, 300.0);
        assert!(matches!(
            decode_event(&scroll),
            Some(Event::Window(WindowEvent::Scroll { x: 12.5, y: 300.0 }))
        ));

        assert!(matches!(
            decode_event(&[EVENT_VISIBILITY, 1]),
            Some(Event::Window(WindowEvent::Visibility { state: VisibilityState::Hidden }))
        ));
        assert!(matches!(
            decode_event(&[EVENT_VISIBILITY, 2]),
            Some(Event::Window(WindowEvent::Visibility { state: VisibilityState::Visible }))
        ));
        assert!(matches!(
            decode_event(&[EVENT_VISIBILITY, 0]),
            Some(Event::Window(WindowEvent::Visibility { state: VisibilityState::Other }))
        ));
        assert!(matches!(
            decode_event(&[EVENT_SHUTDOWN]),
            Some(Event::Window(WindowEvent::Shutdown))
        ));

        for frame in [&resize[..resize.len() - 1], &scroll[..scroll.len() - 1], &[EVENT_VISIBILITY]]
        {
            assert!(decode_event(frame).is_none());
        }
    }

    #[test]
    fn unknown_or_empty_frames_are_rejected() {
        assert!(decode_event(&[]).is_none());
        for kind in [0, 5, 6, 7, 9, 200] {
            assert!(decode_event(&[kind, 0, 0, 0, 0, 0, 0, 0, 0]).is_none(), "kind {kind}");
        }
    }
}
