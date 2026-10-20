use alloc::{string::String, vec::Vec};
use core::{
    clone::Clone,
    cmp::{Eq, PartialEq},
    convert::TryInto,
    fmt::Debug,
    marker::Copy,
    option::Option::{self, None, Some},
    primitive::{bool, char, f32, f64, i32, str, u8, u16, u32, usize},
};

use crate::{
    Error, WireError,
    event::{Event, FetchChunk, WindowEvent},
    field::Field,
};

// === send operation ===

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
pub const OPERATION_RELOAD: u8 = 15;

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
    Reload,
    Error { error: Error },
}

/// ```
/// # use app::host::{Command, Output, dom, OPERATION_FOCUS};
/// let mut frame = Vec::new();
/// Command::Focus { id: dom::Id::new(&[(dom::Tag::Body, None)]) }.encode(&mut frame);
/// assert_eq!(frame[0], OPERATION_FOCUS);
/// ```
impl Output for Command {
    fn encode(&self, frame: &mut Vec<u8>) {
        match *self {
            Command::SetText { ref id, ref value } => {
                frame.push(OPERATION_SET_TEXT);
                id.encode(frame);
                str::encode(value, frame);
            }
            Command::SetValue { ref id, ref value } => {
                frame.push(OPERATION_SET_VALUE);
                id.encode(frame);
                str::encode(value, frame);
            }
            Command::SetAttribute { ref id, attribute, ref value } => {
                frame.push(OPERATION_SET_ATTRIBUTE);
                id.encode(frame);
                attribute.encode(frame);
                str::encode(value, frame);
            }
            Command::RemoveAttribute { ref id, attribute } => {
                frame.push(OPERATION_REMOVE_ATTRIBUTE);
                id.encode(frame);
                attribute.encode(frame);
            }
            Command::AddClass { ref id, value } => {
                frame.push(OPERATION_ADD_CLASS);
                id.encode(frame);
                value.encode(frame);
            }
            Command::RemoveClass { ref id, value } => {
                frame.push(OPERATION_REMOVE_CLASS);
                id.encode(frame);
                value.encode(frame);
            }
            Command::SetStyle { ref id, property, ref value } => {
                frame.push(OPERATION_SET_STYLE);
                id.encode(frame);
                property.encode(frame);
                value.encode(frame);
            }
            Command::RemoveStyle { ref id, property } => {
                frame.push(OPERATION_REMOVE_STYLE);
                id.encode(frame);
                property.encode(frame);
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
                name.encode(frame);
            }
            Command::Fetch { request, method, ref path, ref body } => {
                frame.push(OPERATION_FETCH);
                request.encode(frame);
                method.encode(frame);
                str::encode(path, frame);
                <[u8]>::encode(body, frame);
            }
            Command::Reload => frame.push(OPERATION_RELOAD),
            Command::Error { ref error } => error.encode(frame),
        }
    }
}

impl Output for Error {
    fn encode(&self, frame: &mut Vec<u8>) {
        frame.push(OPERATION_ERROR);
        let mut path = Vec::new();
        self.identifiers(&mut path);
        frame.push(path.len() as u8);
        for identifier in path {
            identifier.encode(frame);
        }
        str::encode(&self.detail(), frame);
    }
}

pub trait Input<'a>: Sized {
    fn decode(input: &mut &'a [u8]) -> Option<Self>;
}

pub trait Output {
    fn encode(&self, output: &mut Vec<u8>);
}

fn take<'a>(input: &mut &'a [u8], count: usize) -> Option<&'a [u8]> {
    let (head, tail) = input.split_at_checked(count)?;
    *input = tail;
    Some(head)
}

macro_rules! scalar {
    ($($type:ty),*) => {$(
        impl Input<'_> for $type {
            fn decode(input: &mut &[u8]) -> Option<Self> {
                Some(<$type>::from_le_bytes(take(input, size_of::<$type>())?.try_into().ok()?))
            }
        }

        impl Output for $type {
            fn encode(&self, output: &mut Vec<u8>) {
                output.extend_from_slice(&self.to_le_bytes());
            }
        }
    )*};
}

scalar!(u8, u16, u32, u64, u128, i32, f32, f64);

macro_rules! wire_enum {
    ($($repr:ty: $($type:ty),+);+ $(;)?) => {$($(
        impl Output for $type {
            fn encode(&self, output: &mut Vec<u8>) {
                (*self as $repr).encode(output);
            }
        }
    )+)+};
}

wire_enum!(u16: Attribute, ClassName, FnName, Keyword, StyleProperty; u8: Method, Unit);

impl<'a, T: Input<'a>> Input<'a> for Vec<T> {
    fn decode(input: &mut &'a [u8]) -> Option<Self> {
        let count = u32::decode(input)? as usize;
        (0..count).map(|_| T::decode(input)).collect()
    }
}

impl Input<'_> for String {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        String::from_utf8(Vec::<u8>::decode(input)?).ok()
    }
}

impl<T: Output> Output for [T] {
    fn encode(&self, output: &mut Vec<u8>) {
        (self.len() as u32).encode(output);
        for item in self {
            item.encode(output);
        }
    }
}

impl Output for str {
    fn encode(&self, output: &mut Vec<u8>) {
        self.as_bytes().encode(output);
    }
}

pub trait Decimal: Sized {
    fn from_f64(value: f64) -> Self;
    fn to_f64(&self) -> f64;
}

const FRACTION_UNIT: u64 = 1000;
const FRACTION_WIDTH: usize = 3;

impl<T: Decimal> Input<'_> for T {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        let (whole, rest) = input.split_at_checked(count_leading_digits(input))?;
        let (fraction, rest) = match rest.split_first() {
            Some((b'.', tail)) => tail.split_at_checked(count_leading_digits(tail))?,
            _ => (&[][..], rest),
        };
        if whole.is_empty() && fraction.is_empty() {
            return None;
        }
        *input = rest;
        let mantissa = whole
            .iter()
            .chain(fraction)
            .fold(0.0, |sum, &byte| sum * 10.0 + f64::from(byte - b'0'));
        let scale = fraction.iter().fold(1.0, |scale, _| scale * 10.0);
        Some(T::from_f64(mantissa / scale))
    }
}

impl<T: Decimal> Output for T {
    fn encode(&self, output: &mut Vec<u8>) {
        let value = self.to_f64();
        let scaled = if value.is_finite() {
            libm::round(libm::fabs(value) * FRACTION_UNIT as f64) as u64
        } else {
            0
        };
        if scaled != 0 && value.is_sign_negative() {
            output.push(b'-');
        }
        push_decimal(output, scaled / FRACTION_UNIT, 1);
        let mut fraction = scaled % FRACTION_UNIT;
        if fraction == 0 {
            return;
        }
        let mut width = FRACTION_WIDTH;
        while fraction % 10 == 0 {
            fraction /= 10;
            width -= 1;
        }
        output.push(b'.');
        push_decimal(output, fraction, width);
    }
}

fn count_leading_digits(bytes: &[u8]) -> usize {
    bytes.iter().take_while(|byte| byte.is_ascii_digit()).count()
}

fn push_decimal(output: &mut Vec<u8>, mut value: u64, width: usize) {
    let mut digits = [b'0'; 20];
    let mut start = digits.len();
    while value > 0 || digits.len() - start < width {
        start -= 1;
        digits[start] = b'0' + (value % 10) as u8;
        value /= 10;
    }
    output.extend_from_slice(&digits[start..]);
}

pub fn parse<'a, T: Input<'a>>(source: &'a [u8]) -> Option<T> {
    let mut rest = source;
    let value = T::decode(&mut rest)?;
    rest.is_empty().then_some(value)
}

pub fn stringify<T: Output + ?Sized>(value: &T) -> String {
    let mut output = Vec::new();
    value.encode(&mut output);
    String::from_utf8(output).unwrap_or_default()
}

/// ```
/// # use app::host::parse_url_search_params;
/// let pairs = parse_url_search_params("a=1&b=%E4%BA%88+%E5%AE%9A&a=2");
/// assert_eq!(
///     pairs,
///     [
///         (String::from("a"), String::from("1")),
///         (String::from("b"), String::from("予 定")),
///         (String::from("a"), String::from("2")),
///     ]
/// );
/// ```
pub fn parse_url_search_params(input: &str) -> Vec<(String, String)> {
    input
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (decode_component(key), decode_component(value))
        })
        .collect()
}

fn decode_component(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        index += 1;
        match byte {
            b'+' => decoded.push(b' '),
            b'%' => match bytes.get(index..index + 2).and_then(parse_hex) {
                Some(value) => {
                    decoded.push(value);
                    index += 2;
                }
                None => decoded.push(b'%'),
            },
            other => decoded.push(other),
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn parse_hex(digits: &[u8]) -> Option<u8> {
    let high = (digits[0] as char).to_digit(16)?;
    let low = (digits[1] as char).to_digit(16)?;
    Some((high << 4 | low) as u8)
}

// === receive (canvas event) ===

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

const ALT: Field = Field::new(1, 1); // bit 1
const CTRL: Field = Field::new(2, 1); // bit 2
const META: Field = Field::new(3, 1); // bit 3
const REPEAT: Field = Field::new(4, 1); // bit 4
const SHIFT: Field = Field::new(5, 1); // bit 5

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
    AriaCurrent = 1,
    Checked,
    DataSurround,
    Disabled,
    Hidden,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClassName {
    Hide = 1,
    Show,
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StyleProperty {
    Background = 1,
    Color,
    Cursor,
    GridTemplateColumns,
    GridTemplateRows,
    Height,
    Translate,
    Width,
    ZIndex,
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

#[derive(Clone, PartialEq, Debug)]
pub enum StyleValue {
    Integer(i32),
    Keyword(Keyword),
    Length(f32, Unit),
    List(Vec<StyleValue>),
    Number(f32),
    Text(String),
}

impl Output for StyleValue {
    fn encode(&self, frame: &mut Vec<u8>) {
        match self {
            Self::Integer(value) => {
                frame.push(1);
                value.encode(frame);
            }
            Self::Keyword(keyword) => {
                frame.push(2);
                keyword.encode(frame);
            }
            Self::Length(value, unit) => {
                frame.push(3);
                value.encode(frame);
                unit.encode(frame);
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
                value.encode(frame);
            }
            Self::Text(text) => {
                frame.push(6);
                str::encode(text, frame);
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FnName {
    CopyText = 1,
    EnterFullscreen,
    ExitFullscreen,
    HideToast,
    ShowToast,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Method {
    Delete = 1,
    Get,
    Post,
    Put,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EventType {
    Cancel = 1,
    Change,
    Click,
    Close,
    ContextMenu,
    Copy,
    Cut,
    Drop,
    FocusIn,
    FocusOut,
    Input,
    KeyDown,
    KeyUp,
    Paste,
    PointerCancel,
    PointerDown,
    PointerMove,
    PointerUp,
    Scroll,
    Submit,
    Other = 0,
}

impl EventType {
    /// init.js EVENT_TYPES
    ///
    /// ```
    /// # use app::host::EventType;
    /// assert_eq!(EventType::from_u8(16), EventType::PointerDown);
    /// assert_eq!(EventType::from_u8(200), EventType::Other);
    /// ```
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Cancel,
            2 => Self::Change,
            3 => Self::Click,
            4 => Self::Close,
            5 => Self::ContextMenu,
            6 => Self::Copy,
            7 => Self::Cut,
            8 => Self::Drop,
            9 => Self::FocusIn,
            10 => Self::FocusOut,
            11 => Self::Input,
            12 => Self::KeyDown,
            13 => Self::KeyUp,
            14 => Self::Paste,
            15 => Self::PointerCancel,
            16 => Self::PointerDown,
            17 => Self::PointerMove,
            18 => Self::PointerUp,
            19 => Self::Scroll,
            20 => Self::Submit,
            _ => Self::Other,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyName {
    Alt = 1,
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
    Other = 0,
}

impl KeyName {
    /// ```
    /// # use app::host::KeyName;
    /// assert_eq!(KeyName::from_u8(37), KeyName::Enter);
    /// assert_eq!(KeyName::from_u8(200), KeyName::Other);
    /// ```
    pub fn from_u8(value: u8) -> Self {
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
pub enum FullscreenEvent {
    Change = 1,
    Error,
    Other = 0,
}

impl FullscreenEvent {
    /// ```
    /// # use app::host::FullscreenEvent;
    /// assert_eq!(FullscreenEvent::from_u8(1), FullscreenEvent::Change);
    /// assert_eq!(FullscreenEvent::from_u8(0), FullscreenEvent::Other);
    /// ```
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Change,
            2 => Self::Error,
            _ => Self::Other,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VisibilityState {
    Hidden = 1,
    Visible,
    Other = 0,
}

impl VisibilityState {
    /// init.js VISIBILITY_STATES
    ///
    /// ```
    /// # use app::host::VisibilityState;
    /// assert_eq!(VisibilityState::from_u8(2), VisibilityState::Visible);
    /// assert_eq!(VisibilityState::from_u8(0), VisibilityState::Other);
    /// ```
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Hidden,
            2 => Self::Visible,
            _ => Self::Other,
        }
    }
}

// === receive (event frame) ===

/// pointer / key / input / change / focus, etc.
pub const EVENT_CANVAS: u8 = 1;
pub const EVENT_RESIZE: u8 = 2;
pub const EVENT_SCROLL: u8 = 3;
pub const EVENT_VISIBILITY: u8 = 4;
pub const EVENT_FETCH: u8 = 5;
pub const EVENT_FULLSCREEN: u8 = 6;
pub const EVENT_SHUTDOWN: u8 = 8;

/// ```
/// # use app::{event::{Event, WindowEvent}, host::{EVENT_SHUTDOWN, Input}};
/// assert!(matches!(Event::decode(&mut &[EVENT_SHUTDOWN][..]), Some(Event::Window(WindowEvent::Shutdown))));
/// assert!(Event::decode(&mut &[200][..]).is_none());
/// assert!(Event::decode(&mut &[][..]).is_none());
/// ```
impl Input<'_> for Event {
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
mod event_frame_tests {
    use alloc::{format, vec::Vec};
    use core::matches;

    use super::*;
    use crate::{
        Rng,
        event::{Event, WindowEvent},
    };

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

    use super::{Input, Output};

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Tag {
        Article = 1,
        Body,
        Button,
        Dd,
        Div,
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
        Hgroup,
        Input,
        Label,
        Li,
        Main,
        Modal, // <dialog id="*modal*">
        Nav,
        Ol,
        Option,
        Output,
        P,
        Section,
        Select,
        Span,
        Strong,
        Table,
        Tbody,
        Td,
        Textarea,
        Th,
        Thead,
        Toast,
        Tr,
        Ul,
        Other = 0,
    }

    impl Tag {
        /// init.js::TAGS
        ///
        /// ```
        /// # use app::host::dom::Tag;
        /// assert_eq!(Tag::Article.as_u8(), 1);
        /// assert_eq!(Tag::Other.as_u8(), 0);
        /// ```
        pub fn as_u8(&self) -> u8 {
            *self as u8
        }

        /// ```
        /// # use app::host::dom::Tag;
        /// assert_eq!(Tag::from_u8(1), Tag::Article);
        /// assert_eq!(Tag::from_u8(200), Tag::Other);
        /// ```
        pub fn from_u8(value: u8) -> Self {
            match value {
                1 => Self::Article,
                2 => Self::Body,
                3 => Self::Button,
                4 => Self::Dd,
                5 => Self::Div,
                6 => Self::Dl,
                7 => Self::Drawer,
                8 => Self::Dt,
                9 => Self::Fieldset,
                10 => Self::Footer,
                11 => Self::Form,
                12 => Self::H1,
                13 => Self::H2,
                14 => Self::H3,
                15 => Self::Header,
                16 => Self::Hgroup,
                17 => Self::Input,
                18 => Self::Label,
                19 => Self::Li,
                20 => Self::Main,
                21 => Self::Modal,
                22 => Self::Nav,
                23 => Self::Ol,
                24 => Self::Option,
                25 => Self::Output,
                26 => Self::P,
                27 => Self::Section,
                28 => Self::Select,
                29 => Self::Span,
                30 => Self::Strong,
                31 => Self::Table,
                32 => Self::Tbody,
                33 => Self::Td,
                34 => Self::Textarea,
                35 => Self::Th,
                36 => Self::Thead,
                37 => Self::Toast,
                38 => Self::Tr,
                39 => Self::Ul,
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
            Self(segs.iter().map(|(tag, n)| Segment { tag: *tag, n: *n }).collect())
        }
    }

    impl Output for Id {
        fn encode(&self, output: &mut Vec<u8>) {
            output.push(self.0.len() as u8);
            for segment in &self.0 {
                output.push(segment.tag.as_u8());
                segment.n.unwrap_or(u32::MAX).encode(output);
            }
        }
    }

    impl Input<'_> for Id {
        fn decode(input: &mut &[u8]) -> Option<Self> {
            let count = u8::decode(input)? as usize;
            let mut segments = Vec::with_capacity(count);
            for _ in 0..count {
                let tag = Tag::from_u8(Input::decode(input)?);
                let number = u32::decode(input)?;
                segments
                    .push(Segment { tag, n: if number == u32::MAX { None } else { Some(number) } });
            }
            Some(Self(segments))
        }
    }
}

#[cfg(test)]
mod decimal_tests {
    use alloc::{format, string::String};

    use super::*;
    use crate::Rng;

    struct Sample(f64);

    impl Decimal for Sample {
        fn from_f64(value: f64) -> Self {
            Self(value)
        }

        fn to_f64(&self) -> f64 {
            self.0
        }
    }

    fn number(source: &str) -> Option<f64> {
        parse::<Sample>(source.as_bytes()).map(|Sample(value)| value)
    }

    #[test]
    fn random_decimal_text_agrees_with_std_and_survives_a_round_trip() {
        for seed in 0..2000 {
            let mut rng = Rng::new(seed);
            let digits = |rng: &mut Rng, count: usize| -> String {
                (0..count).map(|_| char::from(b'0' + rng.below(10) as u8)).collect()
            };
            let whole_digits = rng.below(5);
            let whole = digits(&mut rng, whole_digits);
            let dot = rng.chance(70);
            let fraction_digits = rng.below(7);
            let fraction = if dot { digits(&mut rng, fraction_digits) } else { String::new() };
            let source = format!("{whole}{}{fraction}", if dot { "." } else { "" });
            if whole.is_empty() && fraction.is_empty() {
                assert_eq!(number(&source), None, "{source:?}");
                continue;
            }
            let expected = format!(
                "{}.{}",
                if whole.is_empty() { "0" } else { &whole },
                if fraction.is_empty() { "0" } else { &fraction }
            )
            .parse::<f64>()
            .unwrap();
            assert_eq!(number(&source), Some(expected), "{source:?}");

            let signed =
                if rng.chance(50) { -expected } else { expected } + rng.below(1000) as f64 / 7000.0;
            let written = stringify(&Sample(signed));
            let parsed = written.trim_start_matches('-').parse::<f64>().unwrap();
            assert!((parsed - signed.abs()).abs() <= 0.0005 + 1e-9, "{signed} -> {written}");
            assert_eq!(
                written.starts_with('-'),
                signed < 0.0 && parsed != 0.0,
                "{signed} -> {written}"
            );
            assert_eq!(number(written.trim_start_matches('-')), Some(parsed), "{written}");
            assert_eq!(stringify(&Sample(if signed < 0.0 { -parsed } else { parsed })), written);
        }
    }

    #[test]
    fn decode_rejects_non_decimal_text() {
        for source in ["", ".", "-1", "+1", "1e3", "1.2.3", "abc", " 1", "1 ", "NaN", "inf"] {
            assert_eq!(number(source), None, "{source:?}");
        }
    }

    #[test]
    fn decode_approximates_beyond_f64_precision() {
        assert_eq!(number("9007199254740993"), Some(9007199254740992.0));
        let tiny = number("0.000000000000000000000001").unwrap();
        assert!((tiny - 1e-24).abs() < 1e-36);
    }

    #[test]
    fn decode_advances_the_cursor_only_on_success() {
        let mut rest: &[u8] = b"1.75rem";
        assert_eq!(Sample::decode(&mut rest).map(|Sample(value)| value), Some(1.75));
        assert_eq!(rest, b"rem");
        let mut rest: &[u8] = b"1.2.3";
        assert_eq!(Sample::decode(&mut rest).map(|Sample(value)| value), Some(1.2));
        assert_eq!(rest, b".3");
        let mut rest: &[u8] = b"rem";
        assert!(Sample::decode(&mut rest).is_none());
        assert_eq!(rest, b"rem");
    }

    struct Head<'a>(&'a [u8]);

    impl<'a> Input<'a> for Head<'a> {
        fn decode(input: &mut &'a [u8]) -> Option<Self> {
            let count = u8::decode(input)? as usize;
            Some(Self(take(input, count)?))
        }
    }

    #[test]
    fn a_borrowing_type_outlives_the_cursor() {
        let frame: &[u8] = b"\x03abcrest";
        let head = {
            let mut rest = frame;
            let head = Head::decode(&mut rest).unwrap();
            assert_eq!(rest, b"rest");
            head
        };
        assert_eq!(head.0, b"abc");
        assert_eq!(parse::<Head>(b"\x02ab").map(|head| head.0), Some(&b"ab"[..]));
        assert_eq!(parse::<Head>(b"\x02abc").map(|head| head.0), None);
        assert!(Head::decode(&mut &b"\x05ab"[..]).is_none());
    }

    #[test]
    fn encode_rounds_to_three_fraction_digits() {
        assert_eq!(stringify(&Sample(1.23456)), "1.235");
        assert_eq!(stringify(&Sample(0.0004)), "0");
        assert_eq!(stringify(&Sample(f64::NAN)), "0");
        assert_eq!(stringify(&Sample(f64::INFINITY)), "0");
    }
}

#[cfg(test)]
mod wire_tests {
    use alloc::{format, string::String, vec, vec::Vec};

    use super::*;
    use crate::{
        Rng,
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
        [2, 15, 255, 255, 255, 255, 3, 3, 0, 0, 0]
    }

    fn header_button_3() -> dom::Id {
        dom::Id::new(&[(dom::Tag::Header, None), (dom::Tag::Button, Some(3))])
    }

    #[test]
    fn key_names_match_init_js() {
        let js = js_array("KEY_NAMES");
        assert_eq!(js[0], "");
        for (i, name) in js.iter().enumerate().skip(1) {
            assert_eq!(&key_string(KeyName::from_u8(i as u8)), name, "index {i}");
        }
        assert_eq!(KeyName::from_u8(js.len() as u8), KeyName::Other);
        assert_eq!(KeyName::from_u8(0), KeyName::Other);
    }

    #[test]
    fn event_types_match_init_js() {
        let js = js_array("EVENT_TYPES");
        assert_eq!(js[0], "");
        for (i, name) in js.iter().enumerate().skip(1) {
            let event_type = EventType::from_u8(i as u8);
            assert_eq!(&format!("{event_type:?}").to_lowercase(), name, "index {i}");
        }
        assert_eq!(EventType::from_u8(js.len() as u8), EventType::Other);
        assert_eq!(EventType::from_u8(0), EventType::Other);
    }

    #[test]
    fn tags_match_init_js() {
        let js = js_array("TAGS");
        assert_eq!(js[0], "");
        for (i, name) in js.iter().enumerate().skip(1) {
            let tag = dom::Tag::from_u8(i as u8);
            assert_eq!(&format!("{tag:?}").to_lowercase(), name, "index {i}");
            assert_eq!(tag.as_u8() as usize, i);
        }
        assert_eq!(dom::Tag::from_u8(js.len() as u8), dom::Tag::Other);
        assert_eq!(dom::Tag::Other.as_u8(), 0);
    }

    #[test]
    fn visibility_states_match_init_js() {
        let js = js_array("VISIBILITY_STATES");
        assert_eq!(js, ["", "hidden", "visible"]);
        assert_eq!(VisibilityState::from_u8(1), VisibilityState::Hidden);
        assert_eq!(VisibilityState::from_u8(2), VisibilityState::Visible);
        assert_eq!(VisibilityState::from_u8(0), VisibilityState::Other);
        assert_eq!(VisibilityState::from_u8(3), VisibilityState::Other);
    }

    #[test]
    fn fullscreen_events_match_init_js() {
        let js = js_array("FULLSCREEN_EVENTS");
        assert_eq!(js[0], "");
        for (i, name) in js.iter().enumerate().skip(1) {
            let event = FullscreenEvent::from_u8(i as u8);
            assert_eq!(&format!("fullscreen{event:?}").to_lowercase(), name, "index {i}");
        }
        assert_eq!(FullscreenEvent::from_u8(js.len() as u8), FullscreenEvent::Other);
        assert_eq!(FullscreenEvent::from_u8(0), FullscreenEvent::Other);
    }

    #[test]
    fn command_tables_match_init_js() {
        let tables: [(&str, Vec<(u16, String)>); 2] = [
            (
                "CLASS_NAMES",
                vec![
                    (ClassName::Hide as u16, format!("{:?}", ClassName::Hide)),
                    (ClassName::Show as u16, format!("{:?}", ClassName::Show)),
                ],
            ),
            (
                "FN_NAMES",
                vec![
                    (FnName::CopyText as u16, format!("{:?}", FnName::CopyText)),
                    (FnName::EnterFullscreen as u16, format!("{:?}", FnName::EnterFullscreen)),
                    (FnName::ExitFullscreen as u16, format!("{:?}", FnName::ExitFullscreen)),
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
        let tables: [(&str, Vec<(u16, &str)>); 4] = [
            (
                "ATTRIBUTES",
                vec![
                    (Attribute::AriaCurrent as u16, "aria-current"),
                    (Attribute::Checked as u16, "checked"),
                    (Attribute::DataSurround as u16, "data-surround"),
                    (Attribute::Disabled as u16, "disabled"),
                    (Attribute::Hidden as u16, "hidden"),
                ],
            ),
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
                    (StyleProperty::Color as u16, "color"),
                    (StyleProperty::Cursor as u16, "cursor"),
                    (StyleProperty::GridTemplateColumns as u16, "grid-template-columns"),
                    (StyleProperty::GridTemplateRows as u16, "grid-template-rows"),
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
        assert_eq!((OPERATION_ERROR, OPERATION_FETCH, OPERATION_RELOAD), (13, 14, 15));
        assert!(INIT_JS.contains(&format!("operation === {OPERATION_FETCH}")));
        assert!(INIT_JS.contains(&format!("operation === {OPERATION_ERROR}")));
        assert!(INIT_JS.contains(&format!("operation === {OPERATION_RELOAD}")));
        assert_eq!(js_array("METHODS"), ["", "DELETE", "GET", "POST", "PUT"]);
        let mut methods = Vec::new();
        for method in [Method::Delete, Method::Get, Method::Post, Method::Put] {
            method.encode(&mut methods);
        }
        assert_eq!(methods, [1, 2, 3, 4]);
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
    fn url_search_params_split_decode_and_keep_order() {
        let pairs = parse_url_search_params;
        let owned = |list: &[(&str, &str)]| {
            list.iter()
                .map(|(key, value)| (String::from(*key), String::from(*value)))
                .collect::<Vec<_>>()
        };
        assert!(pairs("").is_empty());
        assert_eq!(pairs("a=1&b=2"), owned(&[("a", "1"), ("b", "2")]));
        assert_eq!(pairs("a=1&a=2&a=3"), owned(&[("a", "1"), ("a", "2"), ("a", "3")]));
        assert_eq!(
            pairs("note=a+b%26c%3D%2B&empty=&bare"),
            owned(&[("note", "a b&c=+"), ("empty", ""), ("bare", ""),])
        );
        assert_eq!(pairs("t=%E4%BA%88%E5%AE%9A&&"), owned(&[("t", "予定")]));
        assert_eq!(pairs("a=%zz&b=%4&c=%"), owned(&[("a", "%zz"), ("b", "%4"), ("c", "%")]));
        assert_eq!(pairs("a=%FF"), owned(&[("a", "\u{FFFD}")]));
        assert_eq!(pairs("a%20b=c=d"), owned(&[("a b", "c=d")]));
    }

    #[test]
    fn random_frames_round_trip_and_every_proper_prefix_is_rejected() {
        type Fields = (u8, u16, u32, i32, u32, u64, String, Vec<u8>, dom::Id);
        fn decode_all(input: &mut &[u8]) -> Option<Fields> {
            Some((
                u8::decode(input)?,
                u16::decode(input)?,
                u32::decode(input)?,
                i32::decode(input)?,
                f32::decode(input)?.to_bits(),
                f64::decode(input)?.to_bits(),
                String::decode(input)?,
                <Vec<u8>>::decode(input)?,
                dom::Id::decode(input)?,
            ))
        }
        for seed in 0..500 {
            let mut rng = Rng::new(seed);
            let text = rng.string();
            let length = rng.below(30);
            let bytes = rng.bytes(length);
            let id = rng.id();
            let fields: Fields = (
                rng.next_u64() as u8,
                rng.next_u64() as u16,
                rng.next_u64() as u32,
                rng.next_u64() as i32,
                rng.next_u64() as u32,
                rng.next_u64(),
                text,
                bytes,
                id,
            );

            let mut frame = Vec::new();
            fields.0.encode(&mut frame);
            fields.1.encode(&mut frame);
            fields.2.encode(&mut frame);
            fields.3.encode(&mut frame);
            f32::from_bits(fields.4).encode(&mut frame);
            f64::from_bits(fields.5).encode(&mut frame);
            str::encode(&fields.6, &mut frame);
            <[u8]>::encode(&fields.7, &mut frame);
            fields.8.encode(&mut frame);

            let mut input = &frame[..];
            assert_eq!(decode_all(&mut input), Some(fields), "seed {seed}");
            assert!(input.is_empty(), "seed {seed}");
            for cut in 0..frame.len() {
                assert_eq!(decode_all(&mut &frame[..cut]), None, "seed {seed} cut {cut}");
            }
        }
    }

    #[test]
    fn strings_reject_invalid_utf8_and_overlong_lengths() {
        let mut invalid = Vec::new();
        2u32.encode(&mut invalid);
        invalid.extend_from_slice(&[0xFF, 0xFE]);
        assert_eq!(String::decode(&mut &invalid[..]), None);

        let mut too_long = Vec::new();
        5u32.encode(&mut too_long);
        too_long.extend_from_slice(b"abcd");
        assert_eq!(String::decode(&mut &too_long[..]), None);
    }

    #[test]
    fn command_layouts() {
        let id = header_button_3();
        let with_id = |operation: u8, tail: &[u8]| {
            let mut frame = vec![operation];
            frame.extend_from_slice(&id_bytes());
            frame.extend_from_slice(tail);
            frame
        };
        let encode = |command: Command| {
            let mut frame = Vec::new();
            command.encode(&mut frame);
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
            with_id(3, &[5, 0, 1, 0, 0, 0, b'x'])
        );
        assert_eq!(
            encode(Command::AddClass { id: id.clone(), value: ClassName::Show }),
            with_id(5, &[2, 0])
        );
        assert_eq!(
            encode(Command::SetStyle {
                id:       id.clone(),
                property: StyleProperty::Width,
                value:    StyleValue::Length(1.5, Unit::Px),
            }),
            with_id(7, &[8, 0, 3, 0, 0, 192, 63, 3])
        );
        assert_eq!(
            encode(Command::SetStyle {
                id:       id.clone(),
                property: StyleProperty::ZIndex,
                value:    StyleValue::Integer(-1),
            }),
            with_id(7, &[9, 0, 1, 255, 255, 255, 255])
        );
        assert_eq!(
            encode(Command::SetStyle {
                id:       id.clone(),
                property: StyleProperty::Cursor,
                value:    StyleValue::Keyword(Keyword::Grab),
            }),
            with_id(7, &[3, 0, 2, 3, 0])
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
            with_id(7, &[7, 0, 4, 2, 3, 0, 0, 192, 63, 3, 3, 0, 0, 32, 192, 4])
        );
        assert_eq!(
            encode(Command::SetStyle {
                id:       id.clone(),
                property: StyleProperty::Height,
                value:    StyleValue::Number(0.5),
            }),
            with_id(7, &[6, 0, 5, 0, 0, 0, 63])
        );
        assert_eq!(
            encode(Command::RemoveStyle { id: id.clone(), property: StyleProperty::Cursor }),
            with_id(8, &[3, 0])
        );
        assert_eq!(encode(Command::Focus { id: id.clone() }), with_id(11, &[]));
        assert_eq!(
            encode(Command::JsFn { id: id.clone(), name: FnName::ShowToast }),
            with_id(12, &[5, 0])
        );
        assert_eq!(
            encode(Command::Error { error: Error::Event(EventError::Decode) }),
            [13, 2, 2, 0, 1, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            encode(Command::Error { error: Error::Arena(ArenaError::CommandOverflow) }),
            [13, 2, 1, 0, 1, 0, 0, 0, 0, 0]
        );
        let panic = encode(Command::Error {
            error: Error::Panic(PanicError {
                location: String::from("a.rs:1"),
                message:  String::from("boom"),
            }),
        });
        assert_eq!(&panic[..6], [13, 2, 3, 0, 1, 0]);
        assert_eq!(encode(Command::Reload), [15]);
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
            &panic[6..],
            [12, 0, 0, 0, b'a', b'.', b'r', b's', b':', b'1', b':', b' ', b'b', b'o', b'o', b'm']
        );
    }
}
