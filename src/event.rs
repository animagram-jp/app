use alloc::{string::String, vec, vec::Vec};
use core::{
    fmt::{self, Debug, Display, Formatter},
    future::Future,
    option::Option,
    primitive::{f64, i32, u8},
};

use self::gesture::{Gesture, Pointer};
use crate::{
    WireError,
    file_store::{FileStore, FileStoreError, StoreId},
    host::{CanvasEvent, Command, FullscreenEvent, VisibilityState},
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

pub enum Event {
    /// Event originating from the DOM.
    Canvas(CanvasEvent),
    /// A recognized gesture. Pushed by `dispatch` itself.
    Gesture(Gesture),
    Window(WindowEvent),
    FetchChunk(FetchChunk),
    Fetched(Response),
    StoreLost(StoreId),
}

pub type Opened<S> = Result<<S as FileStore>::Handle, FileStoreError>;

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

// === handler ===

pub trait Handler: Sized {
    type Store: FileStore;
    fn ready(
        viewport_width: f64,
        viewport_height: f64,
        rem_in_px: f64,
        now: f64,
        timezone_offset_minutes: i32,
    ) -> impl Future<Output = Self>;
    fn initial_draw(&mut self) -> (Vec<Event>, Vec<Command>);
    fn close(&self) -> Vec<Command>;
    fn process_canvas(
        &mut self,
        event: &CanvasEvent,
        pointer: &Pointer,
    ) -> (Vec<Event>, Vec<Command>);

    fn process_gesture(
        &mut self,
        _gesture: &Gesture,
        _pointer: &Pointer,
        _origin: Option<&CanvasEvent>,
    ) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    fn process_resize(&mut self, _width: f64, _height: f64) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    fn process_scroll(&mut self, _x: f64, _y: f64) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    fn process_visibility(&mut self, _state: VisibilityState) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    fn process_fullscreen(&mut self, _event: FullscreenEvent) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    fn process_fetched(&mut self, _response: &Response) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    fn process_lost(&mut self) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }

    fn process_opened(&mut self, _opened: Opened<Self::Store>) -> (Vec<Event>, Vec<Command>) {
        (vec![], vec![])
    }
}

// === store ===

pub mod store {
    use alloc::{vec, vec::Vec};
    use core::{
        mem::take,
        option::Option::{self, None, Some},
        primitive::{u8, u32},
        result::Result::{self, Err, Ok},
    };

    use crate::{
        Error,
        event::{Event, Opened},
        file_store::{FileStore, FileStoreError, StoreId},
        host::Command,
    };

    pub async fn open<S: FileStore>(id: StoreId) -> Result<S, FileStoreError> {
        S::open(id, true).await.and_then(S::new)
    }

    pub fn lost<S: FileStore>(store: &mut Option<S>) -> Vec<(u32, Option<Vec<u8>>)> {
        match store.take() {
            Some(store) => {
                let pending = store.pending();
                store.close();
                pending
            }
            None => Vec::new(),
        }
    }

    pub fn opened<S: FileStore>(
        store: &mut Option<S>,
        lost: &mut Vec<(u32, Option<Vec<u8>>)>,
        opened: Opened<S>,
    ) -> (Vec<Event>, Vec<Command>) {
        match opened.and_then(S::new) {
            Ok(mut reopened) => {
                reopened.replay(take(lost));
                *store = Some(reopened);
                (vec![], vec![])
            }
            Err(error) => {
                (vec![], vec![Command::Error { error: Error::FileStore(error) }, Command::Reload])
            }
        }
    }

    pub fn failed(id: StoreId, error: FileStoreError) -> (Vec<Event>, Vec<Command>) {
        match error {
            FileStoreError::InvalidState(_) => (
                vec![Event::StoreLost(id)],
                vec![Command::Error { error: Error::FileStore(error) }],
            ),
            _ => (vec![], vec![Command::Error { error: Error::FileStore(error) }]),
        }
    }
}

// === gesture (See Gesture.md) ===

pub mod gesture {
    use core::{
        clone::Clone,
        cmp::{Eq, PartialEq},
        default::Default,
        fmt::Debug,
        marker::Copy,
        mem::take,
        option::Option::{self, None, Some},
        primitive::{bool, f64, u32},
    };

    use crate::host::EventType;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Device {
        Touch,
        Mouse,
    }

    pub fn detect_device(pointer_coarse: bool) -> Device {
        if pointer_coarse { Device::Touch } else { Device::Mouse }
    }

    // === gesture: tap, long press, swipe (up,down,left,right), drag (See Gesture.md) ===

    #[derive(Debug, Clone, Copy)]
    pub struct Thresholds {
        pub long_press_ms:      f64,
        pub long_press_slop_px: f64,
        pub drag_start_px:      f64,
        pub swipe_min_px:       f64,
        pub swipe_min_velocity: f64,
        pub swipe_max_ms:       f64,
        pub tap_max_ms:         f64,
        pub tap_slop_px:        f64,
    }

    impl Thresholds {
        pub const MOUSE: Self = Self {
            long_press_ms:      251.0,
            long_press_slop_px: 9.0,
            drag_start_px:      10.0,
            swipe_min_px:       50.0,
            swipe_min_velocity: 0.5,
            swipe_max_ms:       250.0,
            tap_max_ms:         250.0,
            tap_slop_px:        9.0,
        };

        pub const TOUCH: Self = Self {
            long_press_ms:      500.0,
            long_press_slop_px: 16.0,
            drag_start_px:      16.0,
            swipe_min_px:       50.0,
            swipe_min_velocity: 0.5,
            swipe_max_ms:       300.0,
            tap_max_ms:         300.0,
            tap_slop_px:        16.0,
        };

        #[must_use]
        pub const fn for_device(device: Device) -> Self {
            match device {
                Device::Mouse => Self::MOUSE,
                Device::Touch => Self::TOUCH,
            }
        }
    }

    impl Default for Thresholds {
        fn default() -> Self {
            Self::MOUSE
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Gesture {
        Tap,
        LongPress,
        SwipeUp,
        SwipeDown,
        SwipeLeft,
        SwipeRight,
        Drag { x: f64, y: f64 },
        DragEnd,
        DragCancel,
        Pinch { scale: f64, center_x: f64, center_y: f64 },
        PinchEnd,
    }

    #[cfg(test)]
    mod gesture_tests {
        use alloc::vec::Vec;

        use super::*;

        fn run(events: &[(EventType, f64, f64, f64)], th: &Thresholds) -> Vec<Gesture> {
            let mut pointer = Pointer::default();
            let mut out = Vec::new();
            for (event_type, x, y, time) in events {
                pointer.update(event_type, *x, *y, *time);
                if let Some(g) = pointer.detect(event_type, *time, th) {
                    out.push(g);
                }
            }
            out
        }

        #[test]
        fn swipe_right_fires() {
            let th = Thresholds::MOUSE;
            let got = run(
                &[
                    (EventType::PointerDown, 100.0, 100.0, 0.0),
                    (EventType::PointerMove, 200.0, 100.0, 50.0),
                    (EventType::PointerUp, 260.0, 100.0, 100.0),
                ],
                &th,
            );
            assert_eq!(got, [Gesture::SwipeRight]);
        }

        #[test]
        fn swipe_without_move_event() {
            let th = Thresholds::MOUSE;
            let got = run(
                &[
                    (EventType::PointerDown, 100.0, 100.0, 0.0),
                    (EventType::PointerUp, 260.0, 100.0, 100.0),
                ],
                &th,
            );
            assert_eq!(got, [Gesture::SwipeRight]);
        }

        #[test]
        fn swipe_fires_when_motion_continues_to_release() {
            let th = Thresholds::MOUSE;
            let got = run(
                &[
                    (EventType::PointerDown, 0.0, 0.0, 0.0),
                    (EventType::PointerMove, 100.0, 0.0, 50.0),
                    (EventType::PointerMove, 200.0, 0.0, 100.0),
                    (EventType::PointerUp, 260.0, 0.0, 120.0),
                ],
                &th,
            );
            assert_eq!(got, [Gesture::SwipeRight]);
        }

        #[test]
        fn swipe_does_not_fire_after_stopping_before_release() {
            let th = Thresholds::MOUSE;
            let got = run(
                &[
                    (EventType::PointerDown, 0.0, 0.0, 0.0),
                    (EventType::PointerMove, 150.0, 0.0, 20.0),
                    (EventType::PointerUp, 150.0, 0.0, 249.0),
                ],
                &th,
            );
            assert_eq!(got, []);
        }

        // --- drag ---

        #[test]
        fn slow_move_is_drag() {
            let th = Thresholds::MOUSE;
            let got = run(
                &[
                    (EventType::PointerDown, 100.0, 100.0, 0.0),
                    (EventType::PointerMove, 150.0, 100.0, 400.0),
                    (EventType::PointerMove, 200.0, 100.0, 800.0),
                    (EventType::PointerUp, 200.0, 100.0, 900.0),
                ],
                &th,
            );
            assert_eq!(
                got,
                [
                    Gesture::Drag { x: 150.0, y: 100.0 },
                    Gesture::Drag { x: 200.0, y: 100.0 },
                    Gesture::DragEnd,
                ]
            );
        }

        #[test]
        fn cancel_is_distinct_from_end() {
            let th = Thresholds::MOUSE;
            let got = run(
                &[
                    (EventType::PointerDown, 100.0, 100.0, 0.0),
                    (EventType::PointerMove, 150.0, 100.0, 400.0),
                    (EventType::PointerCancel, 150.0, 100.0, 500.0),
                ],
                &th,
            );
            assert_eq!(got, [Gesture::Drag { x: 150.0, y: 100.0 }, Gesture::DragCancel]);
        }

        // --- tap ---

        #[test]
        fn quick_press_is_tap() {
            let th = Thresholds::MOUSE;
            let got = run(
                &[
                    (EventType::PointerDown, 100.0, 100.0, 0.0),
                    (EventType::PointerUp, 101.0, 100.0, 50.0),
                ],
                &th,
            );
            assert_eq!(got, [Gesture::Tap]);
        }

        // --- long press ---

        #[test]
        fn long_press_fires_on_release() {
            let th = Thresholds::MOUSE;
            let got = run(
                &[
                    (EventType::PointerDown, 10.0, 10.0, 0.0),
                    (EventType::PointerUp, 10.0, 10.0, 400.0),
                ],
                &th,
            );
            assert_eq!(got, [Gesture::LongPress]);
        }

        #[test]
        fn long_press_fires_on_move_after_hold() {
            let th = Thresholds::MOUSE;
            let got = run(
                &[
                    (EventType::PointerDown, 10.0, 10.0, 0.0),
                    (EventType::PointerMove, 11.0, 10.0, 300.0),
                ],
                &th,
            );
            assert_eq!(got, [Gesture::LongPress]);
        }

        #[test]
        fn long_press_does_not_repeat() {
            let th = Thresholds::MOUSE;
            let got = run(
                &[
                    (EventType::PointerDown, 10.0, 10.0, 0.0),
                    (EventType::PointerMove, 11.0, 10.0, 300.0),
                    (EventType::PointerMove, 11.0, 10.0, 400.0),
                    (EventType::PointerUp, 11.0, 10.0, 500.0),
                ],
                &th,
            );
            assert_eq!(got, [Gesture::LongPress]);
        }

        #[test]
        fn long_press_suppressed_while_dragging() {
            let th = Thresholds::MOUSE;
            let got = run(
                &[
                    (EventType::PointerDown, 10.0, 10.0, 0.0),
                    (EventType::PointerMove, 40.0, 10.0, 100.0),
                    (EventType::PointerMove, 40.0, 10.0, 400.0),
                ],
                &th,
            );
            assert_eq!(
                got,
                [Gesture::Drag { x: 40.0, y: 10.0 }, Gesture::Drag { x: 40.0, y: 10.0 }]
            );
        }

        #[test]
        fn touch_thresholds_are_looser() {
            let mouse = Thresholds::for_device(Device::Mouse);
            let touch = Thresholds::for_device(Device::Touch);
            assert!(touch.long_press_ms > mouse.long_press_ms);
            assert!(touch.long_press_slop_px > mouse.long_press_slop_px);
            assert!(touch.drag_start_px > mouse.drag_start_px);
        }

        #[test]
        fn same_input_differs_by_device() {
            let events = [
                (EventType::PointerDown, 10.0, 10.0, 0.0),
                (EventType::PointerUp, 10.0, 10.0, 300.0),
            ];
            assert_eq!(run(&events, &Thresholds::MOUSE), [Gesture::LongPress]);
            assert_eq!(run(&events, &Thresholds::TOUCH), []);
        }
    }

    // === gesture: two-finger (pinch / pan) ===
    //
    //

    #[derive(Debug, Clone, Copy)]
    struct TouchPoint {
        id:        u32,
        start_x:   f64,
        start_y:   f64,
        current_x: f64,
        current_y: f64,
    }

    impl TouchPoint {
        const fn new(id: u32, x: f64, y: f64) -> Self {
            Self { id, start_x: x, start_y: y, current_x: x, current_y: y }
        }

        fn displacement(&self) -> (f64, f64) {
            (self.current_x - self.start_x, self.current_y - self.start_y)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    enum TwoFingerMode {
        #[default]
        Undetermined,
        Pan,
        Pinch,
    }

    const TWO_FINGER_COMMIT_PX: f64 = 8.0;

    const PINCH_DOT_THRESHOLD: f64 = 0.0;

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum FoldedInput {
        AsSinglePoint { x: f64, y: f64 },
        Pinch { scale: f64, center_x: f64, center_y: f64 },
        None,
    }

    #[derive(Debug, Clone, Copy, Default)]
    struct TwoFingerState {
        primary:   Option<TouchPoint>,
        secondary: Option<TouchPoint>,
        mode:      TwoFingerMode,
    }

    impl TwoFingerState {
        #[must_use]
        fn add_touch(self, id: u32, x: f64, y: f64) -> Self {
            match (self.primary, self.secondary) {
                (None, _) => Self { primary: Some(TouchPoint::new(id, x, y)), ..self },
                (Some(_), None) => Self {
                    secondary: Some(TouchPoint::new(id, x, y)),
                    mode: TwoFingerMode::Undetermined,
                    ..self
                },
                (Some(_), Some(_)) => self,
            }
        }

        #[must_use]
        fn move_touch(self, id: u32, x: f64, y: f64) -> Self {
            if self.primary.is_some_and(|p| p.id == id) {
                Self {
                    primary: self.primary.map(|p| TouchPoint { current_x: x, current_y: y, ..p }),
                    ..self
                }
            } else if self.secondary.is_some_and(|s| s.id == id) {
                Self {
                    secondary: self.secondary.map(|s| TouchPoint {
                        current_x: x,
                        current_y: y,
                        ..s
                    }),
                    ..self
                }
            } else {
                self
            }
        }

        #[must_use]
        fn remove_touch(self, id: u32) -> (Self, TwoFingerMode) {
            let ended_mode = self.mode;
            if self.primary.is_some_and(|p| p.id == id) {
                (
                    Self {
                        primary:   self.secondary,
                        secondary: None,
                        mode:      TwoFingerMode::Undetermined,
                    },
                    ended_mode,
                )
            } else if self.secondary.is_some_and(|s| s.id == id) {
                (Self { secondary: None, mode: TwoFingerMode::Undetermined, ..self }, ended_mode)
            } else {
                (self, TwoFingerMode::Undetermined)
            }
        }

        #[must_use]
        fn primary_id(&self) -> Option<u32> {
            self.primary.map(|p| p.id)
        }

        #[must_use]
        fn secondary_id(&self) -> Option<u32> {
            self.secondary.map(|p| p.id)
        }

        #[must_use]
        fn primary_position(&self) -> Option<(f64, f64)> {
            self.primary.map(|p| (p.current_x, p.current_y))
        }

        #[must_use]
        fn fold(&mut self) -> FoldedInput {
            let (Some(p), Some(s)) = (self.primary, self.secondary) else {
                return FoldedInput::None;
            };

            if self.mode == TwoFingerMode::Undetermined {
                let d1 = p.displacement();
                let d2 = s.displacement();
                let moved1 = libm::sqrt(d1.0 * d1.0 + d1.1 * d1.1) > TWO_FINGER_COMMIT_PX;
                let moved2 = libm::sqrt(d2.0 * d2.0 + d2.1 * d2.1) > TWO_FINGER_COMMIT_PX;
                if !(moved1 && moved2) {
                    return FoldedInput::None;
                }
                let dot = d1.0 * d2.0 + d1.1 * d2.1;
                self.mode = if dot < PINCH_DOT_THRESHOLD {
                    TwoFingerMode::Pinch
                } else {
                    TwoFingerMode::Pan
                };
            }

            match self.mode {
                TwoFingerMode::Undetermined => FoldedInput::None,
                TwoFingerMode::Pan => FoldedInput::AsSinglePoint {
                    x: (p.current_x + s.current_x) / 2.0,
                    y: (p.current_y + s.current_y) / 2.0,
                },
                TwoFingerMode::Pinch => {
                    let start_distance =
                        measure_distance(p.start_x, p.start_y, s.start_x, s.start_y);
                    if start_distance <= 0.0 {
                        return FoldedInput::None;
                    }
                    let current_distance =
                        measure_distance(p.current_x, p.current_y, s.current_x, s.current_y);
                    FoldedInput::Pinch {
                        scale:    current_distance / start_distance,
                        center_x: (p.current_x + s.current_x) / 2.0,
                        center_y: (p.current_y + s.current_y) / 2.0,
                    }
                }
            }
        }
    }

    fn measure_distance(x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
        let dx = x1 - x0;
        let dy = y1 - y0;
        libm::sqrt(dx * dx + dy * dy)
    }

    #[derive(Debug, Default)]
    pub struct Pointer {
        is_down:          bool,
        start_x:          f64,
        start_y:          f64,
        current_x:        f64,
        current_y:        f64,
        start_time:       f64,
        last_move_x:      f64,
        last_move_y:      f64,
        last_move_time:   f64,
        is_dragging:      bool,
        long_press_fired: bool,
        cancelled:        bool,
        two_fingers:      TwoFingerState,
        panning:          bool,
    }

    impl Pointer {
        #[must_use]
        pub fn track(
            &mut self,
            event_type: &EventType,
            pointer_id: u32,
            x: f64,
            y: f64,
            time: f64,
            thresholds: &Thresholds,
        ) -> Option<Gesture> {
            match event_type {
                EventType::PointerDown => {
                    self.track_down(pointer_id, x, y, time);
                    None
                }
                EventType::PointerMove => self.track_move(pointer_id, x, y, time, thresholds),
                EventType::PointerUp | EventType::PointerCancel => {
                    self.track_up(event_type, pointer_id, x, y, time, thresholds)
                }
                _ => None,
            }
        }

        #[must_use]
        pub const fn is_down(&self) -> bool {
            self.is_down
        }

        #[must_use]
        pub const fn current(&self) -> (f64, f64) {
            (self.current_x, self.current_y)
        }

        fn distance(&self) -> f64 {
            measure_distance(self.start_x, self.start_y, self.current_x, self.current_y)
        }

        fn update(&mut self, event_type: &EventType, x: f64, y: f64, time: f64) {
            match event_type {
                EventType::PointerDown => {
                    *self = Self {
                        is_down:          true,
                        start_x:          x,
                        start_y:          y,
                        current_x:        x,
                        current_y:        y,
                        start_time:       time,
                        last_move_x:      x,
                        last_move_y:      y,
                        last_move_time:   time,
                        is_dragging:      false,
                        long_press_fired: false,
                        cancelled:        false,
                        two_fingers:      self.two_fingers,
                        panning:          self.panning,
                    };
                }
                EventType::PointerMove => {
                    self.current_x = x;
                    self.current_y = y;
                    self.last_move_x = x;
                    self.last_move_y = y;
                    self.last_move_time = time;
                }
                EventType::PointerUp | EventType::PointerCancel => {
                    self.is_down = false;
                    self.current_x = x;
                    self.current_y = y;
                    self.cancelled = matches!(event_type, EventType::PointerCancel);
                }
                _ => {}
            }
        }

        fn clear(&mut self) {
            *self =
                Self { two_fingers: self.two_fingers, panning: self.panning, ..Self::default() };
        }

        fn detect(
            &mut self,
            event_type: &EventType,
            current_time: f64,
            thresholds: &Thresholds,
        ) -> Option<Gesture> {
            match event_type {
                EventType::PointerUp | EventType::PointerCancel => {
                    self.detect_release(current_time, thresholds)
                }
                EventType::PointerMove => self.detect_move(current_time, thresholds),
                _ => None,
            }
        }

        fn detect_release(
            &mut self,
            current_time: f64,
            thresholds: &Thresholds,
        ) -> Option<Gesture> {
            if self.is_dragging {
                self.is_dragging = false;
                return Some(if self.cancelled { Gesture::DragCancel } else { Gesture::DragEnd });
            }

            if self.cancelled {
                return None;
            }

            let dt = current_time - self.start_time;
            if dt <= 0.0 {
                return None;
            }
            let distance = self.distance();

            let move_dt = current_time - self.last_move_time;
            let velocity = if move_dt > 0.0 {
                let mdx = self.current_x - self.last_move_x;
                let mdy = self.current_y - self.last_move_y;
                libm::sqrt(mdx * mdx + mdy * mdy) / move_dt
            } else {
                0.0
            };
            if velocity > thresholds.swipe_min_velocity
                && distance > thresholds.swipe_min_px
                && dt < thresholds.swipe_max_ms
            {
                let dx = self.current_x - self.start_x;
                let dy = self.current_y - self.start_y;
                return Some(if libm::fabs(dx) > libm::fabs(dy) {
                    if dx > 0.0 { Gesture::SwipeRight } else { Gesture::SwipeLeft }
                } else if dy > 0.0 {
                    Gesture::SwipeDown
                } else {
                    Gesture::SwipeUp
                });
            }

            if self.long_press_fired {
                return None;
            }

            if dt > thresholds.long_press_ms && distance < thresholds.long_press_slop_px {
                return Some(Gesture::LongPress);
            }

            if dt < thresholds.tap_max_ms && distance < thresholds.tap_slop_px {
                return Some(Gesture::Tap);
            }

            None
        }

        fn detect_move(&mut self, current_time: f64, thresholds: &Thresholds) -> Option<Gesture> {
            if !self.is_down {
                return None;
            }

            let distance = self.distance();

            if !self.long_press_fired
                && !self.is_dragging
                && distance < thresholds.long_press_slop_px
                && current_time - self.start_time > thresholds.long_press_ms
            {
                self.long_press_fired = true;
                return Some(Gesture::LongPress);
            }

            if distance <= thresholds.drag_start_px {
                return None;
            }

            if self.is_dragging {
                return Some(Gesture::Drag { x: self.current_x, y: self.current_y });
            }

            let dt = current_time - self.start_time;
            if dt > 0.0 && dt < thresholds.swipe_max_ms {
                let velocity = distance / dt;
                if velocity > thresholds.swipe_min_velocity && distance > thresholds.swipe_min_px {
                    return None;
                }
            }

            self.is_dragging = true;
            Some(Gesture::Drag { x: self.current_x, y: self.current_y })
        }

        fn track_down(&mut self, id: u32, x: f64, y: f64, time: f64) {
            if self.two_fingers.primary_id() == Some(id)
                || self.two_fingers.secondary_id() == Some(id)
            {
                self.two_fingers = self.two_fingers.move_touch(id, x, y);
                return;
            }
            if self.two_fingers.primary_id().is_none() {
                self.update(&EventType::PointerDown, x, y, time);
            }
            self.two_fingers = self.two_fingers.add_touch(id, x, y);
        }

        fn track_move(
            &mut self,
            id: u32,
            x: f64,
            y: f64,
            time: f64,
            thresholds: &Thresholds,
        ) -> Option<Gesture> {
            let is_primary = self.two_fingers.primary_id() == Some(id);
            let is_secondary = self.two_fingers.secondary_id() == Some(id);
            if !is_primary && !is_secondary {
                return None;
            }
            self.two_fingers = self.two_fingers.move_touch(id, x, y);

            if self.two_fingers.secondary_id().is_some() {
                return self.track_fold(time, thresholds);
            }

            self.update(&EventType::PointerMove, x, y, time);
            self.detect(&EventType::PointerMove, time, thresholds)
        }

        fn track_up(
            &mut self,
            event_type: &EventType,
            id: u32,
            x: f64,
            y: f64,
            time: f64,
            thresholds: &Thresholds,
        ) -> Option<Gesture> {
            let is_primary = self.two_fingers.primary_id() == Some(id);
            let is_secondary = self.two_fingers.secondary_id() == Some(id);
            let had_secondary = self.two_fingers.secondary_id().is_some();

            if is_secondary || (is_primary && had_secondary) {
                let (next, ended_mode) = self.two_fingers.remove_touch(id);
                self.two_fingers = next;
                let gesture = self.end_session(event_type, ended_mode, time, thresholds);
                self.resync_primary(time);
                gesture
            } else if is_primary {
                self.update(event_type, x, y, time);
                let gesture = self.detect(event_type, time, thresholds);
                self.two_fingers = self.two_fingers.remove_touch(id).0;
                gesture
            } else {
                None
            }
        }

        fn end_session(
            &mut self,
            event_type: &EventType,
            ended_mode: TwoFingerMode,
            time: f64,
            thresholds: &Thresholds,
        ) -> Option<Gesture> {
            match ended_mode {
                TwoFingerMode::Undetermined => None,
                TwoFingerMode::Pinch => Some(Gesture::PinchEnd),
                TwoFingerMode::Pan => {
                    if !take(&mut self.panning) {
                        return None;
                    }
                    let (cx, cy) = self.current();
                    self.update(event_type, cx, cy, time);
                    self.detect(event_type, time, thresholds)
                }
            }
        }

        fn resync_primary(&mut self, time: f64) {
            match self.two_fingers.primary_position() {
                Some((x, y)) => self.update(&EventType::PointerDown, x, y, time),
                None => self.clear(),
            }
        }

        fn track_fold(&mut self, time: f64, thresholds: &Thresholds) -> Option<Gesture> {
            match self.two_fingers.fold() {
                FoldedInput::None => None,
                FoldedInput::Pinch { scale, center_x, center_y } => {
                    Some(Gesture::Pinch { scale, center_x, center_y })
                }
                FoldedInput::AsSinglePoint { x, y } => {
                    if !self.panning {
                        self.panning = true;
                        self.update(&EventType::PointerDown, x, y, time);
                        return None;
                    }
                    self.update(&EventType::PointerMove, x, y, time);
                    self.detect(&EventType::PointerMove, time, thresholds)
                }
            }
        }
    }

    #[cfg(test)]
    mod two_finger_tests {
        use super::*;

        #[test]
        fn symmetric_pinch_out_increases_scale() {
            let mut state =
                TwoFingerState::default().add_touch(1, 140.0, 100.0).add_touch(2, 160.0, 100.0);
            state = state.move_touch(1, 100.0, 100.0).move_touch(2, 200.0, 100.0);
            match state.fold() {
                FoldedInput::Pinch { scale, .. } => assert!(scale > 1.0, "scale = {scale}"),
                other => panic!("expected Pinch, got {other:?}"),
            }
        }

        #[test]
        fn mode_latches_after_commit() {
            let mut state =
                TwoFingerState::default().add_touch(1, 100.0, 100.0).add_touch(2, 200.0, 100.0);
            state = state.move_touch(1, 140.0, 100.0).move_touch(2, 160.0, 100.0);
            assert!(matches!(state.fold(), FoldedInput::Pinch { .. }));

            state = state.move_touch(1, 140.0, 100.0).move_touch(2, 140.0, 100.0);
            assert!(matches!(state.fold(), FoldedInput::Pinch { .. }));
        }

        #[test]
        fn new_session_reclassifies_independently() {
            let mut state =
                TwoFingerState::default().add_touch(1, 100.0, 100.0).add_touch(2, 200.0, 100.0);
            state = state.move_touch(1, 140.0, 100.0).move_touch(2, 160.0, 100.0);
            assert!(matches!(state.fold(), FoldedInput::Pinch { .. }));

            state = state.remove_touch(2).0.remove_touch(1).0;
            state = state.add_touch(3, 0.0, 0.0).add_touch(4, 50.0, 0.0);
            state = state.move_touch(3, 0.0, 50.0).move_touch(4, 50.0, 50.0);
            assert_eq!(state.fold(), FoldedInput::AsSinglePoint { x: 25.0, y: 50.0 });
        }
    }

    #[cfg(test)]
    mod pointer_tests {
        use alloc::vec::Vec;

        use super::*;
        use crate::Rng;

        fn run(
            events: &[(EventType, u32, f64, f64, f64)],
            th: &Thresholds,
        ) -> Vec<Option<Gesture>> {
            let mut pointer = Pointer::default();
            events
                .iter()
                .map(|(event_type, id, x, y, time)| {
                    pointer.track(event_type, *id, *x, *y, *time, th)
                })
                .collect()
        }

        fn random_press_events(
            rng: &mut Rng,
            pointers: u32,
            steps: usize,
        ) -> (Vec<(EventType, u32, f64, f64, f64)>, Vec<u32>) {
            let mut events = Vec::new();
            let mut down: Vec<u32> = Vec::new();
            let mut time = 0.0;
            for _ in 0..steps {
                time += rng.below(400) as f64;
                let free: Vec<u32> = (1..=pointers).filter(|id| !down.contains(id)).collect();
                let choice = rng.below(10);
                let (event_type, id) = if down.is_empty() || (choice < 3 && !free.is_empty()) {
                    (EventType::PointerDown, free[rng.below(free.len())])
                } else if choice < 7 {
                    (EventType::PointerMove, down[rng.below(down.len())])
                } else if choice < 9 {
                    (EventType::PointerUp, down[rng.below(down.len())])
                } else {
                    (EventType::PointerCancel, down[rng.below(down.len())])
                };
                match event_type {
                    EventType::PointerDown => down.push(id),
                    EventType::PointerUp | EventType::PointerCancel => {
                        down.retain(|held| *held != id)
                    }
                    _ => {}
                }
                events.push((event_type, id, rng.below(300) as f64, rng.below(300) as f64, time));
            }
            (events, down)
        }

        #[test]
        fn a_single_pointer_never_breaks_the_drag_grammar() {
            for seed in 0..3000 {
                let mut rng = Rng::new(seed);
                let th = if rng.chance(50) { Thresholds::MOUSE } else { Thresholds::TOUCH };
                let (events, _) = random_press_events(&mut rng, 1, 40);
                let mut dragging = false;
                for (index, got) in run(&events, &th).into_iter().enumerate() {
                    let Some(gesture) = got else { continue };
                    match gesture {
                        Gesture::Drag { .. } => dragging = true,
                        Gesture::DragEnd | Gesture::DragCancel => {
                            assert!(
                                dragging,
                                "seed {seed} event {index}: {gesture:?} without a drag"
                            );
                            dragging = false;
                        }
                        other => {
                            assert!(
                                !dragging,
                                "seed {seed} event {index}: {other:?} inside a drag"
                            );
                        }
                    }
                }
            }
        }

        #[test]
        fn any_pointer_traffic_ends_idle_once_every_pointer_is_released() {
            for seed in 0..5000 {
                let mut rng = Rng::new(seed);
                let th = if rng.chance(50) { Thresholds::MOUSE } else { Thresholds::TOUCH };
                let (mut events, down) = random_press_events(&mut rng, 3, 30);
                let mut time = events.last().map_or(0.0, |event| event.4);
                for id in down {
                    time += 10.0;
                    events.push((EventType::PointerUp, id, 5.0, 5.0, time));
                }
                time += 5000.0;
                events.push((EventType::PointerDown, 1, 100.0, 100.0, time));
                events.push((EventType::PointerUp, 1, 100.0, 100.0, time + 50.0));
                assert_eq!(run(&events, &th).last(), Some(&Some(Gesture::Tap)), "seed {seed}");
            }
        }

        #[test]
        fn unpaired_pointer_events_never_panic() {
            let kinds = [
                EventType::PointerDown,
                EventType::PointerMove,
                EventType::PointerUp,
                EventType::PointerCancel,
            ];
            for seed in 0..3000 {
                let mut rng = Rng::new(seed);
                let events: Vec<_> = (0..40)
                    .map(|step| {
                        (
                            kinds[rng.below(kinds.len())],
                            rng.below(4) as u32,
                            rng.below(300) as f64,
                            rng.below(300) as f64,
                            step as f64 * rng.below(200) as f64,
                        )
                    })
                    .collect();
                let _ = run(&events, &Thresholds::TOUCH);
            }
        }

        #[test]
        fn second_finger_freezes_primary_until_pinch_commits() {
            let th = Thresholds::MOUSE;
            let got = run(
                &[
                    (EventType::PointerDown, 1, 100.0, 100.0, 0.0),
                    (EventType::PointerDown, 2, 200.0, 100.0, 0.0),
                    (EventType::PointerMove, 1, 150.0, 100.0, 50.0),
                    (EventType::PointerMove, 2, 150.0, 100.0, 60.0),
                ],
                &th,
            );
            assert_eq!(
                got,
                [
                    None,
                    None,
                    None,
                    Some(Gesture::Pinch { scale: 0.0, center_x: 150.0, center_y: 100.0 }),
                ]
            );
        }

        #[test]
        fn pinch_end_on_secondary_release() {
            let th = Thresholds::MOUSE;
            let got = run(
                &[
                    (EventType::PointerDown, 1, 100.0, 100.0, 0.0),
                    (EventType::PointerDown, 2, 200.0, 100.0, 0.0),
                    (EventType::PointerMove, 1, 150.0, 100.0, 50.0),
                    (EventType::PointerMove, 2, 150.0, 100.0, 60.0),
                    (EventType::PointerUp, 2, 150.0, 100.0, 100.0),
                ],
                &th,
            );
            assert_eq!(got[4], Some(Gesture::PinchEnd));
        }

        #[test]
        fn a_second_two_finger_pan_behaves_like_the_first() {
            let th = Thresholds::MOUSE;
            let pan = |offset: f64| {
                [
                    (EventType::PointerDown, 1, 0.0, 0.0, offset),
                    (EventType::PointerDown, 2, 100.0, 0.0, offset),
                    (EventType::PointerMove, 1, 30.0, 0.0, offset + 50.0),
                    (EventType::PointerMove, 2, 130.0, 0.0, offset + 60.0),
                    (EventType::PointerMove, 1, 60.0, 0.0, offset + 120.0),
                    (EventType::PointerUp, 2, 130.0, 0.0, offset + 200.0),
                    (EventType::PointerUp, 1, 60.0, 0.0, offset + 210.0),
                ]
            };
            let events = [pan(0.0), pan(1000.0)].concat();
            let got = run(&events, &th);
            assert_eq!(got[0..7], got[7..14]);
            assert_eq!(got[4], Some(Gesture::Drag { x: 95.0, y: 0.0 }));
        }

        #[test]
        fn two_finger_pan_emits_drag_then_drag_end() {
            let th = Thresholds::MOUSE;
            let got = run(
                &[
                    (EventType::PointerDown, 1, 0.0, 0.0, 0.0),
                    (EventType::PointerDown, 2, 100.0, 0.0, 0.0),
                    (EventType::PointerMove, 1, 30.0, 0.0, 50.0),
                    (EventType::PointerMove, 2, 130.0, 0.0, 60.0),
                    (EventType::PointerMove, 1, 60.0, 0.0, 120.0),
                    (EventType::PointerUp, 2, 130.0, 0.0, 200.0),
                ],
                &th,
            );
            assert_eq!(got[0..4], [None, None, None, None]);
            assert_eq!(got[4], Some(Gesture::Drag { x: 95.0, y: 0.0 }));
            assert_eq!(got[5], Some(Gesture::DragEnd));
        }

        #[test]
        fn primary_release_promotes_and_resyncs_single_finger_tracking() {
            let th = Thresholds::MOUSE;
            let got = run(
                &[
                    (EventType::PointerDown, 1, 100.0, 100.0, 0.0),
                    (EventType::PointerDown, 2, 200.0, 100.0, 0.0),
                    (EventType::PointerMove, 1, 140.0, 100.0, 50.0),
                    (EventType::PointerMove, 2, 160.0, 100.0, 60.0),
                    (EventType::PointerUp, 1, 140.0, 100.0, 70.0),
                    (EventType::PointerMove, 2, 161.0, 100.0, 120.0),
                    (EventType::PointerUp, 2, 161.0, 100.0, 170.0),
                ],
                &th,
            );
            assert_eq!(
                got[3],
                Some(Gesture::Pinch { scale: 0.2, center_x: 150.0, center_y: 100.0 })
            );
            assert_eq!(got[4], Some(Gesture::PinchEnd));
            assert_eq!(got[5], None);
            assert_eq!(got[6], Some(Gesture::Tap));
        }

        #[test]
        fn third_finger_does_not_interfere() {
            let th = Thresholds::MOUSE;
            let got = run(
                &[
                    (EventType::PointerDown, 1, 100.0, 100.0, 0.0),
                    (EventType::PointerDown, 2, 200.0, 100.0, 0.0),
                    (EventType::PointerDown, 3, 300.0, 100.0, 0.0),
                    (EventType::PointerMove, 3, 310.0, 100.0, 10.0),
                    (EventType::PointerUp, 3, 310.0, 100.0, 20.0),
                    (EventType::PointerMove, 1, 140.0, 100.0, 50.0),
                    (EventType::PointerMove, 2, 160.0, 100.0, 60.0),
                ],
                &th,
            );
            assert_eq!(got[0..5], [None, None, None, None, None]);
            assert_eq!(
                got[6],
                Some(Gesture::Pinch { scale: 0.2, center_x: 150.0, center_y: 100.0 })
            );
        }

        #[test]
        fn duplicate_pointer_down_does_not_reset_primary_state() {
            let th = Thresholds::MOUSE;
            let got = run(
                &[
                    (EventType::PointerDown, 1, 100.0, 100.0, 0.0),
                    (EventType::PointerDown, 1, 105.0, 100.0, 10.0),
                    (EventType::PointerMove, 1, 115.0, 100.0, 50.0),
                    (EventType::PointerUp, 1, 115.0, 100.0, 100.0),
                ],
                &th,
            );
            assert_eq!(got[1], None);
            assert_eq!(got[2], Some(Gesture::Drag { x: 115.0, y: 100.0 }));
            assert_eq!(got[3], Some(Gesture::DragEnd));
        }

        #[test]
        fn duplicate_pointer_down_does_not_block_genuine_second_finger() {
            let th = Thresholds::MOUSE;
            let got = run(
                &[
                    (EventType::PointerDown, 1, 100.0, 100.0, 0.0),
                    (EventType::PointerDown, 1, 100.0, 100.0, 5.0),
                    (EventType::PointerDown, 2, 200.0, 100.0, 5.0),
                    (EventType::PointerMove, 1, 140.0, 100.0, 50.0),
                    (EventType::PointerMove, 2, 160.0, 100.0, 60.0),
                ],
                &th,
            );
            assert_eq!(
                got[4],
                Some(Gesture::Pinch { scale: 0.2, center_x: 150.0, center_y: 100.0 })
            );
        }
    }
}
