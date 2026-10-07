use alloc::{
    collections::{BTreeMap, VecDeque},
    vec,
    vec::Vec,
};
use core::{
    default::Default,
    iter::Extend,
    option::Option::{None, Some},
    primitive::{bool, f64, i32, u8},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(feature = "calendar")]
use crate::calendar::handler::Handler;
#[cfg(not(feature = "calendar"))]
use crate::handler::Handler;
use crate::{
    Error,
    arena::{APP, RUNNING, emit},
    event::{Event, EventError, Response, WindowEvent, decode_event},
    file_store::StoreId,
    js_client::{
        CanvasEvent, Command, EventType, Thresholds, TouchTracker, detect_device, encode_command,
    },
};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct App {
    touch:      TouchTracker,
    thresholds: Thresholds,
    events:     VecDeque<Event>,
    handler:    Handler,
    commands:   Vec<Command>,
    origin:     Option<CanvasEvent>,
    responses:  BTreeMap<u32, Vec<u8>>,
    reopen:     Option<StoreId>,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl App {
    pub async fn init(
        pointer_coarse: bool,
        viewport_width: f64,
        viewport_height: f64,
        rem_in_px: f64,
        now: f64,
        timezone_offset_minutes: i32,
    ) {
        let mut app = App::new(
            pointer_coarse,
            Handler::ready(
                viewport_width,
                viewport_height,
                rem_in_px,
                now,
                timezone_offset_minutes,
            )
            .await,
        );

        let (_events, commands) = app.handler.initial_draw();
        for command in &commands {
            let mut frame = Vec::new();
            encode_command(&mut frame, command);
            emit(&frame);
        }

        #[allow(clippy::deref_addrof)]
        unsafe {
            *(&raw mut APP) = Some(app)
        };
    }

    /// Decode a event frame and dispatch it together with every event
    /// it derives, in FIFO order, appending the resulting commands.
    pub fn process(&mut self, frame: &[u8]) {
        let Some(event) = decode_event(frame) else {
            self.commands.push(Command::Error { error: Error::Event(EventError::Decode) });
            return;
        };

        self.run(event);
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }

    fn dispatch(&mut self, event: Event) -> (Vec<Event>, Vec<Command>) {
        let Self { handler, touch, thresholds, origin, responses, reopen, .. } = self;

        match event {
            Event::Canvas(canvas_event) => {
                if canvas_event.event_type == EventType::PointerDown
                    && !touch.active_state().is_down()
                {
                    *origin = Some(canvas_event.clone());
                }
                match touch.handle(
                    &canvas_event.event_type,
                    canvas_event.pointer_id,
                    canvas_event.x,
                    canvas_event.y,
                    canvas_event.time,
                    thresholds,
                ) {
                    Some(gesture) => (vec![Event::Gesture(gesture)], vec![]),
                    None => match canvas_event.event_type {
                        EventType::PointerMove
                        | EventType::PointerUp
                        | EventType::PointerCancel => (vec![], vec![]),
                        _ => handler.process_canvas(&canvas_event, touch.active_state()),
                    },
                }
            }
            Event::Gesture(gesture) => {
                handler.process_gesture(&gesture, touch.active_state(), origin.as_ref())
            }
            Event::Window(WindowEvent::Resize { width, height }) => {
                handler.process_resize(width, height)
            }
            Event::Window(WindowEvent::Scroll { x, y }) => handler.process_scroll(x, y),
            Event::Window(WindowEvent::Visibility { state }) => handler.process_visibility(state),
            Event::FetchChunk(chunk) => {
                let body = responses.entry(chunk.request).or_default();
                body.extend_from_slice(&chunk.bytes);
                if !chunk.last {
                    return (vec![], vec![]);
                }
                let body = responses.remove(&chunk.request).unwrap_or_default();
                let response = Response { request: chunk.request, status: chunk.status, body };
                (vec![Event::Fetched(response)], vec![])
            }
            Event::Fetched(response) => handler.process_fetched(&response),
            Event::StoreLost(id) => {
                *reopen = Some(id);
                handler.process_lost()
            }
            Event::StoreOpened(opened) => handler.process_opened(opened),
            Event::Window(WindowEvent::Shutdown) => {
                unsafe { RUNNING = false };
                (vec![], self.handler.close())
            }
        }
    }
}

impl App {
    pub(crate) fn new(pointer_coarse: bool, handler: Handler) -> Self {
        Self {
            touch: TouchTracker::default(),
            thresholds: Thresholds::for_device(detect_device(pointer_coarse)),
            events: VecDeque::new(),
            handler,
            commands: Vec::new(),
            origin: None,
            responses: BTreeMap::new(),
            reopen: None,
        }
    }

    pub(crate) fn run(&mut self, event: Event) {
        self.events.push_back(event);

        while let Some(event) = self.events.pop_front() {
            let (new_events, new_commands) = self.dispatch(event);
            self.events.extend(new_events);
            self.commands.extend(new_commands);
        }
    }

    pub(crate) fn reopen_wanted(&mut self) -> Option<StoreId> {
        self.reopen.take()
    }

    pub fn commands(&self) -> &[Command] {
        &self.commands
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;
    #[cfg(feature = "calendar")]
    use std::fs;

    use super::*;
    use crate::{
        event::EVENT_CANVAS,
        file_store::{Backend, FileStore, StoreId},
        js_client::{Gesture, Output, dom},
        testing::{Rng, block_on},
    };

    const POINTER_DOWN: u8 = 10;
    const POINTER_UP: u8 = 12;

    fn new_app() -> App {
        App::new(false, block_on(Handler::ready(0.0, 0.0, 16.0, 0.0, 0)))
    }

    fn section(n: u32) -> dom::Id {
        dom::Id::new(&[(dom::Tag::Main, None), (dom::Tag::Section, Some(n))])
    }

    fn pointer_frame(event_type: u8, id: &dom::Id, x: f32, pointer_id: u32, time: f64) -> Vec<u8> {
        let mut frame = Vec::new();
        frame.push(EVENT_CANVAS);
        frame.push(event_type);
        id.encode(&mut frame);
        frame.push(0);
        frame.push(0);
        str::encode("", &mut frame);
        x.encode(&mut frame);
        0.0f32.encode(&mut frame);
        x.encode(&mut frame);
        0.0f32.encode(&mut frame);
        frame.extend_from_slice(&time.to_le_bytes());
        pointer_id.encode(&mut frame);
        frame
    }

    fn origin_id(app: &App) -> Option<dom::Id> {
        app.origin.as_ref().map(|origin| origin.id.clone())
    }

    fn chunk(request: u32, status: u16, last: bool, bytes: &[u8]) -> Event {
        Event::FetchChunk(crate::event::FetchChunk { request, status, last, bytes: bytes.to_vec() })
    }

    #[test]
    fn origin_follows_the_pointerdown_that_starts_a_sequence() {
        assert_eq!(EventType::decode_u8(POINTER_DOWN), EventType::PointerDown);
        assert_eq!(EventType::decode_u8(POINTER_UP), EventType::PointerUp);

        let mut app = new_app();
        assert_eq!(origin_id(&app), None);

        app.process(&pointer_frame(POINTER_DOWN, &section(1), 10.0, 1, 0.0));
        assert_eq!(origin_id(&app), Some(section(1)));

        app.process(&pointer_frame(POINTER_UP, &section(1), 10.0, 1, 50.0));
        assert_eq!(origin_id(&app), Some(section(1)));

        app.process(&pointer_frame(POINTER_DOWN, &section(2), 30.0, 2, 1000.0));
        assert_eq!(origin_id(&app), Some(section(2)));
    }

    #[test]
    fn a_second_pointer_does_not_replace_the_origin() {
        let mut app = new_app();

        app.process(&pointer_frame(POINTER_DOWN, &section(1), 10.0, 1, 0.0));
        app.process(&pointer_frame(POINTER_DOWN, &section(2), 200.0, 2, 5.0));
        assert_eq!(origin_id(&app), Some(section(1)));
    }

    #[test]
    fn a_recognized_gesture_is_returned_as_an_event() {
        let mut app = new_app();
        app.process(&pointer_frame(POINTER_DOWN, &section(1), 10.0, 1, 0.0));

        let Some(Event::Canvas(release)) =
            decode_event(&pointer_frame(POINTER_UP, &section(1), 10.0, 1, 50.0))
        else {
            panic!("not a canvas event");
        };
        let (events, commands) = app.dispatch(Event::Canvas(release));
        assert!(matches!(events.as_slice(), [Event::Gesture(Gesture::Tap)]));
        assert!(commands.is_empty());
    }

    #[cfg(not(feature = "calendar"))]
    #[test]
    fn commands_accumulate_across_frames_and_a_bad_frame_does_not_break_the_queue() {
        const CLICK: u8 = 2;
        let toggle = dom::Id::new(&[(dom::Tag::Header, None), (dom::Tag::Button, Some(3))]);
        let mut app = new_app();

        app.process(&pointer_frame(CLICK, &toggle, 0.0, 0, 0.0));
        assert!(matches!(
            app.commands(),
            [Command::RemoveAttribute { .. }, Command::SetAttribute { .. }]
        ));

        app.process(&[]);
        assert!(matches!(
            app.commands(),
            [.., Command::Error { error: Error::Event(EventError::Decode) }]
        ));
        assert_eq!(app.commands().len(), 3);

        app.process(&pointer_frame(CLICK, &toggle, 0.0, 0, 0.0));
        let [.., Command::SetAttribute { id: hidden, .. }] = app.commands() else {
            panic!("not toggled back");
        };
        assert_eq!(*hidden, section(2));
        assert_eq!(app.commands().len(), 5);
        assert!(app.events.is_empty());

        app.clear();
        assert!(app.commands().is_empty());
    }

    #[cfg(feature = "calendar")]
    fn fetch_frame(request: u32, status: u16, last: bool, bytes: &[u8]) -> Vec<u8> {
        let mut frame = Vec::new();
        frame.push(crate::event::EVENT_FETCH);
        request.encode(&mut frame);
        frame.extend_from_slice(&status.to_le_bytes());
        frame.push(last as u8);
        (bytes.len() as u32).encode(&mut frame);
        frame.extend_from_slice(bytes);
        frame
    }

    #[cfg(feature = "calendar")]
    #[test]
    fn fetch_chunks_are_joined_until_the_last_one() {
        let body = fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/distribution/calendar/data/calendar.json"
        ))
        .unwrap();
        let mut app = new_app();

        let chunks: Vec<&[u8]> = body.chunks(4000).collect();
        let last = chunks.len() - 1;
        for (index, chunk) in chunks.iter().enumerate() {
            assert!(app.handler.calendar().is_none());
            app.process(&fetch_frame(1, 200, index == last, chunk));
        }
        assert_eq!(app.handler.calendar().unwrap().appointments.len(), 380);
        assert!(app.responses.is_empty());
    }

    #[test]
    fn random_chunked_responses_arrive_whole_and_exactly_once() {
        for seed in 0..1000 {
            let mut rng = Rng::new(seed);
            let mut app = new_app();
            let mut queues: Vec<(u32, u16, Vec<Vec<u8>>, Vec<u8>)> = Vec::new();
            for request in 1..=1 + rng.below(3) as u32 {
                let length = rng.below(50);
                let body = rng.bytes(length);
                let mut chunks = Vec::new();
                let mut rest = &body[..];
                while !rest.is_empty() {
                    let take = 1 + rng.below(rest.len().min(10));
                    chunks.push(rest[..take].to_vec());
                    rest = &rest[take..];
                }
                chunks.push(Vec::new());
                let status = if seed % 2 == 0 { 200 } else { 200 + request as u16 };
                queues.push((request, status, chunks, body));
            }
            let mut delivered: Vec<(u32, u16, Vec<u8>)> = Vec::new();
            while queues.iter().any(|(_, _, chunks, _)| !chunks.is_empty()) {
                let pick = rng.below(queues.len());
                let (request, status, chunks, _) = &mut queues[pick];
                if chunks.is_empty() {
                    continue;
                }
                let bytes = chunks.remove(0);
                let last = chunks.is_empty();
                let (events, commands) = app.dispatch(chunk(*request, *status, last, &bytes));
                assert!(commands.is_empty(), "seed {seed}");
                for event in events {
                    let Event::Fetched(response) = event else {
                        panic!("seed {seed}: not a response")
                    };
                    assert!(last, "seed {seed}: delivered before the last chunk");
                    delivered.push((response.request, response.status, response.body));
                }
            }
            delivered.sort_by_key(|(request, ..)| *request);
            let expected: Vec<_> = queues
                .into_iter()
                .map(|(request, status, _, body)| (request, status, body))
                .collect();
            assert_eq!(delivered, expected, "seed {seed}");
            assert!(app.responses.is_empty(), "seed {seed}");
        }
    }

    #[test]
    fn arbitrary_frames_never_panic_and_the_event_queue_stays_drained() {
        for seed in 0..400 {
            let mut rng = Rng::new(seed);
            let mut app = new_app();
            for step in 0..60 {
                let frame = match rng.below(4) {
                    0 => {
                        let id = rng.id();
                        pointer_frame(
                            rng.below(40) as u8,
                            &id,
                            rng.below(400) as f32,
                            rng.below(3) as u32,
                            step as f64 * 30.0,
                        )
                    }
                    1 => {
                        let length = rng.below(60);
                        rng.bytes(length)
                    }
                    2 => {
                        let length = rng.below(20);
                        let bytes = rng.bytes(length);
                        let mut frame = Vec::new();
                        frame.push(crate::event::EVENT_FETCH);
                        (1 + rng.below(3) as u32).encode(&mut frame);
                        frame.extend_from_slice(&(200 + rng.below(300) as u16).to_le_bytes());
                        frame.push(rng.below(2) as u8);
                        <[u8]>::encode(&bytes, &mut frame);
                        frame
                    }
                    _ => {
                        let mut frame = Vec::new();
                        frame.push(2 + rng.below(3) as u8);
                        (rng.below(800) as f32).encode(&mut frame);
                        (rng.below(800) as f32).encode(&mut frame);
                        frame
                    }
                };
                app.clear();
                app.process(&frame);
                assert!(app.events.is_empty(), "seed {seed} step {step}");
            }
        }
    }

    #[test]
    fn a_lost_store_is_remembered_once_for_the_serve_loop() {
        let mut app = new_app();
        assert!(app.reopen_wanted().is_none());
        let id = StoreId { name: "x", version: "0.0" };
        app.run(Event::StoreLost(id));
        assert_eq!(app.reopen_wanted().map(|id| (id.name, id.version)), Some(("x", "0.0")));
        assert!(app.reopen_wanted().is_none());
        app.run(Event::StoreOpened(block_on(Backend::open(id, true))));
        assert!(app.events.is_empty());
    }
}
