use alloc::{string::String, vec::Vec};
use core::{
    fmt::{self, Debug, Display, Formatter},
    option::Option::{self, None, Some},
    primitive::{f64, u8},
};

use crate::{
    file_store::{Backend, FileStore, FileStoreError, StoreId},
    js_client::{
        CanvasEvent, EventType, FullscreenEvent, Gesture, Input, KeyName, VisibilityState,
        WireError,
    },
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
}

/// pointer / key / input / change / focus, etc.
pub const EVENT_CANVAS: u8 = 1;
pub const EVENT_RESIZE: u8 = 2;
pub const EVENT_SCROLL: u8 = 3;
pub const EVENT_VISIBILITY: u8 = 4;
pub const EVENT_FETCH: u8 = 5;
pub const EVENT_FULLSCREEN: u8 = 6;
pub const EVENT_SHUTDOWN: u8 = 8;

pub enum Event {
    /// Event originating from the DOM.
    Canvas(CanvasEvent),
    /// A recognized gesture. Pushed by `dispatch` itself.
    Gesture(Gesture),
    Window(WindowEvent),
    FetchChunk(FetchChunk),
    Fetched(Response),
    StoreLost(StoreId),
    StoreOpened(Opened),
}

pub type Opened = Result<<Backend as FileStore>::Handle, FileStoreError>;

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
    Fullscreen(FullscreenEvent),
}

/// ```
/// # use app::{event::{Event, WindowEvent, EVENT_SHUTDOWN}, js_client::Input};
/// assert!(matches!(Event::decode(&mut &[EVENT_SHUTDOWN][..]), Some(Event::Window(WindowEvent::Shutdown))));
/// assert!(Event::decode(&mut &[200][..]).is_none());
/// assert!(Event::decode(&mut &[][..]).is_none());
/// ```
impl Input for Event {
    fn decode(source: &mut &[u8]) -> Option<Self> {
        let mut input = *source;
        let kind = u8::decode(&mut input)?;
        let event = match kind {
            EVENT_CANVAS => Event::Canvas(CanvasEvent {
                event_type: EventType::from_u8(u8::decode(&mut input)?),
                id:         Input::decode(&mut input)?,
                key:        KeyName::from_u8(u8::decode(&mut input)?),
                flags:      Input::decode(&mut input)?,
                value:      Input::decode(&mut input)?,
                x:          f32::decode(&mut input)? as f64,
                y:          f32::decode(&mut input)? as f64,
                local_x:    f32::decode(&mut input)? as f64,
                local_y:    f32::decode(&mut input)? as f64,
                time:       Input::decode(&mut input)?,
                pointer_id: Input::decode(&mut input)?,
            }),
            EVENT_RESIZE => Event::Window(WindowEvent::Resize {
                width:  f32::decode(&mut input)? as f64,
                height: f32::decode(&mut input)? as f64,
            }),
            EVENT_SCROLL => Event::Window(WindowEvent::Scroll {
                x: f32::decode(&mut input)? as f64,
                y: f32::decode(&mut input)? as f64,
            }),
            EVENT_VISIBILITY => Event::Window(WindowEvent::Visibility {
                state: VisibilityState::from_u8(u8::decode(&mut input)?),
            }),
            EVENT_FETCH => Event::FetchChunk(FetchChunk {
                request: Input::decode(&mut input)?,
                status:  Input::decode(&mut input)?,
                last:    u8::decode(&mut input)? != 0,
                bytes:   Input::decode(&mut input)?,
            }),
            EVENT_FULLSCREEN => Event::Window(WindowEvent::Fullscreen(FullscreenEvent::from_u8(
                u8::decode(&mut input)?,
            ))),
            EVENT_SHUTDOWN => Event::Window(WindowEvent::Shutdown),
            _ => return None,
        };
        *source = input;
        Some(event)
    }
}

#[cfg(test)]
mod tests {
    use alloc::{format, vec::Vec};
    use core::matches;

    use super::*;
    use crate::{js_client::Output, testing::Rng};

    fn decode_event(frame: &[u8]) -> Option<Event> {
        Event::decode(&mut { frame })
    }

    const INIT_JS: &str = include_str!("../distribution/init.js");

    fn js_constant(name: &str) -> u8 {
        let head = format!("const {name} = ");
        let start = INIT_JS.find(&head).unwrap_or_else(|| panic!("{name} not found")) + head.len();
        INIT_JS[start..].split(';').next().unwrap().parse().unwrap()
    }

    #[test]
    fn frame_kinds_match_init_js() {
        assert_eq!(js_constant("EVENT_CANVAS"), EVENT_CANVAS);
        assert_eq!(js_constant("EVENT_RESIZE"), EVENT_RESIZE);
        assert_eq!(js_constant("EVENT_SCROLL"), EVENT_SCROLL);
        assert_eq!(js_constant("EVENT_VISIBILITY"), EVENT_VISIBILITY);
        assert_eq!(js_constant("EVENT_FETCH"), EVENT_FETCH);
        assert_eq!(js_constant("EVENT_FULLSCREEN"), EVENT_FULLSCREEN);
        assert_eq!(js_constant("EVENT_SHUTDOWN"), EVENT_SHUTDOWN);
    }

    fn pack(kind: u8, parts: &[&dyn Fn(&mut Vec<u8>)]) -> Vec<u8> {
        let mut frame = Vec::new();
        frame.push(kind);
        for part in parts {
            part(&mut frame);
        }
        frame
    }

    fn float(rng: &mut Rng) -> f32 {
        (rng.next_u64() as i32) as f32 / 8.0
    }

    fn assert_decodes_only_whole(frame: &[u8], seed: u64) {
        assert!(decode_event(frame).is_some(), "seed {seed}");
        for cut in 0..frame.len() {
            assert!(decode_event(&frame[..cut]).is_none(), "seed {seed} cut {cut}");
        }
    }

    #[test]
    fn random_frames_decode_to_their_fields_and_every_proper_prefix_is_rejected() {
        let kinds = [
            EVENT_CANVAS,
            EVENT_RESIZE,
            EVENT_SCROLL,
            EVENT_VISIBILITY,
            EVENT_FETCH,
            EVENT_FULLSCREEN,
            EVENT_SHUTDOWN,
        ];
        for seed in 0..500 {
            let mut rng = Rng::new(seed);
            match kinds[rng.below(kinds.len())] {
                EVENT_CANVAS => {
                    let (kind, key, flags) =
                        (rng.next_u64() as u8, rng.next_u64() as u8, rng.next_u64() as u8);
                    let (id, value) = (rng.id(), rng.string());
                    let coordinates =
                        [float(&mut rng), float(&mut rng), float(&mut rng), float(&mut rng)];
                    let (time, pointer) =
                        (rng.next_u64() as i32 as f64 / 4.0, rng.next_u64() as u32);
                    let frame = pack(
                        EVENT_CANVAS,
                        &[
                            &|f| f.push(kind),
                            &|f| id.encode(f),
                            &|f| f.push(key),
                            &|f| f.push(flags),
                            &|f| str::encode(&value, f),
                            &|f| coordinates.iter().for_each(|c| c.encode(f)),
                            &|f| time.encode(f),
                            &|f| pointer.encode(f),
                        ],
                    );
                    assert_decodes_only_whole(&frame, seed);
                    let Some(Event::Canvas(event)) = decode_event(&frame) else {
                        panic!("seed {seed}: not a canvas event");
                    };
                    assert_eq!(event.event_type, EventType::from_u8(kind), "seed {seed}");
                    assert_eq!(event.key, KeyName::from_u8(key), "seed {seed}");
                    assert_eq!(
                        (&event.id, &event.value, event.flags),
                        (&id, &value, flags),
                        "seed {seed}"
                    );
                    let [x, y, local_x, local_y] = coordinates.map(f64::from);
                    assert_eq!(
                        (event.x, event.y, event.local_x, event.local_y),
                        (x, y, local_x, local_y)
                    );
                    assert_eq!((event.time, event.pointer_id), (time, pointer), "seed {seed}");
                    assert_eq!(event.root_origin(), (x - local_x, y - local_y), "seed {seed}");
                }
                kind @ (EVENT_RESIZE | EVENT_SCROLL) => {
                    let (a, b) = (float(&mut rng), float(&mut rng));
                    let frame = pack(kind, &[&|f| a.encode(f), &|f| b.encode(f)]);
                    assert_decodes_only_whole(&frame, seed);
                    let (a, b) = (f64::from(a), f64::from(b));
                    match decode_event(&frame) {
                        Some(Event::Window(WindowEvent::Resize { width, height }))
                            if kind == EVENT_RESIZE =>
                        {
                            assert_eq!((width, height), (a, b), "seed {seed}")
                        }
                        Some(Event::Window(WindowEvent::Scroll { x, y }))
                            if kind == EVENT_SCROLL =>
                        {
                            assert_eq!((x, y), (a, b), "seed {seed}")
                        }
                        _ => panic!("seed {seed}: wrong window event"),
                    }
                }
                EVENT_VISIBILITY => {
                    let state = rng.next_u64() as u8;
                    let frame = pack(EVENT_VISIBILITY, &[&|f| f.push(state)]);
                    assert_decodes_only_whole(&frame, seed);
                    let Some(Event::Window(WindowEvent::Visibility { state: decoded })) =
                        decode_event(&frame)
                    else {
                        panic!("seed {seed}: not a visibility event");
                    };
                    assert_eq!(decoded, VisibilityState::from_u8(state), "seed {seed}");
                }
                EVENT_FULLSCREEN => {
                    let event = rng.next_u64() as u8;
                    let frame = pack(EVENT_FULLSCREEN, &[&|f| f.push(event)]);
                    assert_decodes_only_whole(&frame, seed);
                    let Some(Event::Window(WindowEvent::Fullscreen(decoded))) =
                        decode_event(&frame)
                    else {
                        panic!("seed {seed}: not a fullscreen event");
                    };
                    assert_eq!(decoded, FullscreenEvent::from_u8(event), "seed {seed}");
                }
                EVENT_FETCH => {
                    let (request, status, last) =
                        (rng.next_u64() as u32, rng.next_u64() as u16, rng.next_u64() as u8);
                    let length = rng.below(40);
                    let bytes = rng.bytes(length);
                    let frame = pack(
                        EVENT_FETCH,
                        &[&|f| request.encode(f), &|f| status.encode(f), &|f| f.push(last), &|f| {
                            <[u8]>::encode(&bytes, f)
                        }],
                    );
                    assert_decodes_only_whole(&frame, seed);
                    let Some(Event::FetchChunk(chunk)) = decode_event(&frame) else {
                        panic!("seed {seed}: not a fetch chunk");
                    };
                    assert_eq!(
                        (chunk.request, chunk.status, chunk.last, chunk.bytes),
                        (request, status, last != 0, bytes)
                    );
                }
                _ => assert!(matches!(
                    decode_event(&[EVENT_SHUTDOWN]),
                    Some(Event::Window(WindowEvent::Shutdown))
                )),
            }
        }
    }

    #[test]
    fn arbitrary_bytes_never_panic_and_unknown_kinds_are_rejected() {
        let known = [
            EVENT_CANVAS,
            EVENT_RESIZE,
            EVENT_SCROLL,
            EVENT_VISIBILITY,
            EVENT_FETCH,
            EVENT_FULLSCREEN,
            EVENT_SHUTDOWN,
        ];
        for seed in 0..2000 {
            let mut rng = Rng::new(seed);
            let length = rng.below(80);
            let mut frame = rng.bytes(length);
            if rng.chance(30) && !frame.is_empty() {
                frame[0] = known[rng.below(known.len())];
            }
            let decoded = decode_event(&frame);
            if frame.first().is_none_or(|kind| !known.contains(kind)) {
                assert!(decoded.is_none(), "seed {seed}");
            }
        }
    }
}
