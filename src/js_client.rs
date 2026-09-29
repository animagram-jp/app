use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{
    clone::Clone,
    cmp::{Eq, PartialEq},
    convert::TryInto,
    default::Default,
    fmt::Debug,
    marker::Copy,
    option::Option::{self, None, Some},
    primitive::{bool, f32, f64, i32, str, u8, u16, u32, usize},
};

use crate::{Error, field::Field};

// === send operation ===
//
// see distribution/init.js execute operation

pub const OPERATION_SET_TEXT: u8 = 1;
pub const OPERATION_SET_VALUE: u8 = 2;
pub const OPERATION_SET_ATTRIBUTE: u8 = 3;
pub const OPERATION_REMOVE_ATTRIBUTE: u8 = 4;
pub const OPERATION_ADD_CLASS: u8 = 5;
pub const OPERATION_REMOVE_CLASS: u8 = 6;
pub const OPERATION_SET_STYLE: u8 = 7;
pub const OPERATION_REMOVE_STYLE: u8 = 8;
pub const OPERATION_SHOW_MODAL: u8 = 9;
pub const OPERATION_CLOSE_MODAL: u8 = 10;
pub const OPERATION_FOCUS: u8 = 11;
pub const OPERATION_JS_FN: u8 = 12;

pub const OPERATION_ERROR: u8 = 13;
pub const OPERATION_FETCH: u8 = 14;

pub trait WireError {
    fn identifiers(&self, path: &mut Vec<u16>);
    fn detail(&self) -> String;
    fn is_serious(&self) -> bool;
}

macro_rules! wire_error {
    ($name:ident { $($variant:ident($inner:ty) = $identifier:expr),+ $(,)? }) => {
        #[derive(Debug)]
        pub enum $name {
            $($variant($inner)),+
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::write!(f, "{:?}", self)
            }
        }

        impl $crate::js_client::WireError for $name {
            fn identifiers(&self, path: &mut ::alloc::vec::Vec<u16>) {
                match self {
                    $($name::$variant(error) => {
                        path.push($identifier);
                        $crate::js_client::WireError::identifiers(error, path);
                    })+
                }
            }

            fn detail(&self) -> ::alloc::string::String {
                match self {
                    $($name::$variant(error) => $crate::js_client::WireError::detail(error)),+
                }
            }

            fn is_serious(&self) -> bool {
                match self {
                    $($name::$variant(error) => $crate::js_client::WireError::is_serious(error)),+
                }
            }
        }
    };
}

pub(crate) use wire_error;

/// Command from Wasm to JavaScript thread
pub enum Command {
    SetText { id: dom::Id, value: String },
    SetValue { id: dom::Id, value: String },
    SetAttribute { id: dom::Id, attribute: Attribute, value: String },
    RemoveAttribute { id: dom::Id, attribute: Attribute },
    AddClass { id: dom::Id, value: ClassName },
    RemoveClass { id: dom::Id, value: ClassName },
    SetStyle { id: dom::Id, property: StyleProperty, value: StyleValue },
    RemoveStyle { id: dom::Id, property: StyleProperty },
    ShowModal { id: dom::Id },
    CloseModal { id: dom::Id },
    Focus { id: dom::Id },
    JsFn { id: dom::Id, name: FnName },
    Fetch { request: u32, method: Method, path: String, body: Vec<u8> },
    Error { error: Error },
}

///
///
/// ```
/// # use app::js_client::{encode_command, Command, dom, OPERATION_FOCUS};
/// let mut frame = Vec::new();
/// encode_command(&mut frame, &Command::Focus {
///     id: dom::Id::new(&[(dom::Tag::Body, None)]),
/// });
/// assert_eq!(frame[0], OPERATION_FOCUS);
/// ```
pub fn encode_command(frame: &mut Vec<u8>, command: &Command) {
    match *command {
        Command::SetText { ref id, ref value } => {
            frame.push(OPERATION_SET_TEXT);
            id.encode(frame);
            put_str(frame, value);
        }
        Command::SetValue { ref id, ref value } => {
            frame.push(OPERATION_SET_VALUE);
            id.encode(frame);
            put_str(frame, value);
        }
        Command::SetAttribute { ref id, attribute, ref value } => {
            frame.push(OPERATION_SET_ATTRIBUTE);
            id.encode(frame);
            put_u16(frame, attribute.encode_u16());
            put_str(frame, value);
        }
        Command::RemoveAttribute { ref id, attribute } => {
            frame.push(OPERATION_REMOVE_ATTRIBUTE);
            id.encode(frame);
            put_u16(frame, attribute.encode_u16());
        }
        Command::AddClass { ref id, value } => {
            frame.push(OPERATION_ADD_CLASS);
            id.encode(frame);
            put_u16(frame, value.encode_u16());
        }
        Command::RemoveClass { ref id, value } => {
            frame.push(OPERATION_REMOVE_CLASS);
            id.encode(frame);
            put_u16(frame, value.encode_u16());
        }
        Command::SetStyle { ref id, property, ref value } => {
            frame.push(OPERATION_SET_STYLE);
            id.encode(frame);
            put_u16(frame, property.encode_u16());
            value.encode(frame);
        }
        Command::RemoveStyle { ref id, property } => {
            frame.push(OPERATION_REMOVE_STYLE);
            id.encode(frame);
            put_u16(frame, property.encode_u16());
        }
        Command::ShowModal { ref id } => {
            frame.push(OPERATION_SHOW_MODAL);
            id.encode(frame);
        }
        Command::CloseModal { ref id } => {
            frame.push(OPERATION_CLOSE_MODAL);
            id.encode(frame);
        }
        Command::Focus { ref id } => {
            frame.push(OPERATION_FOCUS);
            id.encode(frame);
        }
        Command::JsFn { ref id, name } => {
            frame.push(OPERATION_JS_FN);
            id.encode(frame);
            put_u16(frame, name.encode_u16());
        }
        Command::Fetch { request, method, ref path, ref body } => {
            frame.push(OPERATION_FETCH);
            put_u32(frame, request);
            frame.push(method.encode_u8());
            put_str(frame, path);
            put_bytes(frame, body);
        }
        Command::Error { ref error } => encode_error(frame, error, &error.detail()),
    }
}

pub(crate) fn encode_error(frame: &mut Vec<u8>, error: &Error, detail: &str) {
    frame.push(OPERATION_ERROR);
    frame.push(error.is_serious() as u8);
    let mut path = Vec::new();
    error.identifiers(&mut path);
    frame.push(path.len() as u8);
    for identifier in path {
        put_u16(frame, identifier);
    }
    put_str(frame, detail);
}

pub(crate) fn put_u16(frame: &mut Vec<u8>, value: u16) {
    frame.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_u32(frame: &mut Vec<u8>, value: u32) {
    frame.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_i32(frame: &mut Vec<u8>, value: i32) {
    frame.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_f32(frame: &mut Vec<u8>, value: f32) {
    frame.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_bytes(frame: &mut Vec<u8>, value: &[u8]) {
    put_u32(frame, value.len() as u32);
    frame.extend_from_slice(value);
}

pub(crate) fn put_str(frame: &mut Vec<u8>, value: &str) {
    put_bytes(frame, value.as_bytes());
}

fn take<'a>(input: &mut &'a [u8], count: usize) -> Option<&'a [u8]> {
    let (head, tail) = input.split_at_checked(count)?;
    *input = tail;
    Some(head)
}

pub(crate) fn get_u8(input: &mut &[u8]) -> Option<u8> {
    Some(take(input, 1)?[0])
}

pub(crate) fn get_u32(input: &mut &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(take(input, 4)?.try_into().ok()?))
}

pub(crate) fn get_f32(input: &mut &[u8]) -> Option<f32> {
    Some(f32::from_le_bytes(take(input, 4)?.try_into().ok()?))
}

pub(crate) fn get_f64(input: &mut &[u8]) -> Option<f64> {
    Some(f64::from_le_bytes(take(input, 8)?.try_into().ok()?))
}

pub(crate) fn get_u16(input: &mut &[u8]) -> Option<u16> {
    Some(u16::from_le_bytes(take(input, 2)?.try_into().ok()?))
}

pub(crate) fn get_bytes<'a>(input: &mut &'a [u8]) -> Option<&'a [u8]> {
    let length = get_u32(input)? as usize;
    take(input, length)
}

pub(crate) fn get_string(input: &mut &[u8]) -> Option<String> {
    Some(str::from_utf8(get_bytes(input)?).ok()?.to_string())
}

// === receive (canvas event) ===

///
/// crate::event::decode_event
#[derive(Clone)]
pub struct CanvasEvent {
    pub event_type: EventType,
    pub id:         dom::Id,
    pub key:        KeyName,
    pub flags:      u8,
    pub value:      String,
    pub x:          f64,
    pub y:          f64,
    pub local_x:    f64,
    pub local_y:    f64,
    pub time:       f64,
    pub pointer_id: u32,
}

const ALT: Field = Field { position: 1, mask: (1 << 1) - 1 }; // bit 1
const CTRL: Field = Field { position: 2, mask: (1 << 1) - 1 }; // bit 2
const META: Field = Field { position: 3, mask: (1 << 1) - 1 }; // bit 3
const REPEAT: Field = Field { position: 4, mask: (1 << 1) - 1 }; // bit 4
const SHIFT: Field = Field { position: 5, mask: (1 << 1) - 1 }; // bit 5

impl CanvasEvent {
    pub fn root_origin(&self) -> (f64, f64) {
        (self.x - self.local_x, self.y - self.local_y)
    }

    pub fn alt(&self) -> bool {
        ALT.get::<u8>(self.flags as u64) == 1
    }

    pub fn ctrl(&self) -> bool {
        CTRL.get::<u8>(self.flags as u64) == 1
    }

    pub fn meta(&self) -> bool {
        META.get::<u8>(self.flags as u64) == 1
    }

    pub fn repeat(&self) -> bool {
        REPEAT.get::<u8>(self.flags as u64) == 1
    }

    pub fn shift(&self) -> bool {
        SHIFT.get::<u8>(self.flags as u64) == 1
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Attribute {
    Disabled = 1,
    Hidden,
}

impl Attribute {
    fn encode_u16(self) -> u16 {
        self as u16
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClassName {
    Hide = 1,
    Show,
    Hidden,
    Highlighted,
}

impl ClassName {
    fn encode_u16(self) -> u16 {
        self as u16
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Keyword {
    Default = 1,
    EwResize,
    Grab,
    NeswResize,
    NsResize,
    NwseResize,
}

impl Keyword {
    fn encode_u16(self) -> u16 {
        self as u16
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StyleProperty {
    Background = 1,
    Cursor,
    Height,
    Translate,
    Width,
    ZIndex,
}

impl StyleProperty {
    fn encode_u16(self) -> u16 {
        self as u16
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Unit {
    Em = 1,
    Percent,
    Px,
    Rem,
    Vh,
    Vmax,
    Vmin,
    Vw,
}

impl Unit {
    fn encode_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum StyleValue {
    Integer(i32),
    Keyword(Keyword),
    Length(f32, Unit),
    List(Vec<StyleValue>),
    Number(f32),
    Text(String),
}

impl StyleValue {
    fn encode(&self, frame: &mut Vec<u8>) {
        match self {
            Self::Integer(value) => {
                frame.push(1);
                put_i32(frame, *value);
            }
            Self::Keyword(keyword) => {
                frame.push(2);
                put_u16(frame, keyword.encode_u16());
            }
            Self::Length(value, unit) => {
                frame.push(3);
                put_f32(frame, *value);
                frame.push(unit.encode_u8());
            }
            Self::List(items) => {
                frame.push(4);
                frame.push(items.len() as u8);
                for item in items {
                    item.encode(frame);
                }
            }
            Self::Number(value) => {
                frame.push(5);
                put_f32(frame, *value);
            }
            Self::Text(text) => {
                frame.push(6);
                put_str(frame, text);
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FnName {
    HideToast = 1,
    ShowToast,
}

impl FnName {
    fn encode_u16(self) -> u16 {
        self as u16
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Method {
    Delete = 1,
    Get,
    Post,
    Put,
}

impl Method {
    fn encode_u8(self) -> u8 {
        self as u8
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
            13 => Self::Scroll,
            14 => Self::Submit,
            _ => Self::Other,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyName {
    Alt,
    AltGraph,
    Ampersand,
    Apostrophe,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    Asterisk,
    At,
    Backslash,
    Backspace,
    Backtick,
    CapsLock,
    Caret,
    CloseBrace,
    CloseBracket,
    CloseParen,
    Colon,
    Comma,
    ContextMenu,
    Control,
    Delete,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    Dollar,
    DoubleQuote,
    End,
    Enter,
    Equal,
    Escape,
    Exclamation,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Greater,
    Hash,
    Home,
    Insert,
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,
    Less,
    Meta,
    Minus,
    OpenBrace,
    OpenBracket,
    OpenParen,
    PageDown,
    PageUp,
    Percent,
    Period,
    Pipe,
    Plus,
    Question,
    Semicolon,
    Shift,
    Slash,
    Space,
    Tab,
    Tilde,
    Underscore,
    Other,
}

impl KeyName {
    /// ```
    /// # use app::js_client::KeyName;
    /// assert_eq!(KeyName::decode_u8(37), KeyName::Enter);
    /// assert_eq!(KeyName::decode_u8(200), KeyName::Other);
    /// ```
    pub fn decode_u8(value: u8) -> Self {
        match value {
            1 => Self::Alt,
            2 => Self::AltGraph,
            3 => Self::Ampersand,
            4 => Self::Apostrophe,
            5 => Self::ArrowDown,
            6 => Self::ArrowLeft,
            7 => Self::ArrowRight,
            8 => Self::ArrowUp,
            9 => Self::Asterisk,
            10 => Self::At,
            11 => Self::Backslash,
            12 => Self::Backspace,
            13 => Self::Backtick,
            14 => Self::CapsLock,
            15 => Self::Caret,
            16 => Self::CloseBrace,
            17 => Self::CloseBracket,
            18 => Self::CloseParen,
            19 => Self::Colon,
            20 => Self::Comma,
            21 => Self::ContextMenu,
            22 => Self::Control,
            23 => Self::Delete,
            24 => Self::Digit0,
            25 => Self::Digit1,
            26 => Self::Digit2,
            27 => Self::Digit3,
            28 => Self::Digit4,
            29 => Self::Digit5,
            30 => Self::Digit6,
            31 => Self::Digit7,
            32 => Self::Digit8,
            33 => Self::Digit9,
            34 => Self::Dollar,
            35 => Self::DoubleQuote,
            36 => Self::End,
            37 => Self::Enter,
            38 => Self::Equal,
            39 => Self::Escape,
            40 => Self::Exclamation,
            41 => Self::F1,
            42 => Self::F2,
            43 => Self::F3,
            44 => Self::F4,
            45 => Self::F5,
            46 => Self::F6,
            47 => Self::F7,
            48 => Self::F8,
            49 => Self::F9,
            50 => Self::F10,
            51 => Self::F11,
            52 => Self::F12,
            53 => Self::Greater,
            54 => Self::Hash,
            55 => Self::Home,
            56 => Self::Insert,
            57 => Self::KeyA,
            58 => Self::KeyB,
            59 => Self::KeyC,
            60 => Self::KeyD,
            61 => Self::KeyE,
            62 => Self::KeyF,
            63 => Self::KeyG,
            64 => Self::KeyH,
            65 => Self::KeyI,
            66 => Self::KeyJ,
            67 => Self::KeyK,
            68 => Self::KeyL,
            69 => Self::KeyM,
            70 => Self::KeyN,
            71 => Self::KeyO,
            72 => Self::KeyP,
            73 => Self::KeyQ,
            74 => Self::KeyR,
            75 => Self::KeyS,
            76 => Self::KeyT,
            77 => Self::KeyU,
            78 => Self::KeyV,
            79 => Self::KeyW,
            80 => Self::KeyX,
            81 => Self::KeyY,
            82 => Self::KeyZ,
            83 => Self::Less,
            84 => Self::Meta,
            85 => Self::Minus,
            86 => Self::OpenBrace,
            87 => Self::OpenBracket,
            88 => Self::OpenParen,
            89 => Self::PageDown,
            90 => Self::PageUp,
            91 => Self::Percent,
            92 => Self::Period,
            93 => Self::Pipe,
            94 => Self::Plus,
            95 => Self::Question,
            96 => Self::Semicolon,
            97 => Self::Shift,
            98 => Self::Slash,
            99 => Self::Space,
            100 => Self::Tab,
            101 => Self::Tilde,
            102 => Self::Underscore,
            _ => Self::Other,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VisibilityState {
    Hidden,
    Visible,
    Other,
}

impl VisibilityState {
    /// init.js VISIBILITY_STATES
    ///
    /// ```
    /// # use app::js_client::VisibilityState;
    /// assert_eq!(VisibilityState::decode_u8(2), VisibilityState::Visible);
    /// assert_eq!(VisibilityState::decode_u8(0), VisibilityState::Other);
    /// ```
    pub fn decode_u8(value: u8) -> Self {
        match value {
            1 => Self::Hidden,
            2 => Self::Visible,
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
    is_dragging:      bool,
    long_press_fired: bool,
    cancelled:        bool,
}

impl PointerState {
    #[must_use]
    pub const fn is_down(&self) -> bool {
        self.is_down
    }

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
        option::Option::{self, None, Some},
        primitive::{u8, u32, usize},
    };

    use super::{get_u8, get_u32, put_u32};

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

        pub fn encode(&self, frame: &mut Vec<u8>) {
            frame.push(self.0.len() as u8);
            for segment in &self.0 {
                frame.push(segment.tag.encode_u8());
                put_u32(frame, segment.n.unwrap_or(u32::MAX));
            }
        }

        pub fn decode(input: &mut &[u8]) -> Option<Self> {
            let count = get_u8(input)? as usize;
            let mut segments = Vec::with_capacity(count);
            for _ in 0..count {
                let tag = Tag::decode_u8(get_u8(input)?);
                let number = get_u32(input)?;
                segments
                    .push(Segment { tag, n: if number == u32::MAX { None } else { Some(number) } });
            }
            Some(Self(segments))
        }
    }
}

#[cfg(test)]
mod wire_tests {
    use alloc::{format, string::String, vec, vec::Vec};

    use super::*;
    use crate::{
        arena::{ArenaError, PanicError},
        event::EventError,
    };

    const INIT_JS: &str = include_str!("../distribution/init.js");

    const SYMBOLS: [(&str, &str); 32] = [
        ("Ampersand", "&"),
        ("Apostrophe", "'"),
        ("Asterisk", "*"),
        ("At", "@"),
        ("Backslash", "\\"),
        ("Backtick", "`"),
        ("Caret", "^"),
        ("CloseBrace", "}"),
        ("CloseBracket", "]"),
        ("CloseParen", ")"),
        ("Colon", ":"),
        ("Comma", ","),
        ("Dollar", "$"),
        ("DoubleQuote", "\""),
        ("Equal", "="),
        ("Exclamation", "!"),
        ("Greater", ">"),
        ("Hash", "#"),
        ("Less", "<"),
        ("Minus", "-"),
        ("OpenBrace", "{"),
        ("OpenBracket", "["),
        ("OpenParen", "("),
        ("Percent", "%"),
        ("Period", "."),
        ("Pipe", "|"),
        ("Plus", "+"),
        ("Question", "?"),
        ("Semicolon", ";"),
        ("Slash", "/"),
        ("Tilde", "~"),
        ("Underscore", "_"),
    ];

    fn unquote(entry: &str) -> String {
        let inner = &entry[1..entry.len() - 1];
        let mut out = String::new();
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                out.push(chars.next().unwrap());
            } else {
                out.push(c);
            }
        }
        out
    }

    fn js_array(name: &str) -> Vec<String> {
        let head = format!("const {name} = [\n");
        let start = INIT_JS.find(&head).unwrap_or_else(|| panic!("{name} not found")) + head.len();
        let end = start + INIT_JS[start..].find("\n];").unwrap();
        INIT_JS[start..end]
            .lines()
            .map(|line| {
                let entry = line.trim().trim_end_matches(',');
                if entry == "null" { String::new() } else { unquote(entry) }
            })
            .collect()
    }

    fn snake(name: &str) -> String {
        let mut out = String::new();
        for (i, c) in name.chars().enumerate() {
            if c.is_ascii_uppercase() {
                if i > 0 {
                    out.push('_');
                }
                out.push(c.to_ascii_lowercase());
            } else {
                out.push(c);
            }
        }
        out
    }

    fn key_string(key: KeyName) -> String {
        let name = format!("{key:?}");
        if name == "Space" {
            return String::from(" ");
        }
        if let Some(digit) = name.strip_prefix("Digit") {
            return String::from(digit);
        }
        if let Some(letter) = name.strip_prefix("Key") {
            return letter.to_lowercase();
        }
        match SYMBOLS.iter().find(|(variant, _)| *variant == name) {
            Some((_, key)) => String::from(*key),
            None => name,
        }
    }

    fn id_bytes() -> [u8; 11] {
        [2, 14, 255, 255, 255, 255, 3, 3, 0, 0, 0]
    }

    fn header_button_3() -> dom::Id {
        dom::Id::new(&[(dom::Tag::Header, None), (dom::Tag::Button, Some(3))])
    }

    #[test]
    fn key_names_match_init_js() {
        let js = js_array("KEY_NAMES");
        assert_eq!(js[0], "");
        for (i, name) in js.iter().enumerate().skip(1) {
            assert_eq!(&key_string(KeyName::decode_u8(i as u8)), name, "index {i}");
        }
        assert_eq!(KeyName::decode_u8(js.len() as u8), KeyName::Other);
        assert_eq!(KeyName::decode_u8(0), KeyName::Other);
    }

    #[test]
    fn event_types_match_init_js() {
        let js = js_array("EVENT_TYPES");
        assert_eq!(js[0], "");
        for (i, name) in js.iter().enumerate().skip(1) {
            let event_type = EventType::decode_u8(i as u8);
            assert_eq!(&format!("{event_type:?}").to_lowercase(), name, "index {i}");
        }
        assert_eq!(EventType::decode_u8(js.len() as u8), EventType::Other);
        assert_eq!(EventType::decode_u8(0), EventType::Other);
    }

    #[test]
    fn tags_match_init_js() {
        let js = js_array("TAGS");
        assert_eq!(js[0], "");
        for (i, name) in js.iter().enumerate().skip(1) {
            let tag = dom::Tag::decode_u8(i as u8);
            assert_eq!(&format!("{tag:?}").to_lowercase(), name, "index {i}");
            assert_eq!(tag.encode_u8() as usize, i);
        }
        assert_eq!(dom::Tag::decode_u8(js.len() as u8), dom::Tag::Other);
        assert_eq!(dom::Tag::Other.encode_u8(), 0);
    }

    #[test]
    fn visibility_states_match_init_js() {
        let js = js_array("VISIBILITY_STATES");
        assert_eq!(js, ["", "hidden", "visible"]);
        assert_eq!(VisibilityState::decode_u8(1), VisibilityState::Hidden);
        assert_eq!(VisibilityState::decode_u8(2), VisibilityState::Visible);
        assert_eq!(VisibilityState::decode_u8(0), VisibilityState::Other);
        assert_eq!(VisibilityState::decode_u8(3), VisibilityState::Other);
    }

    #[test]
    fn command_tables_match_init_js() {
        let tables: [(&str, Vec<(u16, String)>); 3] = [
            (
                "ATTRIBUTES",
                vec![
                    (Attribute::Disabled as u16, format!("{:?}", Attribute::Disabled)),
                    (Attribute::Hidden as u16, format!("{:?}", Attribute::Hidden)),
                ],
            ),
            (
                "CLASS_NAMES",
                vec![
                    (ClassName::Hide as u16, format!("{:?}", ClassName::Hide)),
                    (ClassName::Show as u16, format!("{:?}", ClassName::Show)),
                    (ClassName::Hidden as u16, format!("{:?}", ClassName::Hidden)),
                    (ClassName::Highlighted as u16, format!("{:?}", ClassName::Highlighted)),
                ],
            ),
            (
                "FN_NAMES",
                vec![
                    (FnName::HideToast as u16, format!("{:?}", FnName::HideToast)),
                    (FnName::ShowToast as u16, format!("{:?}", FnName::ShowToast)),
                ],
            ),
        ];
        for (name, variants) in tables {
            let js = js_array(name);
            assert_eq!(js[0], "", "{name}[0]");
            assert_eq!(js.len(), variants.len() + 1, "{name} length");
            for (value, debug) in variants {
                assert!(value >= 1, "{name} uses 0");
                assert_eq!(js[value as usize], snake(&debug), "{name}[{value}]");
            }
        }
    }

    #[test]
    fn style_tables_match_init_js() {
        let tables: [(&str, Vec<(u16, &str)>); 3] = [
            (
                "STYLE_KEYWORDS",
                vec![
                    (Keyword::Default as u16, "default"),
                    (Keyword::EwResize as u16, "ew-resize"),
                    (Keyword::Grab as u16, "grab"),
                    (Keyword::NeswResize as u16, "nesw-resize"),
                    (Keyword::NsResize as u16, "ns-resize"),
                    (Keyword::NwseResize as u16, "nwse-resize"),
                ],
            ),
            (
                "STYLE_PROPERTIES",
                vec![
                    (StyleProperty::Background as u16, "background"),
                    (StyleProperty::Cursor as u16, "cursor"),
                    (StyleProperty::Height as u16, "height"),
                    (StyleProperty::Translate as u16, "translate"),
                    (StyleProperty::Width as u16, "width"),
                    (StyleProperty::ZIndex as u16, "z-index"),
                ],
            ),
            (
                "STYLE_UNITS",
                vec![
                    (Unit::Em as u16, "em"),
                    (Unit::Percent as u16, "%"),
                    (Unit::Px as u16, "px"),
                    (Unit::Rem as u16, "rem"),
                    (Unit::Vh as u16, "vh"),
                    (Unit::Vmax as u16, "vmax"),
                    (Unit::Vmin as u16, "vmin"),
                    (Unit::Vw as u16, "vw"),
                ],
            ),
        ];
        for (name, variants) in tables {
            let js = js_array(name);
            assert_eq!(js[0], "", "{name}[0]");
            assert_eq!(js.len(), variants.len() + 1, "{name} length");
            for (value, expected) in variants {
                assert!(value >= 1, "{name} uses 0");
                assert_eq!(js[value as usize], expected, "{name}[{value}]");
            }
        }
        let start = INIT_JS.find("function get_style_value(").unwrap();
        let body = &INIT_JS[start..start + INIT_JS[start..].find("\n}\n").unwrap()];
        for tag in 1..=6 {
            assert!(body.contains(&format!("case {tag}:")), "style value tag {tag} missing");
        }
    }

    #[test]
    fn operations_match_init_js() {
        let operations = [
            (OPERATION_SET_TEXT, 1),
            (OPERATION_SET_VALUE, 2),
            (OPERATION_SET_ATTRIBUTE, 3),
            (OPERATION_REMOVE_ATTRIBUTE, 4),
            (OPERATION_ADD_CLASS, 5),
            (OPERATION_REMOVE_CLASS, 6),
            (OPERATION_SET_STYLE, 7),
            (OPERATION_REMOVE_STYLE, 8),
            (OPERATION_SHOW_MODAL, 9),
            (OPERATION_CLOSE_MODAL, 10),
            (OPERATION_FOCUS, 11),
            (OPERATION_JS_FN, 12),
        ];
        for (operation, number) in operations {
            assert_eq!(operation as usize, number);
            assert!(INIT_JS.contains(&format!("case {number:>2}:")), "case {number} missing");
        }
        assert_eq!((OPERATION_ERROR, OPERATION_FETCH), (13, 14));
        assert!(INIT_JS.contains(&format!("operation === {OPERATION_FETCH}")));
        assert!(INIT_JS.contains(&format!("operation === {OPERATION_ERROR}")));
        assert_eq!(js_array("METHODS"), ["", "DELETE", "GET", "POST", "PUT"]);
        assert_eq!(
            [Method::Delete, Method::Get, Method::Post, Method::Put].map(Method::encode_u8),
            [1, 2, 3, 4]
        );
    }

    #[test]
    fn key_flags_match_init_js() {
        let start = INIT_JS.find("function key_flags(").unwrap();
        let body = &INIT_JS[start..start + INIT_JS[start..].find("\n}\n").unwrap()];
        let shifts: Vec<u32> = body
            .split("<< ")
            .skip(1)
            .map(|rest| {
                rest.chars().take_while(char::is_ascii_digit).collect::<String>().parse().unwrap()
            })
            .collect();
        assert_eq!(
            shifts,
            [ALT.position, CTRL.position, META.position, REPEAT.position, SHIFT.position]
        );
        assert!(shifts.iter().all(|position| *position >= 1));
    }

    #[test]
    fn canvas_event_flags_are_independent_bits() {
        let event = |flags| CanvasEvent {
            event_type: EventType::KeyDown,
            id: dom::Id::new(&[]),
            key: KeyName::Enter,
            flags,
            value: String::new(),
            x: 0.0,
            y: 0.0,
            local_x: 0.0,
            local_y: 0.0,
            time: 0.0,
            pointer_id: 0,
        };
        for bit in 0..8u32 {
            let e = event(1 << bit);
            let got = [e.alt(), e.ctrl(), e.meta(), e.repeat(), e.shift()];
            let mut want = [false; 5];
            if (1..=5).contains(&bit) {
                want[bit as usize - 1] = true;
            }
            assert_eq!(got, want, "bit {bit}");
        }
        let all = event(0xFF);
        assert!(all.alt() && all.ctrl() && all.meta() && all.repeat() && all.shift());
    }

    #[test]
    fn primitives_round_trip() {
        let mut frame = Vec::new();
        frame.push(7);
        put_u16(&mut frame, 0x0102);
        put_u32(&mut frame, 0xDEAD_BEEF);
        put_i32(&mut frame, -2);
        put_f32(&mut frame, 1.5);
        frame.extend_from_slice(&2.5f64.to_le_bytes());
        put_str(&mut frame, "日本語");
        assert_eq!(&frame[1..3], [0x02, 0x01]);
        assert_eq!(&frame[7..11], (-2i32).to_le_bytes());

        let mut input = &frame[..];
        assert_eq!(get_u8(&mut input), Some(7));
        input = &input[2..];
        assert_eq!(get_u32(&mut input), Some(0xDEAD_BEEF));
        input = &input[4..];
        assert_eq!(get_f32(&mut input), Some(1.5));
        assert_eq!(get_f64(&mut input), Some(2.5));
        assert_eq!(get_string(&mut input).as_deref(), Some("日本語"));
        assert!(input.is_empty());
        assert_eq!(get_u8(&mut input), None);
    }

    #[test]
    fn bytes_round_trip_and_reject_short_input() {
        let mut frame = Vec::new();
        put_bytes(&mut frame, &[1, 2, 3]);
        put_u16(&mut frame, 0x0102);
        let mut input = &frame[..];
        assert_eq!(get_bytes(&mut input), Some(&[1, 2, 3][..]));
        assert_eq!(get_u16(&mut input), Some(0x0102));
        assert!(input.is_empty());
        assert_eq!(get_bytes(&mut &[4, 0, 0, 0, 1][..]), None);
        assert_eq!(get_u16(&mut &[1][..]), None);
    }

    #[test]
    fn get_functions_reject_short_or_invalid_input() {
        assert_eq!(get_u32(&mut &[1, 2, 3][..]), None);
        assert_eq!(get_f32(&mut &[0; 3][..]), None);
        assert_eq!(get_f64(&mut &[0; 7][..]), None);

        let mut too_long = Vec::new();
        put_u32(&mut too_long, 5);
        too_long.extend_from_slice(b"abcd");
        assert_eq!(get_string(&mut &too_long[..]), None);

        let mut invalid = Vec::new();
        put_u32(&mut invalid, 2);
        invalid.extend_from_slice(&[0xFF, 0xFE]);
        assert_eq!(get_string(&mut &invalid[..]), None);
    }

    #[test]
    fn dom_id_round_trip() {
        let ids = [
            dom::Id::new(&[]),
            dom::Id::new(&[(dom::Tag::Body, None)]),
            header_button_3(),
            dom::Id::new(&[
                (dom::Tag::Main, None),
                (dom::Tag::Section, Some(2)),
                (dom::Tag::Fieldset, Some(u32::MAX - 1)),
            ]),
            dom::Id::new(&[(dom::Tag::Other, Some(0))]),
        ];
        for id in ids {
            let mut frame = Vec::new();
            id.encode(&mut frame);
            let mut input = &frame[..];
            assert_eq!(dom::Id::decode(&mut input), Some(id.clone()));
            assert!(input.is_empty());
            for cut in 0..frame.len() {
                assert_eq!(dom::Id::decode(&mut &frame[..cut]), None, "cut {cut}");
            }
        }
    }

    #[test]
    fn encode_command_layouts() {
        let id = header_button_3();
        let with_id = |operation: u8, tail: &[u8]| {
            let mut frame = vec![operation];
            frame.extend_from_slice(&id_bytes());
            frame.extend_from_slice(tail);
            frame
        };
        let encode = |command: Command| {
            let mut frame = Vec::new();
            encode_command(&mut frame, &command);
            frame
        };

        assert_eq!(
            encode(Command::SetText { id: id.clone(), value: String::from("hi") }),
            with_id(1, &[2, 0, 0, 0, b'h', b'i'])
        );
        assert_eq!(
            encode(Command::SetAttribute {
                id:        id.clone(),
                attribute: Attribute::Hidden,
                value:     String::from("x"),
            }),
            with_id(3, &[2, 0, 1, 0, 0, 0, b'x'])
        );
        assert_eq!(
            encode(Command::AddClass { id: id.clone(), value: ClassName::Hidden }),
            with_id(5, &[3, 0])
        );
        assert_eq!(
            encode(Command::SetStyle {
                id:       id.clone(),
                property: StyleProperty::Width,
                value:    StyleValue::Length(1.5, Unit::Px),
            }),
            with_id(7, &[5, 0, 3, 0, 0, 192, 63, 3])
        );
        assert_eq!(
            encode(Command::SetStyle {
                id:       id.clone(),
                property: StyleProperty::ZIndex,
                value:    StyleValue::Integer(-1),
            }),
            with_id(7, &[6, 0, 1, 255, 255, 255, 255])
        );
        assert_eq!(
            encode(Command::SetStyle {
                id:       id.clone(),
                property: StyleProperty::Cursor,
                value:    StyleValue::Keyword(Keyword::Grab),
            }),
            with_id(7, &[2, 0, 2, 3, 0])
        );
        assert_eq!(
            encode(Command::SetStyle {
                id:       id.clone(),
                property: StyleProperty::Background,
                value:    StyleValue::Text(String::from("red")),
            }),
            with_id(7, &[1, 0, 6, 3, 0, 0, 0, b'r', b'e', b'd'])
        );
        assert_eq!(
            encode(Command::SetStyle {
                id:       id.clone(),
                property: StyleProperty::Translate,
                value:    StyleValue::List(vec![
                    StyleValue::Length(1.5, Unit::Px),
                    StyleValue::Length(-2.5, Unit::Rem),
                ]),
            }),
            with_id(7, &[4, 0, 4, 2, 3, 0, 0, 192, 63, 3, 3, 0, 0, 32, 192, 4])
        );
        assert_eq!(
            encode(Command::SetStyle {
                id:       id.clone(),
                property: StyleProperty::Height,
                value:    StyleValue::Number(0.5),
            }),
            with_id(7, &[3, 0, 5, 0, 0, 0, 63])
        );
        assert_eq!(
            encode(Command::RemoveStyle { id: id.clone(), property: StyleProperty::Cursor }),
            with_id(8, &[2, 0])
        );
        assert_eq!(encode(Command::Focus { id: id.clone() }), with_id(11, &[]));
        assert_eq!(
            encode(Command::JsFn { id: id.clone(), name: FnName::ShowToast }),
            with_id(12, &[2, 0])
        );
        assert_eq!(
            encode(Command::Error { error: Error::Event(EventError::Decode) }),
            [13, 0, 2, 2, 0, 1, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            encode(Command::Error { error: Error::Arena(ArenaError::CommandOverflow) }),
            [13, 0, 2, 1, 0, 1, 0, 0, 0, 0, 0]
        );
        let panic = encode(Command::Error {
            error: Error::Panic(PanicError {
                location: String::from("a.rs:1"),
                message:  String::from("boom"),
            }),
        });
        assert_eq!(&panic[..7], [13, 1, 2, 4, 0, 1, 0]);
        assert_eq!(
            encode(Command::Fetch {
                request: 258,
                method:  Method::Post,
                path:    String::from("/a"),
                body:    vec![9, 8],
            }),
            [14, 2, 1, 0, 0, 3, 2, 0, 0, 0, b'/', b'a', 2, 0, 0, 0, 9, 8]
        );
        assert_eq!(
            &panic[7..],
            [12, 0, 0, 0, b'a', b'.', b'r', b's', b':', b'1', b':', b' ', b'b', b'o', b'o', b'm']
        );
    }
}
