use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{
    clone::Clone,
    cmp::{Eq, PartialEq},
    default::Default,
    fmt::{self, Debug, Display, Formatter},
    marker::Copy,
    option::Option::{self, None, Some},
    primitive::{bool, f32, f64, i32, u8, u16, u32},
};

use crate::{arena::Encoder, file_store::FileStoreError};

// === send operation ===
//
// see distribution/init.js execute operation

/// case  1: el.textContent = d.string() ?? ""; break;
pub const OPERATION_SET_TEXT: u8 = 1;
/// case  2: el.value = d.string() ?? ""; break;
pub const OPERATION_SET_VALUE: u8 = 2;
/// case  3: el.setAttribute(NAMES[d.u16()], d.string() ?? ""); break;
pub const OPERATION_SET_ATTRIBUTE: u8 = 3;
/// case  4: el.removeAttribute(NAMES[d.u16()]); break;
pub const OPERATION_REMOVE_ATTRIBUTE: u8 = 4;
/// case  5: el.classList.add(NAMES[d.u16()]); break;
pub const OPERATION_ADD_CLASS: u8 = 5;
/// case  6: el.classList.remove(NAMES[d.u16()]); break;
pub const OPERATION_REMOVE_CLASS: u8 = 6;
/// case  7: el.style.width = d.u32() + "px"; break;
pub const OPERATION_SET_WIDTH: u8 = 7;
/// case  8: el.style.height = d.u32() + "px"; break;
pub const OPERATION_SET_HEIGHT: u8 = 8;
/// case  9: el.style.zIndex = d.i32(); break;
pub const OPERATION_SET_Z_INDEX: u8 = 9;
/// case 10: el.style.background = d.string(); break;
pub const OPERATION_SET_BACKGROUND: u8 = 10;
/// case 11: el.style.translate = `${d.f32()}px ${d.f32()}px`; break;
pub const OPERATION_SET_TRANSLATE: u8 = 11;
/// case 12: el.style.cursor = NAMES[d.u16()] ?? ""; break;
pub const OPERATION_SET_CURSOR: u8 = 12;
/// case 13: el.showModal(); break;
pub const OPERATION_SHOW_MODAL: u8 = 13;
/// case 14: el.close(); break;
pub const OPERATION_CLOSE_MODAL: u8 = 14;
/// case 15: el.focus(); break;
pub const OPERATION_FOCUS: u8 = 15;
/// case 16: js_fn[NAMES[d.u16()]]?.(el); break;
pub const OPERATION_JS_FN: u8 = 16;

pub const OPERATION_ERROR: u8 = 18;

///
#[derive(Debug)]
pub enum CommandError {
    Decode,
    CommandOverflow,
    Panic { location: String, message: String },
    FileStore(FileStoreError),
}

impl Display for CommandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<FileStoreError> for CommandError {
    fn from(error: FileStoreError) -> Self {
        CommandError::FileStore(error)
    }
}

impl CommandError {
    /// ```
    /// # use app::js_client::CommandError;
    /// assert!(!CommandError::Decode.is_serious());
    /// assert!(CommandError::Panic { location: "".into(), message: "".into() }.is_serious());
    /// ```
    pub fn is_serious(&self) -> bool {
        match self {
            CommandError::Decode | CommandError::CommandOverflow => false,
            CommandError::Panic { .. } | CommandError::FileStore(_) => true,
        }
    }

    pub(crate) fn wire_code(&self) -> u8 {
        match self {
            CommandError::Decode => 1,
            CommandError::CommandOverflow => 2,
            CommandError::Panic { .. } => 3,
            CommandError::FileStore(_) => 4,
        }
    }
}

/// Command from Wasm to JavaScript thread
pub enum Command {
    SetText {
        id:    dom::Id,
        value: String,
    },
    SetValue {
        id:    dom::Id,
        value: String,
    },
    SetAttribute {
        id:        dom::Id,
        attribute: Attribute,
        value:     String,
    },
    RemoveAttribute {
        id:        dom::Id,
        attribute: Attribute,
    },
    AddClass {
        id:    dom::Id,
        value: ClassName,
    },
    RemoveClass {
        id:    dom::Id,
        value: ClassName,
    },
    SetWidth {
        id: dom::Id,
        px: u32,
    },
    SetHeight {
        id: dom::Id,
        px: u32,
    },
    SetZIndex {
        id: dom::Id,
        z:  i32,
    },
    SetBackground {
        id:    dom::Id,
        value: String,
    },
    SetTranslate {
        id: dom::Id,
        x:  f32,
        y:  f32,
    },
    SetCursor {
        id:    dom::Id,
        value: CursorValue,
    },
    ShowModal {
        id: dom::Id,
    },
    CloseModal {
        id: dom::Id,
    },
    Focus {
        id: dom::Id,
    },
    JsFn {
        id:   dom::Id,
        name: FnName,
    },
    Error {
        error: CommandError,
    },
}

///
///
/// ```
/// # use app::js_client::{encode_command, Command, dom, OPERATION_FOCUS};
/// let mut commands = Vec::new();
/// encode_command(&mut commands, &Command::Focus {
///     id: dom::Id::new(&[(dom::Tag::Body, None)]),
/// });
/// assert_eq!(commands[0], OPERATION_FOCUS);
/// ```
pub fn encode_command(commands: &mut Vec<u8>, command: &Command) {
    let mut encoder = Encoder::new(commands);
    match *command {
        Command::SetText { ref id, ref value } => {
            encoder.u8(OPERATION_SET_TEXT);
            encoder.id(id);
            encoder.str(value);
        }
        Command::SetValue { ref id, ref value } => {
            encoder.u8(OPERATION_SET_VALUE);
            encoder.id(id);
            encoder.str(value);
        }
        Command::SetAttribute { ref id, attribute, ref value } => {
            encoder.u8(OPERATION_SET_ATTRIBUTE);
            encoder.id(id);
            encoder.u16(attribute.encode_u16());
            encoder.str(value);
        }
        Command::RemoveAttribute { ref id, attribute } => {
            encoder.u8(OPERATION_REMOVE_ATTRIBUTE);
            encoder.id(id);
            encoder.u16(attribute.encode_u16());
        }
        Command::AddClass { ref id, value } => {
            encoder.u8(OPERATION_ADD_CLASS);
            encoder.id(id);
            encoder.u16(value.encode_u16());
        }
        Command::RemoveClass { ref id, value } => {
            encoder.u8(OPERATION_REMOVE_CLASS);
            encoder.id(id);
            encoder.u16(value.encode_u16());
        }
        Command::SetWidth { ref id, px } => {
            encoder.u8(OPERATION_SET_WIDTH);
            encoder.id(id);
            encoder.u32(px);
        }
        Command::SetHeight { ref id, px } => {
            encoder.u8(OPERATION_SET_HEIGHT);
            encoder.id(id);
            encoder.u32(px);
        }
        Command::SetZIndex { ref id, z } => {
            encoder.u8(OPERATION_SET_Z_INDEX);
            encoder.id(id);
            encoder.i32(z);
        }
        Command::SetBackground { ref id, ref value } => {
            encoder.u8(OPERATION_SET_BACKGROUND);
            encoder.id(id);
            encoder.str(value);
        }
        Command::SetTranslate { ref id, x, y } => {
            encoder.u8(OPERATION_SET_TRANSLATE);
            encoder.id(id);
            encoder.f32(x);
            encoder.f32(y);
        }
        Command::SetCursor { ref id, value } => {
            encoder.u8(OPERATION_SET_CURSOR);
            encoder.id(id);
            encoder.u16(value.encode_u16());
        }
        Command::ShowModal { ref id } => {
            encoder.u8(OPERATION_SHOW_MODAL);
            encoder.id(id);
        }
        Command::CloseModal { ref id } => {
            encoder.u8(OPERATION_CLOSE_MODAL);
            encoder.id(id);
        }
        Command::Focus { ref id } => {
            encoder.u8(OPERATION_FOCUS);
            encoder.id(id);
        }
        Command::JsFn { ref id, name } => {
            encoder.u8(OPERATION_JS_FN);
            encoder.id(id);
            encoder.u16(name.encode_u16());
        }
        Command::Error { ref error } => encode_error(&mut encoder, error, &error.to_string()),
    }
}

pub(crate) fn encode_error(encoder: &mut Encoder, error: &CommandError, message: &str) {
    encoder.u8(OPERATION_ERROR);
    encoder.u8(error.is_serious() as u8);
    encoder.u8(error.wire_code());
    encoder.str(message);
}

// === receive (canvas event) ===

///
/// crate::event::decode_event
pub struct CanvasEvent {
    pub event_type: EventType,
    pub id:         dom::Id,
    pub key:        KeyName,
    pub value:      String,
    pub x:          f64,
    pub y:          f64,
    pub time:       f64,
    pub pointer_id: u32,
}


#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Attribute {
    Disabled,
    Hidden,
}

impl Attribute {
    fn encode_u16(self) -> u16 {
        self as u16
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClassName {
    Hide,
    Show,
    Hidden,
}

impl ClassName {
    fn encode_u16(self) -> u16 {
        self as u16
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CursorValue {
    Default,
    Grab,
}

impl CursorValue {
    fn encode_u16(self) -> u16 {
        self as u16
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FnName {
    HideToast,
    ShowToast,
}

impl FnName {
    fn encode_u16(self) -> u16 {
        self as u16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Device {
    Touch,
    Mouse,
}

pub fn detect_device(pointer_coarse: bool) -> Device {
    if pointer_coarse { Device::Touch } else { Device::Mouse }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EventType {
    Change,
    Click,
    ContextMenu,
    Drop,
    FocusIn,
    FocusOut,
    Input,
    KeyDown,
    PointerCancel,
    PointerDown,
    PointerMove,
    PointerUp,
    Resize,
    Scroll,
    Submit,
    Other,
}

impl EventType {
    /// init.js EVENT_TYPES
    ///
    /// ```
    /// # use app::js_client::EventType;
    /// assert_eq!(EventType::decode_u8(10), EventType::PointerDown);
    /// assert_eq!(EventType::decode_u8(200), EventType::Other);
    /// ```
    pub fn decode_u8(value: u8) -> Self {
        match value {
            1 => Self::Change,
            2 => Self::Click,
            3 => Self::ContextMenu,
            4 => Self::Drop,
            5 => Self::FocusIn,
            6 => Self::FocusOut,
            7 => Self::Input,
            8 => Self::KeyDown,
            9 => Self::PointerCancel,
            10 => Self::PointerDown,
            11 => Self::PointerMove,
            12 => Self::PointerUp,
            13 => Self::Resize,
            14 => Self::Scroll,
            15 => Self::Submit,
            _ => Self::Other,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyName {
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    Backspace,
    Enter,
    Escape,
    Tab,
    Other,
}

impl KeyName {
    /// ```
    /// # use app::js_client::KeyName;
    /// assert_eq!(KeyName::decode_u8(6), KeyName::Enter);
    /// assert_eq!(KeyName::decode_u8(200), KeyName::Other);
    /// ```
    pub fn decode_u8(value: u8) -> Self {
        match value {
            1 => Self::ArrowDown,
            2 => Self::ArrowLeft,
            3 => Self::ArrowRight,
            4 => Self::ArrowUp,
            5 => Self::Backspace,
            6 => Self::Enter,
            7 => Self::Escape,
            8 => Self::Tab,
            _ => Self::Other,
        }
    }
}

// === gesture: tap, long press, swipe (up,down,left,right), drag (See ./docs/Gesture.md) ===

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

#[derive(Debug, Default, Clone, Copy)]
pub struct PointerState {
    is_down:          bool,
    start_x:          f64,
    start_y:          f64,
    current_x:        f64,
    current_y:        f64,
    start_time:       f64,
    last_move_x:      f64,
    last_move_y:      f64,
    last_move_time:   f64,
    drag_offset:      (f64, f64),
    drag_px:          (f64, f64),
    is_dragging:      bool,
    long_press_fired: bool,
    cancelled:        bool,
}

impl PointerState {
    #[must_use]
    pub fn update(self, event_type: &EventType, x: f64, y: f64, time: f64) -> Self {
        match event_type {
            EventType::PointerDown => Self {
                is_down:          true,
                start_x:          x,
                start_y:          y,
                current_x:        x,
                current_y:        y,
                start_time:       time,
                last_move_x:      x,
                last_move_y:      y,
                last_move_time:   time,
                drag_offset:      (0.0, 0.0),
                drag_px:          (0.0, 0.0),
                is_dragging:      false,
                long_press_fired: false,
                cancelled:        false,
            },
            EventType::PointerMove => Self {
                current_x: x,
                current_y: y,
                last_move_x: x,
                last_move_y: y,
                last_move_time: time,
                ..self
            },
            EventType::PointerUp => {
                Self { is_down: false, current_x: x, current_y: y, cancelled: false, ..self }
            }
            EventType::PointerCancel => {
                Self { is_down: false, current_x: x, current_y: y, cancelled: true, ..self }
            }
            _ => self,
        }
    }

    fn distance(&self) -> f64 {
        let dx = self.current_x - self.start_x;
        let dy = self.current_y - self.start_y;
        libm::sqrt(dx * dx + dy * dy)
    }

    #[must_use]
    pub const fn current(&self) -> (f64, f64) {
        (self.current_x, self.current_y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gesture {
    Tap, // includes click
    LongPress,
    SwipeUp,
    SwipeDown,
    SwipeLeft,
    SwipeRight,
    Drag {
        x: f64,
        y: f64,
    }, // in dragging
    DragEnd,
    DragCancel,
    ///
    Pinch {
        scale:    f64,
        center_x: f64,
        center_y: f64,
    },
    PinchEnd,
}

///
///
///
///
#[must_use]
pub fn detect_gesture(
    state: &mut PointerState,
    prev_state: &PointerState,
    event_type: &EventType,
    current_time: f64,
    thresholds: &Thresholds,
) -> Option<Gesture> {
    match event_type {
        EventType::PointerUp | EventType::PointerCancel => {
            detect_on_release(state, prev_state, current_time, thresholds)
        }
        EventType::PointerMove => detect_on_move(state, current_time, thresholds),
        _ => None,
    }
}

fn detect_on_release(
    state: &mut PointerState,
    prev_state: &PointerState,
    current_time: f64,
    thresholds: &Thresholds,
) -> Option<Gesture> {
    if prev_state.is_dragging {
        state.is_dragging = false;
        return Some(if state.cancelled { Gesture::DragCancel } else { Gesture::DragEnd });
    }

    if state.cancelled {
        return None;
    }

    let dt = current_time - state.start_time;
    if dt <= 0.0 {
        return None;
    }
    let distance = state.distance();

    //
    let move_dt = current_time - state.last_move_time;
    let velocity = if move_dt > 0.0 {
        let mdx = state.current_x - state.last_move_x;
        let mdy = state.current_y - state.last_move_y;
        libm::sqrt(mdx * mdx + mdy * mdy) / move_dt
    } else {
        0.0
    };
    if velocity > thresholds.swipe_min_velocity
        && distance > thresholds.swipe_min_px
        && dt < thresholds.swipe_max_ms
    {
        let dx = state.current_x - state.start_x;
        let dy = state.current_y - state.start_y;
        return Some(if libm::fabs(dx) > libm::fabs(dy) {
            if dx > 0.0 { Gesture::SwipeRight } else { Gesture::SwipeLeft }
        } else if dy > 0.0 {
            Gesture::SwipeDown
        } else {
            Gesture::SwipeUp
        });
    }

    if state.long_press_fired {
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

fn detect_on_move(
    state: &mut PointerState,
    current_time: f64,
    thresholds: &Thresholds,
) -> Option<Gesture> {
    if !state.is_down {
        return None;
    }

    let distance = state.distance();

    if !state.long_press_fired
        && !state.is_dragging
        && distance < thresholds.long_press_slop_px
        && current_time - state.start_time > thresholds.long_press_ms
    {
        state.long_press_fired = true;
        return Some(Gesture::LongPress);
    }

    if distance <= thresholds.drag_start_px {
        return None;
    }

    if state.is_dragging {
        return Some(Gesture::Drag { x: state.current_x, y: state.current_y });
    }

    let dt = current_time - state.start_time;
    if dt > 0.0 && dt < thresholds.swipe_max_ms {
        let velocity = distance / dt;
        if velocity > thresholds.swipe_min_velocity && distance > thresholds.swipe_min_px {
            return None;
        }
    }

    state.is_dragging = true;
    Some(Gesture::Drag { x: state.current_x, y: state.current_y })
}

#[cfg(test)]
mod gesture_tests {
    use alloc::vec::Vec;

    use super::*;

    fn run(events: &[(EventType, f64, f64, f64)], th: &Thresholds) -> Vec<Gesture> {
        let mut state = PointerState::default();
        let mut out = Vec::new();
        for (event_type, x, y, time) in events {
            let prev = state;
            state = state.update(event_type, *x, *y, *time);
            if let Some(g) = detect_gesture(&mut state, &prev, event_type, *time, th) {
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
            &[(EventType::PointerDown, 10.0, 10.0, 0.0), (EventType::PointerUp, 10.0, 10.0, 400.0)],
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
        assert_eq!(got, [Gesture::Drag { x: 40.0, y: 10.0 }, Gesture::Drag { x: 40.0, y: 10.0 }]);
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
        let events =
            [(EventType::PointerDown, 10.0, 10.0, 0.0), (EventType::PointerUp, 10.0, 10.0, 300.0)];
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

///
const TWO_FINGER_COMMIT_PX: f64 = 8.0;

///
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
    fn touch_down(self, id: u32, x: f64, y: f64) -> Self {
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
    fn touch_move(self, id: u32, x: f64, y: f64) -> Self {
        if self.primary.is_some_and(|p| p.id == id) {
            Self {
                primary: self.primary.map(|p| TouchPoint { current_x: x, current_y: y, ..p }),
                ..self
            }
        } else if self.secondary.is_some_and(|s| s.id == id) {
            Self {
                secondary: self.secondary.map(|s| TouchPoint { current_x: x, current_y: y, ..s }),
                ..self
            }
        } else {
            self
        }
    }

    #[must_use]
    fn touch_up(self, id: u32) -> (Self, TwoFingerMode) {
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
    fn primary_current(&self) -> Option<(f64, f64)> {
        self.primary.map(|p| (p.current_x, p.current_y))
    }

    ///
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
            self.mode =
                if dot < PINCH_DOT_THRESHOLD { TwoFingerMode::Pinch } else { TwoFingerMode::Pan };
        }

        match self.mode {
            TwoFingerMode::Undetermined => FoldedInput::None,
            TwoFingerMode::Pan => FoldedInput::AsSinglePoint {
                x: (p.current_x + s.current_x) / 2.0,
                y: (p.current_y + s.current_y) / 2.0,
            },
            TwoFingerMode::Pinch => {
                let start_distance = two_point_distance(p.start_x, p.start_y, s.start_x, s.start_y);
                if start_distance <= 0.0 {
                    return FoldedInput::None;
                }
                let current_distance =
                    two_point_distance(p.current_x, p.current_y, s.current_x, s.current_y);
                FoldedInput::Pinch {
                    scale:    current_distance / start_distance,
                    center_x: (p.current_x + s.current_x) / 2.0,
                    center_y: (p.current_y + s.current_y) / 2.0,
                }
            }
        }
    }
}

fn two_point_distance(x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
    let dx = x1 - x0;
    let dy = y1 - y0;
    libm::sqrt(dx * dx + dy * dy)
}


///
///
///
///
///
///
///
///
#[derive(Debug, Default)]
pub struct TouchTracker {
    primary_state: PointerState,
    two_fingers:   TwoFingerState,
    pan_state:     Option<PointerState>,
}

impl TouchTracker {
    #[must_use]
    pub fn handle(
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
                self.on_down(pointer_id, x, y, time);
                None
            }
            EventType::PointerMove => self.on_move(pointer_id, x, y, time, thresholds),
            EventType::PointerUp | EventType::PointerCancel => {
                self.on_up(event_type, pointer_id, x, y, time, thresholds)
            }
            _ => None,
        }
    }

    #[must_use]
    pub const fn active_state(&self) -> &PointerState {
        match &self.pan_state {
            Some(state) => state,
            None => &self.primary_state,
        }
    }

    fn on_down(&mut self, id: u32, x: f64, y: f64, time: f64) {
        if self.two_fingers.primary_id() == Some(id) || self.two_fingers.secondary_id() == Some(id)
        {
            self.two_fingers = self.two_fingers.touch_move(id, x, y);
            return;
        }
        if self.two_fingers.primary_id().is_none() {
            self.primary_state = self.primary_state.update(&EventType::PointerDown, x, y, time);
        }
        self.two_fingers = self.two_fingers.touch_down(id, x, y);
    }

    fn on_move(
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
        self.two_fingers = self.two_fingers.touch_move(id, x, y);

        if self.two_fingers.secondary_id().is_some() {
            return self.fold_and_emit(time, thresholds);
        }

        let prev = self.primary_state;
        self.primary_state = self.primary_state.update(&EventType::PointerMove, x, y, time);
        detect_gesture(&mut self.primary_state, &prev, &EventType::PointerMove, time, thresholds)
    }

    fn on_up(
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
            let (next, ended_mode) = self.two_fingers.touch_up(id);
            self.two_fingers = next;
            let gesture = self.end_two_finger_session(event_type, ended_mode, time, thresholds);
            self.resync_primary(time);
            gesture
        } else if is_primary {
            let prev = self.primary_state;
            self.primary_state = self.primary_state.update(event_type, x, y, time);
            let gesture =
                detect_gesture(&mut self.primary_state, &prev, event_type, time, thresholds);
            self.two_fingers = self.two_fingers.touch_up(id).0;
            gesture
        } else {
            None
        }
    }

    fn end_two_finger_session(
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
                let state = self.pan_state.take()?;
                let (cx, cy) = state.current();
                let prev = state;
                let mut next = state.update(event_type, cx, cy, time);
                detect_gesture(&mut next, &prev, event_type, time, thresholds)
            }
        }
    }

    fn resync_primary(&mut self, time: f64) {
        self.primary_state = match self.two_fingers.primary_current() {
            Some((x, y)) => PointerState::default().update(&EventType::PointerDown, x, y, time),
            None => PointerState::default(),
        };
    }

    fn fold_and_emit(&mut self, time: f64, thresholds: &Thresholds) -> Option<Gesture> {
        match self.two_fingers.fold() {
            FoldedInput::None => None,
            FoldedInput::Pinch { scale, center_x, center_y } => {
                Some(Gesture::Pinch { scale, center_x, center_y })
            }
            FoldedInput::AsSinglePoint { x, y } => match self.pan_state {
                None => {
                    self.pan_state =
                        Some(PointerState::default().update(&EventType::PointerDown, x, y, time));
                    None
                }
                Some(state) => {
                    let prev = state;
                    let mut next = state.update(&EventType::PointerMove, x, y, time);
                    let gesture =
                        detect_gesture(&mut next, &prev, &EventType::PointerMove, time, thresholds);
                    self.pan_state = Some(next);
                    gesture
                }
            },
        }
    }
}

#[cfg(test)]
mod two_finger_tests {
    use super::*;

    #[test]
    fn waits_for_both_fingers_before_classifying() {
        let mut state =
            TwoFingerState::default().touch_down(1, 100.0, 100.0).touch_down(2, 200.0, 100.0);

        state = state.touch_move(1, 110.0, 100.0);
        assert_eq!(state.fold(), FoldedInput::None);
        state = state.touch_move(1, 130.0, 100.0);
        assert_eq!(state.fold(), FoldedInput::None);

        state = state.touch_move(1, 150.0, 100.0).touch_move(2, 150.0, 100.0);
        assert_eq!(
            state.fold(),
            FoldedInput::Pinch { scale: 0.0, center_x: 150.0, center_y: 100.0 }
        );
    }

    #[test]
    fn symmetric_pinch_in_reduces_scale() {
        let mut state =
            TwoFingerState::default().touch_down(1, 100.0, 100.0).touch_down(2, 200.0, 100.0);
        state = state.touch_move(1, 140.0, 100.0).touch_move(2, 160.0, 100.0);
        match state.fold() {
            FoldedInput::Pinch { scale, .. } => assert!(scale < 1.0, "scale = {scale}"),
            other => panic!("expected Pinch, got {other:?}"),
        }
    }

    #[test]
    fn symmetric_pinch_out_increases_scale() {
        let mut state =
            TwoFingerState::default().touch_down(1, 140.0, 100.0).touch_down(2, 160.0, 100.0);
        state = state.touch_move(1, 100.0, 100.0).touch_move(2, 200.0, 100.0);
        match state.fold() {
            FoldedInput::Pinch { scale, .. } => assert!(scale > 1.0, "scale = {scale}"),
            other => panic!("expected Pinch, got {other:?}"),
        }
    }

    #[test]
    fn parallel_motion_is_pan_not_pinch() {
        let mut state =
            TwoFingerState::default().touch_down(1, 100.0, 100.0).touch_down(2, 200.0, 100.0);
        state = state.touch_move(1, 120.0, 100.0).touch_move(2, 220.0, 100.0);
        assert_eq!(state.fold(), FoldedInput::AsSinglePoint { x: 170.0, y: 100.0 });
    }

    #[test]
    fn mode_latches_after_commit() {
        let mut state =
            TwoFingerState::default().touch_down(1, 100.0, 100.0).touch_down(2, 200.0, 100.0);
        state = state.touch_move(1, 140.0, 100.0).touch_move(2, 160.0, 100.0);
        assert!(matches!(state.fold(), FoldedInput::Pinch { .. }));

        state = state.touch_move(1, 140.0, 100.0).touch_move(2, 140.0, 100.0);
        assert!(matches!(state.fold(), FoldedInput::Pinch { .. }));
    }

    #[test]
    fn third_finger_is_ignored() {
        let state = TwoFingerState::default()
            .touch_down(1, 100.0, 100.0)
            .touch_down(2, 200.0, 100.0)
            .touch_down(3, 300.0, 100.0);
        assert_eq!(state.primary_id(), Some(1));
        assert_eq!(state.secondary_id(), Some(2));
    }

    #[test]
    fn primary_release_promotes_secondary() {
        let mut state =
            TwoFingerState::default().touch_down(1, 100.0, 100.0).touch_down(2, 200.0, 100.0);
        state = state.touch_move(1, 140.0, 100.0).touch_move(2, 160.0, 100.0);
        assert!(matches!(state.fold(), FoldedInput::Pinch { .. }));

        let (next, ended_mode) = state.touch_up(1);
        state = next;
        assert_eq!(ended_mode, TwoFingerMode::Pinch);
        assert_eq!(state.primary_id(), Some(2));
        assert!(state.secondary_id().is_none());
        assert_eq!(state.fold(), FoldedInput::None);
    }

    #[test]
    fn new_session_reclassifies_independently() {
        let mut state =
            TwoFingerState::default().touch_down(1, 100.0, 100.0).touch_down(2, 200.0, 100.0);
        state = state.touch_move(1, 140.0, 100.0).touch_move(2, 160.0, 100.0);
        assert!(matches!(state.fold(), FoldedInput::Pinch { .. }));

        state = state.touch_up(2).0.touch_up(1).0;
        state = state.touch_down(3, 0.0, 0.0).touch_down(4, 50.0, 0.0);
        state = state.touch_move(3, 0.0, 50.0).touch_move(4, 50.0, 50.0);
        assert_eq!(state.fold(), FoldedInput::AsSinglePoint { x: 25.0, y: 50.0 });
    }
}

#[cfg(test)]
mod touch_tracker_tests {
    use alloc::vec::Vec;

    use super::*;

    fn run(events: &[(EventType, u32, f64, f64, f64)], th: &Thresholds) -> Vec<Option<Gesture>> {
        let mut tracker = TouchTracker::default();
        events
            .iter()
            .map(|(event_type, id, x, y, time)| tracker.handle(event_type, *id, *x, *y, *time, th))
            .collect()
    }

    #[test]
    fn single_finger_behaves_like_before() {
        let th = Thresholds::MOUSE;
        let got = run(
            &[
                (EventType::PointerDown, 1, 100.0, 100.0, 0.0),
                (EventType::PointerMove, 1, 200.0, 100.0, 50.0),
                (EventType::PointerUp, 1, 260.0, 100.0, 100.0),
            ],
            &th,
        );
        assert_eq!(got, [None, None, Some(Gesture::SwipeRight)]);
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
        assert_eq!(got[3], Some(Gesture::Pinch { scale: 0.2, center_x: 150.0, center_y: 100.0 }));
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
        assert_eq!(got[6], Some(Gesture::Pinch { scale: 0.2, center_x: 150.0, center_y: 100.0 }));
    }

    ///
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
        assert_eq!(got[4], Some(Gesture::Pinch { scale: 0.2, center_x: 150.0, center_y: 100.0 }));
    }
}

// === dom (rust item <=> element id) ===

pub mod dom {
    use alloc::vec::Vec;
    use core::{
        clone::Clone,
        cmp::PartialEq,
        fmt::Debug,
        iter::Iterator,
        option::Option,
        primitive::{u8, u32},
    };

    #[derive(Debug, Clone, PartialEq)]
    pub enum Tag {
        Article,
        Body,
        Button,
        Dd,
        Dl,
        Drawer, // <dialog id="*drawer*">
        Dt,
        Fieldset,
        Footer,
        Form,
        H1,
        H2,
        H3,
        Header,
        Input,
        Li,
        Main,
        Modal, // <dialog id="*modal*">
        Ol,
        Output,
        P,
        Section,
        Select,
        Span,
        Table,
        Tbody,
        Td,
        Textarea,
        Th,
        Thead,
        Tr,
        Ul,
        Other,
    }

    impl Tag {
        /// init.js::TAGS
        ///
        /// ```
        /// # use app::js_client::dom::Tag;
        /// assert_eq!(Tag::Article.encode_u8(), 1);
        /// assert_eq!(Tag::Other.encode_u8(), 0);
        /// ```
        pub fn encode_u8(&self) -> u8 {
            match self {
                Self::Article => 1,
                Self::Body => 2,
                Self::Button => 3,
                Self::Dd => 4,
                Self::Dl => 5,
                Self::Drawer => 6,
                Self::Dt => 7,
                Self::Fieldset => 8,
                Self::Footer => 9,
                Self::Form => 10,
                Self::H1 => 11,
                Self::H2 => 12,
                Self::H3 => 13,
                Self::Header => 14,
                Self::Input => 15,
                Self::Li => 16,
                Self::Main => 17,
                Self::Modal => 18,
                Self::Ol => 19,
                Self::Output => 20,
                Self::P => 21,
                Self::Section => 22,
                Self::Select => 23,
                Self::Span => 24,
                Self::Table => 25,
                Self::Tbody => 26,
                Self::Td => 27,
                Self::Textarea => 28,
                Self::Th => 29,
                Self::Thead => 30,
                Self::Tr => 31,
                Self::Ul => 32,
                Self::Other => 0,
            }
        }

        /// ```
        /// # use app::js_client::dom::Tag;
        /// assert_eq!(Tag::decode_u8(1), Tag::Article);
        /// assert_eq!(Tag::decode_u8(200), Tag::Other);
        /// ```
        pub fn decode_u8(value: u8) -> Self {
            match value {
                1 => Self::Article,
                2 => Self::Body,
                3 => Self::Button,
                4 => Self::Dd,
                5 => Self::Dl,
                6 => Self::Drawer,
                7 => Self::Dt,
                8 => Self::Fieldset,
                9 => Self::Footer,
                10 => Self::Form,
                11 => Self::H1,
                12 => Self::H2,
                13 => Self::H3,
                14 => Self::Header,
                15 => Self::Input,
                16 => Self::Li,
                17 => Self::Main,
                18 => Self::Modal,
                19 => Self::Ol,
                20 => Self::Output,
                21 => Self::P,
                22 => Self::Section,
                23 => Self::Select,
                24 => Self::Span,
                25 => Self::Table,
                26 => Self::Tbody,
                27 => Self::Td,
                28 => Self::Textarea,
                29 => Self::Th,
                30 => Self::Thead,
                31 => Self::Tr,
                32 => Self::Ul,
                _ => Self::Other,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct Segment {
        pub tag: Tag,
        pub n:   Option<u32>,
    }

    /// element id。
    #[derive(Debug, Clone, PartialEq)]
    pub struct Id(pub Vec<Segment>);

    impl Id {
        pub fn new(segs: &[(Tag, Option<u32>)]) -> Self {
            Self(segs.iter().map(|(tag, n)| Segment { tag: tag.clone(), n: *n }).collect())
        }
    }
}
