use alloc::{string::String, vec::Vec};
use core::{
    fmt::{self, Debug, Display, Formatter},
    option::Option::{self, None, Some},
    primitive::{f64, u8},
};

use crate::js_client::{
    CanvasEvent, EventType, Gesture, Input, KeyName, VisibilityState, WireError,
};

#[derive(Debug)]
pub enum EventError {
    Decode,
}

impl Display for EventError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl WireError for EventError {
    fn identifiers(&self, path: &mut Vec<u16>) {
        match self {
            EventError::Decode => path.push(1),
        }
    }

    fn detail(&self) -> String {
        String::new()
    }

    fn is_serious(&self) -> bool {
        false
    }
}

/// pointer / key / input / change / focus, etc.
pub const EVENT_CANVAS: u8 = 1;
pub const EVENT_RESIZE: u8 = 2;
pub const EVENT_SCROLL: u8 = 3;
pub const EVENT_VISIBILITY: u8 = 4;
pub const EVENT_FETCH: u8 = 5;
pub const EVENT_SHUTDOWN: u8 = 8;

pub enum Event {
    /// Event originating from the DOM.
    Canvas(CanvasEvent),
    /// A recognized gesture. Pushed by `dispatch` itself.
    Gesture(Gesture),
    Window(WindowEvent),
    FetchChunk(FetchChunk),
    Fetched(Response),
}

pub struct FetchChunk {
    pub request: u32,
    pub status:  u16,
    pub last:    bool,
    pub bytes:   Vec<u8>,
}

pub struct Response {
    pub request: u32,
    pub status:  u16,
    pub body:    Vec<u8>,
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
    let kind = input.decode::<u8>()?;
    Some(match kind {
        EVENT_CANVAS => Event::Canvas(CanvasEvent {
            event_type: EventType::decode_u8(input.decode::<u8>()?),
            id:         input.decode()?,
            key:        KeyName::decode_u8(input.decode::<u8>()?),
            flags:      input.decode()?,
            value:      input.decode()?,
            x:          input.decode::<f32>()? as f64,
            y:          input.decode::<f32>()? as f64,
            local_x:    input.decode::<f32>()? as f64,
            local_y:    input.decode::<f32>()? as f64,
            time:       input.decode()?,
            pointer_id: input.decode()?,
        }),
        EVENT_RESIZE => Event::Window(WindowEvent::Resize {
            width:  input.decode::<f32>()? as f64,
            height: input.decode::<f32>()? as f64,
        }),
        EVENT_SCROLL => Event::Window(WindowEvent::Scroll {
            x: input.decode::<f32>()? as f64,
            y: input.decode::<f32>()? as f64,
        }),
        EVENT_VISIBILITY => Event::Window(WindowEvent::Visibility {
            state: VisibilityState::decode_u8(input.decode::<u8>()?),
        }),
        EVENT_FETCH => Event::FetchChunk(FetchChunk {
            request: input.decode()?,
            status:  input.decode()?,
            last:    input.decode::<u8>()? != 0,
            bytes:   input.decode()?,
        }),
        EVENT_SHUTDOWN => Event::Window(WindowEvent::Shutdown),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use alloc::{format, vec::Vec};
    use core::matches;

    use super::*;
    use crate::js_client::{Encode, Output, dom};

    const INIT_JS: &str = include_str!("../distribution/init.js");

    const KEY_DOWN: u8 = 8;
    const ENTER: u8 = 37;
    const CTRL_REPEAT_SHIFT: u8 = (1 << 2) | (1 << 4) | (1 << 5);

    fn js_constant(name: &str) -> u8 {
        let head = format!("const {name} = ");
        let start = INIT_JS.find(&head).unwrap_or_else(|| panic!("{name} not found")) + head.len();
        INIT_JS[start..].split(';').next().unwrap().parse().unwrap()
    }

    fn section(n: u32) -> dom::Id {
        dom::Id::new(&[(dom::Tag::Main, None), (dom::Tag::Section, Some(n))])
    }

    fn canvas_frame() -> Vec<u8> {
        let mut frame = Vec::new();
        frame.push(EVENT_CANVAS);
        frame.push(KEY_DOWN);
        section(2).encode(&mut frame);
        frame.push(ENTER);
        frame.push(CTRL_REPEAT_SHIFT);
        frame.encode::<str>("a");
        frame.encode::<f32>(&1.5);
        frame.encode::<f32>(&2.5);
        frame.encode::<f32>(&0.5);
        frame.encode::<f32>(&1.0);
        frame.extend_from_slice(&3.0f64.to_le_bytes());
        frame.encode::<u32>(&9);
        frame
    }

    #[test]
    fn frame_kinds_match_init_js() {
        assert_eq!(js_constant("EVENT_CANVAS"), EVENT_CANVAS);
        assert_eq!(js_constant("EVENT_RESIZE"), EVENT_RESIZE);
        assert_eq!(js_constant("EVENT_SCROLL"), EVENT_SCROLL);
        assert_eq!(js_constant("EVENT_VISIBILITY"), EVENT_VISIBILITY);
        assert_eq!(js_constant("EVENT_FETCH"), EVENT_FETCH);
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
        assert_eq!(event.id, section(2));
        assert_eq!(event.key, KeyName::Enter);
        assert_eq!(
            [event.alt(), event.ctrl(), event.meta(), event.repeat(), event.shift()],
            [false, true, false, true, true]
        );
        assert_eq!(event.value, "a");
        assert_eq!((event.x, event.y, event.time, event.pointer_id), (1.5, 2.5, 3.0, 9));
        assert_eq!((event.local_x, event.local_y), (0.5, 1.0));
        assert_eq!(event.root_origin(), (1.0, 1.5));
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
        resize.encode::<f32>(&640.0);
        resize.encode::<f32>(&480.0);
        assert!(matches!(
            decode_event(&resize),
            Some(Event::Window(WindowEvent::Resize { width: 640.0, height: 480.0 }))
        ));

        let mut scroll = Vec::new();
        scroll.push(EVENT_SCROLL);
        scroll.encode::<f32>(&12.5);
        scroll.encode::<f32>(&300.0);
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
    fn event_error_decode_is_recoverable_and_has_no_detail() {
        let mut path = Vec::new();
        EventError::Decode.identifiers(&mut path);
        assert_eq!(path, [1]);
        assert_eq!(EventError::Decode.detail(), "");
        assert!(!EventError::Decode.is_serious());
    }

    #[test]
    fn decodes_fetch_chunks() {
        let mut frame = Vec::new();
        frame.push(EVENT_FETCH);
        frame.encode::<u32>(&7);
        frame.extend_from_slice(&404u16.to_le_bytes());
        frame.push(1);
        frame.encode::<u32>(&2);
        frame.extend_from_slice(&[5, 6]);
        let Some(Event::FetchChunk(chunk)) = decode_event(&frame) else {
            panic!("not a fetch chunk");
        };
        assert_eq!((chunk.request, chunk.status, chunk.last), (7, 404, true));
        assert_eq!(chunk.bytes, [5, 6]);
        for cut in 0..frame.len() {
            assert!(decode_event(&frame[..cut]).is_none(), "cut {cut}");
        }
    }

    #[test]
    fn unknown_or_empty_frames_are_rejected() {
        assert!(decode_event(&[]).is_none());
        for kind in [0, 6, 7, 9, 200] {
            assert!(decode_event(&[kind, 0, 0, 0, 0, 0, 0, 0, 0]).is_none(), "kind {kind}");
        }
    }
}
