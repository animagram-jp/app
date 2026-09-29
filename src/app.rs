use alloc::{collections::VecDeque, vec, vec::Vec};
use core::{
    default::Default,
    iter::Extend,
    option::Option::{None, Some},
    primitive::{bool, f64, u8},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

use crate::{
    Error,
    arena::{APP, RUNNING, emit},
    event::{Event, EventError, WindowEvent, decode_event},
    handler::Handler,
    js_client::{
        CanvasEvent, Command, EventType, Thresholds, TouchTracker, detect_device, encode_command,
    },
};

// === App ===

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct App {
    touch:      TouchTracker,
    thresholds: Thresholds,
    events:     VecDeque<Event>,
    handler:    Handler,
    commands:   Vec<Command>,
    origin:     Option<CanvasEvent>,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl App {
    pub async fn init(pointer_coarse: bool, viewport_width: f64, viewport_height: f64) {
        let app = App::new(pointer_coarse, Handler::ready(viewport_width, viewport_height).await);

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

    /// process(Event) and FIFO queue command (layout `[event:u8][payload...]`)
    ///
    /// ```no_run
    /// # async fn example() {
    /// # use app::app::App;
    /// # use app::arena::APP;
    /// # use app::js_client::Command;
    /// App::init(false, 0.0, 0.0).await;
    /// let app = unsafe { (*(&raw mut APP)).as_mut() }.unwrap();
    /// app.clear();
    /// app.process(&[]);
    /// assert!(matches!(app.commands()[0], Command::Error { .. }));
    /// # }
    /// ```
    pub fn process(&mut self, frame: &[u8]) {
        let Some(event) = decode_event(frame) else {
            self.commands.push(Command::Error { error: Error::Event(EventError::Decode) });
            return;
        };

        self.events.push_back(event);

        while let Some(event) = self.events.pop_front() {
            let (new_events, new_commands) = self.dispatch(event);
            self.events.extend(new_events);
            self.commands.extend(new_commands);
        }
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }

    fn dispatch(&mut self, event: Event) -> (Vec<Event>, Vec<Command>) {
        let Self { handler, touch, thresholds, origin, .. } = self;

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
        }
    }

    pub fn commands(&self) -> &[Command] {
        &self.commands
    }
}

#[cfg(all(test, not(feature = "worker")))]
mod tests {
    use alloc::vec::Vec;
    use core::{
        future::Future,
        pin::pin,
        task::{Context, Poll, Waker},
    };

    use super::*;
    use crate::{
        event::EVENT_CANVAS,
        js_client::{Gesture, dom, put_f32, put_str, put_u32},
    };

    const POINTER_DOWN: u8 = 10;
    const POINTER_UP: u8 = 12;

    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
                return output;
            }
        }
    }

    fn new_app() -> App {
        App::new(false, block_on(Handler::ready(0.0, 0.0)))
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
        put_str(&mut frame, "");
        put_f32(&mut frame, x);
        put_f32(&mut frame, 0.0);
        put_f32(&mut frame, x);
        put_f32(&mut frame, 0.0);
        frame.extend_from_slice(&time.to_le_bytes());
        put_u32(&mut frame, pointer_id);
        frame
    }

    fn origin_id(app: &App) -> Option<dom::Id> {
        app.origin.as_ref().map(|origin| origin.id.clone())
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

    #[test]
    fn the_event_queue_is_empty_after_a_whole_tap() {
        let mut app = new_app();

        app.process(&pointer_frame(POINTER_DOWN, &section(1), 10.0, 1, 0.0));
        app.process(&pointer_frame(POINTER_UP, &section(1), 10.0, 1, 50.0));
        assert!(app.events.is_empty());
    }
}
