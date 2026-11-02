use alloc::{string::String, vec, vec::Vec};
use core::{
    fmt::{self, Debug, Display, Formatter},
    future::Future,
    option::Option,
    primitive::{f64, i32, u8, u64},
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
    Resize { width_in_px: f64, height_in_px: f64 },
    Scroll { x_in_px: f64, y_in_px: f64 },
    PageHide,
    Visibility { state: VisibilityState },
    Fullscreen(FullscreenEvent),
}

// === handler ===

pub trait Handler: Sized {
    type Store: FileStore;
    fn ready(
        viewport_width_in_px: f64,
        viewport_height_in_px: f64,
        rem_in_px: f64,
        now_in_ms: u64,
        timezone_offset_minutes: i32,
        data_url: String,
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
        file_store::{FileStore, FileStoreError, Mount, StoreId},
        host::Command,
    };

    pub async fn open<S: Mount>(id: StoreId) -> Result<S, FileStoreError> {
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
                let reload = !matches!(error, FileStoreError::NoModificationAllowed(_));
                let mut commands = vec![Command::Error { error: Error::FileStore(error) }];
                if reload {
                    commands.push(Command::Reload);
                }
                (vec![], commands)
            }
        }
    }

    pub fn retryable(error: &FileStoreError) -> bool {
        matches!(error, FileStoreError::InvalidState(_) | FileStoreError::NoModificationAllowed(_))
    }

    pub fn failed(id: StoreId, error: FileStoreError) -> (Vec<Event>, Vec<Command>) {
        let events = if retryable(&error) { vec![Event::StoreLost(id)] } else { vec![] };
        (events, vec![Command::Error { error: Error::FileStore(error) }])
    }

    pub fn startup(id: StoreId, errors: &[Error]) -> Vec<Event> {
        let retry =
            errors.iter().any(|error| matches!(error, Error::FileStore(error) if retryable(error)));
        if retry { vec![Event::StoreLost(id)] } else { vec![] }
    }
}

// === gesture (See Gesture.md) ===

pub mod gesture {
    use core::{
        clone::Clone,
        cmp::{Eq, PartialEq},
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

    #[derive(Debug, Clone, Copy)]
    pub struct Thresholds {
        pub long_press_in_ms:         f64,
        pub long_press_slop_in_px:    f64,
        pub drag_start_in_px:         f64,
        pub swipe_min_in_px:          f64,
        pub swipe_min_velocity:       f64,
        pub swipe_max_in_ms:          f64,
        pub tap_max_in_ms:            f64,
        pub tap_slop_in_px:           f64,
        pub two_pointer_commit_in_px: f64,
        pub pinch_dot_max:            f64,
    }

    impl Thresholds {
        pub const MOUSE: Self = Self {
            long_press_in_ms:         251.0,
            long_press_slop_in_px:    9.0,
            drag_start_in_px:         10.0,
            swipe_min_in_px:          50.0,
            swipe_min_velocity:       0.5,
            swipe_max_in_ms:          250.0,
            tap_max_in_ms:            250.0,
            tap_slop_in_px:           9.0,
            two_pointer_commit_in_px: 8.0,
            pinch_dot_max:            0.0,
        };

        pub const TOUCH: Self = Self {
            long_press_in_ms:         500.0,
            long_press_slop_in_px:    16.0,
            drag_start_in_px:         16.0,
            swipe_min_in_px:          50.0,
            swipe_min_velocity:       0.5,
            swipe_max_in_ms:          300.0,
            tap_max_in_ms:            300.0,
            tap_slop_in_px:           16.0,
            two_pointer_commit_in_px: 8.0,
            pinch_dot_max:            0.0,
        };

        #[must_use]
        pub const fn for_device(device: Device) -> Self {
            match device {
                Device::Mouse => Self::MOUSE,
                Device::Touch => Self::TOUCH,
            }
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

    fn measure_distance(x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
        let dx = x1 - x0;
        let dy = y1 - y0;
        libm::sqrt(dx * dx + dy * dy)
    }

    #[derive(Debug, Clone, Copy)]
    struct TrackedPointer {
        id:         u32,
        start_x:    f64,
        start_y:    f64,
        start_time: f64,
        current_x:  f64,
        current_y:  f64,
    }

    impl TrackedPointer {
        const fn new(id: u32, x: f64, y: f64, time: f64) -> Self {
            Self { id, start_x: x, start_y: y, start_time: time, current_x: x, current_y: y }
        }

        fn displacement(&self) -> (f64, f64) {
            (self.current_x - self.start_x, self.current_y - self.start_y)
        }

        fn travelled(&self) -> f64 {
            measure_distance(self.start_x, self.start_y, self.current_x, self.current_y)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TwoPointerMode {
        Pan,
        Pinch,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum FoldedInput {
        Pan { x: f64, y: f64 },
        Pinch { scale: f64, center_x: f64, center_y: f64 },
        None,
    }

    #[derive(Debug, Clone, Copy)]
    struct TwoPointerState {
        primary:   Option<TrackedPointer>,
        secondary: Option<TrackedPointer>,
        mode:      Option<TwoPointerMode>,
    }

    impl TwoPointerState {
        const NONE: Self = Self { primary: None, secondary: None, mode: None };

        #[must_use]
        fn add(self, id: u32, x: f64, y: f64, time: f64) -> Self {
            match (self.primary, self.secondary) {
                (None, _) => Self { primary: Some(TrackedPointer::new(id, x, y, time)), ..self },
                (Some(_), None) => Self {
                    secondary: Some(TrackedPointer::new(id, x, y, time)),
                    mode: None,
                    ..self
                },
                (Some(_), Some(_)) => self,
            }
        }

        #[must_use]
        fn move_to(self, id: u32, x: f64, y: f64) -> Self {
            let moved = |p: TrackedPointer| TrackedPointer { current_x: x, current_y: y, ..p };
            if self.primary.is_some_and(|p| p.id == id) {
                Self { primary: self.primary.map(moved), ..self }
            } else if self.secondary.is_some_and(|s| s.id == id) {
                Self { secondary: self.secondary.map(moved), ..self }
            } else {
                self
            }
        }

        #[must_use]
        fn remove(self, id: u32) -> (Self, Option<TwoPointerMode>) {
            if self.primary.is_some_and(|p| p.id == id) {
                (Self { primary: self.secondary, secondary: None, mode: None }, self.mode)
            } else if self.secondary.is_some_and(|s| s.id == id) {
                (Self { secondary: None, mode: None, ..self }, self.mode)
            } else {
                (self, None)
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
        fn primary_start_time(&self) -> Option<f64> {
            self.primary.map(|p| p.start_time)
        }

        #[must_use]
        fn primary_start(&self) -> Option<(f64, f64)> {
            self.primary.map(|p| (p.start_x, p.start_y))
        }

        #[must_use]
        fn fold(&mut self, thresholds: &Thresholds) -> FoldedInput {
            let (Some(p), Some(s)) = (self.primary, self.secondary) else {
                return FoldedInput::None;
            };

            let mode = match self.mode {
                Some(mode) => mode,
                None => {
                    let commit = thresholds.two_pointer_commit_in_px;
                    if !(p.travelled() > commit && s.travelled() > commit) {
                        return FoldedInput::None;
                    }
                    let d1 = p.displacement();
                    let d2 = s.displacement();
                    let dot = d1.0 * d2.0 + d1.1 * d2.1;
                    let mode = if dot < thresholds.pinch_dot_max {
                        TwoPointerMode::Pinch
                    } else {
                        TwoPointerMode::Pan
                    };
                    self.mode = Some(mode);
                    mode
                }
            };

            let center_x = (p.current_x + s.current_x) / 2.0;
            let center_y = (p.current_y + s.current_y) / 2.0;
            match mode {
                TwoPointerMode::Pan => FoldedInput::Pan { x: center_x, y: center_y },
                TwoPointerMode::Pinch => {
                    let start_distance =
                        measure_distance(p.start_x, p.start_y, s.start_x, s.start_y);
                    if start_distance <= 0.0 {
                        return FoldedInput::None;
                    }
                    let current_distance =
                        measure_distance(p.current_x, p.current_y, s.current_x, s.current_y);
                    FoldedInput::Pinch {
                        scale: current_distance / start_distance,
                        center_x,
                        center_y,
                    }
                }
            }
        }
    }

    #[derive(Debug)]
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
        two_pointers:     TwoPointerState,
        panning:          bool,
    }

    impl Pointer {
        pub const NONE: Self = Self {
            is_down:          false,
            start_x:          0.0,
            start_y:          0.0,
            current_x:        0.0,
            current_y:        0.0,
            start_time:       0.0,
            last_move_x:      0.0,
            last_move_y:      0.0,
            last_move_time:   0.0,
            is_dragging:      false,
            long_press_fired: false,
            cancelled:        false,
            two_pointers:     TwoPointerState::NONE,
            panning:          false,
        };

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
                EventType::PointerDown => self.track_down(pointer_id, x, y, time),
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

        fn resynced(&self) -> bool {
            self.two_pointers
                .primary_start()
                .is_some_and(|(x, y)| x != self.start_x || y != self.start_y)
        }

        fn track_down(&mut self, id: u32, x: f64, y: f64, time: f64) -> Option<Gesture> {
            if self.two_pointers.primary_id() == Some(id)
                || self.two_pointers.secondary_id() == Some(id)
            {
                self.two_pointers = self.two_pointers.move_to(id, x, y);
                return None;
            }
            if self.two_pointers.primary_id().is_none() {
                self.start(x, y, time);
            }
            let second = self.two_pointers.secondary_id().is_none();
            self.two_pointers = self.two_pointers.add(id, x, y, time);
            if second && take(&mut self.is_dragging) { Some(Gesture::DragCancel) } else { None }
        }

        fn track_move(
            &mut self,
            id: u32,
            x: f64,
            y: f64,
            time: f64,
            thresholds: &Thresholds,
        ) -> Option<Gesture> {
            let is_primary = self.two_pointers.primary_id() == Some(id);
            let is_secondary = self.two_pointers.secondary_id() == Some(id);
            if !is_primary && !is_secondary {
                return None;
            }
            self.two_pointers = self.two_pointers.move_to(id, x, y);

            if self.two_pointers.secondary_id().is_some() {
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
            let is_primary = self.two_pointers.primary_id() == Some(id);
            let is_secondary = self.two_pointers.secondary_id() == Some(id);
            let had_secondary = self.two_pointers.secondary_id().is_some();

            if is_secondary || (is_primary && had_secondary) {
                let (next, ended) = self.two_pointers.remove(id);
                self.two_pointers = next;
                let gesture = self.end(event_type, ended, time, thresholds);
                self.resync_primary(is_primary);
                gesture
            } else if is_primary {
                self.update(event_type, x, y, time);
                let gesture = self.detect(event_type, time, thresholds);
                self.two_pointers = self.two_pointers.remove(id).0;
                gesture
            } else {
                None
            }
        }

        fn track_fold(&mut self, time: f64, thresholds: &Thresholds) -> Option<Gesture> {
            match self.two_pointers.fold(thresholds) {
                FoldedInput::None => None,
                FoldedInput::Pinch { scale, center_x, center_y } => {
                    Some(Gesture::Pinch { scale, center_x, center_y })
                }
                FoldedInput::Pan { x, y } => {
                    if !self.panning {
                        self.panning = true;
                        self.relocate(x, y);
                        return None;
                    }
                    self.update(&EventType::PointerMove, x, y, time);
                    self.detect(&EventType::PointerMove, time, thresholds)
                }
            }
        }

        fn end(
            &mut self,
            event_type: &EventType,
            ended: Option<TwoPointerMode>,
            time: f64,
            thresholds: &Thresholds,
        ) -> Option<Gesture> {
            match ended {
                None => None,
                Some(TwoPointerMode::Pinch) => Some(Gesture::PinchEnd),
                Some(TwoPointerMode::Pan) => {
                    if !take(&mut self.panning) {
                        return None;
                    }
                    let (cx, cy) = self.current();
                    self.update(event_type, cx, cy, time);
                    self.detect(event_type, time, thresholds)
                }
            }
        }

        fn resync_primary(&mut self, promoted: bool) {
            let Some((x, y)) = self.two_pointers.primary_position() else {
                return self.clear();
            };
            self.relocate(x, y);
            if promoted {
                let time = self.two_pointers.primary_start_time().unwrap_or(self.start_time);
                self.start_time = time;
                self.last_move_time = time;
                self.long_press_fired = false;
            }
        }

        fn start(&mut self, x: f64, y: f64, time: f64) {
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
                two_pointers:     self.two_pointers,
                panning:          self.panning,
            };
        }

        fn relocate(&mut self, x: f64, y: f64) {
            self.start_x = x;
            self.start_y = y;
            self.current_x = x;
            self.current_y = y;
            self.last_move_x = x;
            self.last_move_y = y;
            self.is_dragging = false;
            self.cancelled = false;
        }

        fn update(&mut self, event_type: &EventType, x: f64, y: f64, time: f64) {
            match event_type {
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
            *self = Self { two_pointers: self.two_pointers, panning: self.panning, ..Self::NONE };
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

            if self.cancelled || self.resynced() {
                return None;
            }

            let dt = current_time - self.start_time;
            if dt <= 0.0 {
                return None;
            }
            let distance = self.distance();

            let move_dt = current_time - self.last_move_time;
            let velocity = if move_dt > 0.0 {
                measure_distance(self.last_move_x, self.last_move_y, self.current_x, self.current_y)
                    / move_dt
            } else {
                0.0
            };
            if velocity > thresholds.swipe_min_velocity
                && distance > thresholds.swipe_min_in_px
                && dt < thresholds.swipe_max_in_ms
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

            if dt > thresholds.long_press_in_ms && distance < thresholds.long_press_slop_in_px {
                return Some(Gesture::LongPress);
            }

            if dt < thresholds.tap_max_in_ms && distance < thresholds.tap_slop_in_px {
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
                && !self.resynced()
                && distance < thresholds.long_press_slop_in_px
                && current_time - self.start_time > thresholds.long_press_in_ms
            {
                self.long_press_fired = true;
                return Some(Gesture::LongPress);
            }

            if distance <= thresholds.drag_start_in_px {
                return None;
            }

            if self.is_dragging {
                return Some(Gesture::Drag { x: self.current_x, y: self.current_y });
            }

            let dt = current_time - self.start_time;
            if dt > 0.0 && dt < thresholds.swipe_max_in_ms {
                let velocity = distance / dt;
                if velocity > thresholds.swipe_min_velocity && distance > thresholds.swipe_min_in_px
                {
                    return None;
                }
            }

            self.is_dragging = true;
            Some(Gesture::Drag { x: self.current_x, y: self.current_y })
        }
    }

    #[cfg(test)]
    mod gesture_tests {
        use alloc::vec::Vec;

        use super::*;

        fn gestures_of(events: &[(EventType, f64, f64, f64)], th: &Thresholds) -> Vec<Gesture> {
            let mut pointer = Pointer::NONE;
            events
                .iter()
                .filter_map(|(event_type, x, y, time)| {
                    pointer.track(event_type, 1, *x, *y, *time, th)
                })
                .collect()
        }

        #[test]
        fn swipe_right_fires() {
            let th = Thresholds::MOUSE;
            let got = gestures_of(
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
        fn swipe_direction_follows_the_dominant_axis() {
            let th = Thresholds::MOUSE;
            let release = |x: f64, y: f64| {
                gestures_of(
                    &[
                        (EventType::PointerDown, 100.0, 100.0, 0.0),
                        (EventType::PointerUp, x, y, 100.0),
                    ],
                    &th,
                )
            };
            assert_eq!(release(0.0, 110.0), [Gesture::SwipeLeft]);
            assert_eq!(release(90.0, 0.0), [Gesture::SwipeUp]);
            assert_eq!(release(110.0, 200.0), [Gesture::SwipeDown]);
        }

        #[test]
        fn a_swipe_fires_without_any_move_event() {
            let th = Thresholds::MOUSE;
            let got = gestures_of(
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
            let got = gestures_of(
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
            let got = gestures_of(
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
            let got = gestures_of(
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
            let got = gestures_of(
                &[
                    (EventType::PointerDown, 100.0, 100.0, 0.0),
                    (EventType::PointerMove, 150.0, 100.0, 400.0),
                    (EventType::PointerCancel, 150.0, 100.0, 500.0),
                ],
                &th,
            );
            assert_eq!(got, [Gesture::Drag { x: 150.0, y: 100.0 }, Gesture::DragCancel]);
        }

        #[test]
        fn cancel_without_drag_is_silent() {
            let th = Thresholds::MOUSE;
            let got = gestures_of(
                &[
                    (EventType::PointerDown, 100.0, 100.0, 0.0),
                    (EventType::PointerCancel, 100.0, 100.0, 50.0),
                ],
                &th,
            );
            assert_eq!(got, []);
        }

        // --- tap ---

        #[test]
        fn quick_press_is_tap() {
            let th = Thresholds::MOUSE;
            let got = gestures_of(
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
            let got = gestures_of(
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
            let got = gestures_of(
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
            let got = gestures_of(
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
        fn long_press_is_suppressed_while_dragging() {
            let th = Thresholds::MOUSE;
            let got = gestures_of(
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
            assert!(touch.long_press_in_ms > mouse.long_press_in_ms);
            assert!(touch.long_press_slop_in_px > mouse.long_press_slop_in_px);
            assert!(touch.drag_start_in_px > mouse.drag_start_in_px);
        }

        #[test]
        fn same_input_differs_by_device() {
            let events = [
                (EventType::PointerDown, 10.0, 10.0, 0.0),
                (EventType::PointerUp, 10.0, 10.0, 300.0),
            ];
            assert_eq!(gestures_of(&events, &Thresholds::MOUSE), [Gesture::LongPress]);
            assert_eq!(gestures_of(&events, &Thresholds::TOUCH), []);
        }
    }

    #[cfg(test)]
    mod two_pointer_tests {
        use super::*;

        #[test]
        fn symmetric_pinch_out_increases_scale() {
            let th = Thresholds::MOUSE;
            let mut state =
                TwoPointerState::NONE.add(1, 140.0, 100.0, 0.0).add(2, 160.0, 100.0, 0.0);
            state = state.move_to(1, 100.0, 100.0).move_to(2, 200.0, 100.0);
            match state.fold(&th) {
                FoldedInput::Pinch { scale, .. } => assert!(scale > 1.0, "scale = {scale}"),
                other => panic!("expected Pinch, got {other:?}"),
            }
        }

        #[test]
        fn mode_latches_after_commit() {
            let th = Thresholds::MOUSE;
            let mut state =
                TwoPointerState::NONE.add(1, 100.0, 100.0, 0.0).add(2, 200.0, 100.0, 0.0);
            state = state.move_to(1, 140.0, 100.0).move_to(2, 160.0, 100.0);
            assert!(matches!(state.fold(&th), FoldedInput::Pinch { .. }));

            state = state.move_to(1, 140.0, 100.0).move_to(2, 140.0, 100.0);
            assert!(matches!(state.fold(&th), FoldedInput::Pinch { .. }));
        }

        #[test]
        fn new_pair_reclassifies_independently() {
            let th = Thresholds::MOUSE;
            let mut state =
                TwoPointerState::NONE.add(1, 100.0, 100.0, 0.0).add(2, 200.0, 100.0, 0.0);
            state = state.move_to(1, 140.0, 100.0).move_to(2, 160.0, 100.0);
            assert!(matches!(state.fold(&th), FoldedInput::Pinch { .. }));

            state = state.remove(2).0.remove(1).0;
            state = state.add(3, 0.0, 0.0, 0.0).add(4, 50.0, 0.0, 0.0);
            state = state.move_to(3, 0.0, 50.0).move_to(4, 50.0, 50.0);
            assert_eq!(state.fold(&th), FoldedInput::Pan { x: 25.0, y: 50.0 });
        }
    }

    #[cfg(test)]
    mod pointer_tests {
        use alloc::vec::Vec;

        use super::*;
        use crate::Rng;

        type PressEvent = (EventType, u32, f64, f64, f64);

        fn trace(events: &[PressEvent], th: &Thresholds) -> Vec<Option<Gesture>> {
            let mut pointer = Pointer::NONE;
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
        ) -> (Vec<PressEvent>, Vec<u32>) {
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

        fn assert_drag_grammar(pointers: u32) {
            for seed in 0..3000 {
                let mut rng = Rng::new(seed);
                let th = if rng.chance(50) { Thresholds::MOUSE } else { Thresholds::TOUCH };
                let (events, _) = random_press_events(&mut rng, pointers, 40);
                let mut dragging = false;
                for (index, got) in trace(&events, &th).into_iter().enumerate() {
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
        fn a_single_pointer_never_breaks_the_drag_grammar() {
            assert_drag_grammar(1);
        }

        #[test]
        fn several_pointers_never_break_the_drag_grammar() {
            assert_drag_grammar(3);
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
                assert_eq!(trace(&events, &th).last(), Some(&Some(Gesture::Tap)), "seed {seed}");
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
                let _ = trace(&events, &Thresholds::TOUCH);
            }
        }

        #[test]
        fn second_pointer_freezes_primary_until_pinch_commits() {
            let th = Thresholds::MOUSE;
            let got = trace(
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
        fn pinch_end_fires_on_secondary_release() {
            let th = Thresholds::MOUSE;
            let got = trace(
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
        fn primary_release_ends_the_pinch_without_a_tap() {
            let th = Thresholds::MOUSE;
            let got = trace(
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
            assert_eq!(got[5..], [None, None]);
        }

        #[test]
        fn two_pointer_pan_emits_drag_then_drag_end() {
            let th = Thresholds::MOUSE;
            let got = trace(
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
        fn a_second_two_pointer_pan_behaves_like_the_first() {
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
            let got = trace(&events, &th);
            assert_eq!(got[0..7], got[7..14]);
            assert_eq!(got[4], Some(Gesture::Drag { x: 95.0, y: 0.0 }));
        }

        #[test]
        fn a_short_two_pointer_pan_is_not_a_tap() {
            let th = Thresholds::MOUSE;
            let got = trace(
                &[
                    (EventType::PointerDown, 1, 0.0, 0.0, 0.0),
                    (EventType::PointerDown, 2, 100.0, 0.0, 0.0),
                    (EventType::PointerMove, 1, 10.0, 0.0, 50.0),
                    (EventType::PointerMove, 2, 110.0, 0.0, 60.0),
                    (EventType::PointerMove, 1, 12.0, 0.0, 80.0),
                    (EventType::PointerUp, 2, 110.0, 0.0, 100.0),
                ],
                &th,
            );
            assert_eq!(got[5], None);
        }

        const PINCH_THEN_PRIMARY_UP: [PressEvent; 5] = [
            (EventType::PointerDown, 1, 100.0, 100.0, 0.0),
            (EventType::PointerDown, 2, 200.0, 100.0, 0.0),
            (EventType::PointerMove, 1, 140.0, 100.0, 50.0),
            (EventType::PointerMove, 2, 160.0, 100.0, 60.0),
            (EventType::PointerUp, 1, 140.0, 100.0, 70.0),
        ];

        #[test]
        fn a_second_pointer_during_a_drag_cancels_the_drag() {
            let th = Thresholds::MOUSE;
            let got = trace(
                &[
                    (EventType::PointerDown, 1, 100.0, 100.0, 0.0),
                    (EventType::PointerMove, 1, 150.0, 100.0, 400.0),
                    (EventType::PointerDown, 2, 300.0, 100.0, 500.0),
                    (EventType::PointerMove, 1, 200.0, 100.0, 600.0),
                    (EventType::PointerMove, 2, 250.0, 100.0, 700.0),
                ],
                &th,
            );
            assert_eq!(got[1], Some(Gesture::Drag { x: 150.0, y: 100.0 }));
            assert_eq!(got[2], Some(Gesture::DragCancel));
            assert!(matches!(got[4], Some(Gesture::Pinch { .. })));
        }

        #[test]
        fn a_pointer_left_over_from_a_pinch_can_still_drag() {
            let th = Thresholds::MOUSE;
            let events = [
                PINCH_THEN_PRIMARY_UP.as_slice(),
                &[
                    (EventType::PointerMove, 2, 200.0, 100.0, 500.0),
                    (EventType::PointerUp, 2, 200.0, 100.0, 600.0),
                ],
            ]
            .concat();
            let got = trace(&events, &th);
            assert_eq!(got[5], Some(Gesture::Drag { x: 200.0, y: 100.0 }));
            assert_eq!(got[6], Some(Gesture::DragEnd));
        }

        #[test]
        fn a_pointer_left_over_from_a_pinch_does_not_long_press() {
            let th = Thresholds::MOUSE;
            let events = [
                PINCH_THEN_PRIMARY_UP.as_slice(),
                &[(EventType::PointerUp, 2, 160.0, 100.0, 1000.0)],
            ]
            .concat();
            assert_eq!(trace(&events, &th)[5], None);
        }

        #[test]
        fn a_pointer_left_over_from_a_pinch_does_not_swipe() {
            let th = Thresholds::MOUSE;
            let events = [
                PINCH_THEN_PRIMARY_UP.as_slice(),
                &[
                    (EventType::PointerMove, 2, 220.0, 100.0, 100.0),
                    (EventType::PointerUp, 2, 260.0, 100.0, 130.0),
                ],
            ]
            .concat();
            assert_eq!(trace(&events, &th)[5..], [None, None]);
        }

        #[test]
        fn two_pointers_held_and_released_together_make_a_long_press() {
            let th = Thresholds::MOUSE;
            let got = trace(
                &[
                    (EventType::PointerDown, 1, 100.0, 100.0, 0.0),
                    (EventType::PointerDown, 2, 200.0, 100.0, 0.0),
                    (EventType::PointerUp, 2, 200.0, 100.0, 1000.0),
                    (EventType::PointerUp, 1, 100.0, 100.0, 1010.0),
                ],
                &th,
            );
            assert_eq!(got[2..], [None, Some(Gesture::LongPress)]);
        }

        #[test]
        fn a_second_pointer_touching_and_leaving_keeps_the_hold_time() {
            let th = Thresholds::MOUSE;
            let got = trace(
                &[
                    (EventType::PointerDown, 1, 100.0, 100.0, 0.0),
                    (EventType::PointerDown, 2, 200.0, 100.0, 400.0),
                    (EventType::PointerUp, 2, 200.0, 100.0, 500.0),
                    (EventType::PointerUp, 1, 100.0, 100.0, 600.0),
                ],
                &th,
            );
            assert_eq!(got[3], Some(Gesture::LongPress));
        }

        #[test]
        fn a_promoted_pointer_is_timed_from_its_own_contact() {
            let th = Thresholds::MOUSE;
            let got = trace(
                &[
                    (EventType::PointerDown, 1, 100.0, 100.0, 0.0),
                    (EventType::PointerDown, 2, 200.0, 100.0, 900.0),
                    (EventType::PointerUp, 1, 100.0, 100.0, 1000.0),
                    (EventType::PointerUp, 2, 200.0, 100.0, 1100.0),
                ],
                &th,
            );
            assert_eq!(got[3], Some(Gesture::Tap));
        }

        #[test]
        fn third_pointer_does_not_interfere() {
            let th = Thresholds::MOUSE;
            let got = trace(
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
            let got = trace(
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
        fn duplicate_pointer_down_does_not_block_genuine_second_pointer() {
            let th = Thresholds::MOUSE;
            let got = trace(
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
