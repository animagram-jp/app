use wasm_bindgen::JsValue;
use js_sys::Reflect;
use serde::Serialize;

// ============================================================
// send operation
// ============================================================

pub enum Operation {
    SetText,
    SetValue,
    SetAttribute,
    RemoveAttribute,
    AddClass,
    RemoveClass,
    SetWidth,
    SetHeight,
    SetZIndex,
    SetBackground,
    SetTranslate,
    ShowModal,
    CloseModal,
    Focus,
    JsFn,
}

impl Operation {
    pub fn as_u8(&self) -> u8 {
        match self {
            Self::SetText         =>  1,
            Self::SetValue        =>  2,
            Self::SetAttribute    =>  3,
            Self::RemoveAttribute =>  4,
            Self::AddClass        =>  5,
            Self::RemoveClass     =>  6,
            Self::SetWidth        =>  7,
            Self::SetHeight       =>  8,
            Self::SetZIndex       =>  9,
            Self::SetBackground   => 10,
            Self::SetTranslate    => 11,
            Self::ShowModal       => 12,
            Self::CloseModal      => 13,
            Self::Focus           => 14,
            Self::JsFn            => 15,
        }
    }
}

#[derive(Serialize)]
pub struct Command {
    operation: u8,
    id:        String,
    #[serde(skip_serializing_if = "Option::is_none")]
    attribute: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value:     Option<String>,
}

impl Command {
    pub fn new(operation: Operation, id: &str, attribute: Option<&str>, value: Option<&str>) -> Self {
        Self {
            operation: operation.as_u8(),
            id:        id.to_string(),
            attribute: attribute.map(str::to_string),
            value:     value.map(str::to_string),
        }
    }
}

// ============================================================
// receive (js value)
// ============================================================

/// js由来の文字列をstrとして取得
pub fn get_js_str(obj: &JsValue, key: &str) -> Option<String> {
    Reflect::get(obj, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_string())
}

/// js由来の整数をu32として取得
pub fn get_js_u32(obj: &JsValue, key: &str) -> u32 {
    Reflect::get(obj, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_f64())
        .and_then(|f| {
            if f >= 0.0 && f <= u32::MAX as f64 && f.fract() == 0.0 {
                Some(f as u32)
            } else {
                None
            }
        })
        .unwrap_or(0)
}

/// js由来の整数をi32として取得
pub fn get_js_i32(obj: &JsValue, key: &str) -> i32 {
    Reflect::get(obj, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_f64())
        .and_then(|f| {
            if f >= i32::MIN as f64 && f <= i32::MAX as f64 && f.fract() == 0.0 {
                Some(f as i32)
            } else {
                None
            }
        })
        .unwrap_or(0)
}

/// js由来の小数をf64として取得
pub fn get_js_f64(obj: &JsValue, key: &str) -> Option<f64> {
    Reflect::get(obj, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_f64())
        .and_then(|f| {
            if f.is_finite() {
                Some(f)
            } else {
                None
            }
        })
}

/// js由来のデータを構造体のまま取得
pub fn get_js_field(obj: &JsValue, key: &str) -> Option<JsValue> {
    Reflect::get(obj, &JsValue::from_str(key)).ok()
}

pub enum EventType {
    Submit,
    Click,
    ContextMenu,
    KeyDown,
    Input,
    Change,
    FocusIn,
    FocusOut,
    Resize,
    Scroll,
    Drop,
    PointerDown,
    PointerUp,
    PointerMove,
    PointerCancel,
    Other,
}

impl EventType {
    pub fn decode(event_type: &str) -> Self {
        match event_type {
            "submit"       => Self::Submit,
            "click"        => Self::Click,
            "contextmenu"  => Self::ContextMenu,
            "keydown"      => Self::KeyDown,
            "input"        => Self::Input,
            "change"       => Self::Change,
            "focusin"      => Self::FocusIn,
            "focusout"     => Self::FocusOut,
            "resize"       => Self::Resize,
            "scroll"       => Self::Scroll,
            "drop"         => Self::Drop,
            "pointerdown"  => Self::PointerDown,
            "pointerup"    => Self::PointerUp,
            "pointermove"  => Self::PointerMove,
            "pointercancel"=> Self::PointerCancel,
            _              => Self::Other,
        }
    }
}

pub enum KeyName {
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Enter,
    Escape,
    Tab,
    Backspace,
    Other,
}

impl KeyName {
    pub fn decode(key_name: &str) -> Self {
        match key_name {
            "ArrowUp"    => Self::ArrowUp,
            "ArrowDown"  => Self::ArrowDown,
            "ArrowLeft"  => Self::ArrowLeft,
            "ArrowRight" => Self::ArrowRight,
            "Enter"      => Self::Enter,
            "Escape"     => Self::Escape,
            "Tab"        => Self::Tab,
            "Backspace"  => Self::Backspace,
            _            => Self::Other,
        }
    }
}

// ============================================================
// device
// ============================================================

pub enum Device {
    Touch,
    Mouse,
}

// pointer_coarse: window.matchMedia('(pointer: coarse)').matches
pub fn detect_device(pointer_coarse: bool) -> Device {
    if pointer_coarse { Device::Touch } else { Device::Mouse }
}

// ============================================================
// gesture: long press, swipe (up,down,left,right), drag
// ============================================================

pub enum Gesture {
    LongPress,
    SwipeUp,
    SwipeDown,
    SwipeLeft,
    SwipeRight,
    Drag { x: f64, y: f64 },
    DragEnd,
}

// pointerdown:   is_down = true, 座標・時刻記録, タイマー起動
// pointermove:   座標がブレていたら長押しキャンセル (指がズレた)
// pointerup:     経過時間で click か 長押し か判定
// pointercancel: 全部リセット (割り込まれた時)
#[derive(Default, Clone, Copy)]
pub struct PointerState {
    is_down:    bool,   // default: false
    start_x:    f64,    // default: 0.0
    start_y:    f64,    // default: 0.0
    current_x:  f64,    // default: 0.0
    current_y:  f64,    // default: 0.0
    start_time: f64,    // default: 0.0
    drag_offset: (f64, f64), // PointerDown時の (pointer_px - カード左上px)
    drag_px:     (f64, f64), // Drag中のカード左上px座標(一時)
    is_dragging: bool,       // Dragジェスチャが1回以上発火した
}

impl PointerState {
    // payloadから必要な値を全て引数で受け取り、新しい状態を返す
    pub fn update(self, event_type: &EventType, x: f64, y: f64, time: f64) -> Self {
        match event_type {
            EventType::PointerDown => Self {
                is_down:     true,
                start_x:     x,
                start_y:     y,
                current_x:   x,
                current_y:   y,
                start_time:  time,
                drag_offset: (0.0, 0.0),
                drag_px:     (0.0, 0.0),
                is_dragging: false,
            },
            EventType::PointerMove => Self {
                current_x: x,
                current_y: y,
                ..self
            },
            EventType::PointerUp | EventType::PointerCancel => Self {
                is_dragging: false,
                ..Self::default()
            },
            _ => self,
        }
    }
}

pub fn detect_gesture(state: &mut PointerState, prev_state: &PointerState, event_type: &EventType, current_time: f64) -> Option<Gesture> {
    if !state.is_down {
        if prev_state.is_dragging {
            return Some(Gesture::DragEnd);
        }
        return None;
    }

    let dx = state.current_x - state.start_x;
    let dy = state.current_y - state.start_y;
    let dt = current_time - state.start_time;
    let distance = (dx * dx + dy * dy).sqrt();

    // long press: 時間長い + 座標ブレ小さい
    if dt > 251.0 && distance < 9.0 {
        return Some(Gesture::LongPress);
    }

    // swipe: PointerUp時のみ + velocity > 0.5 px/ms + duration < 250ms
    if matches!(event_type, EventType::PointerUp) && dt > 0.0 {
        let velocity = distance / dt;
        if velocity > 0.5 && distance > 50.0 && dt < 250.0 {
            return Some(if dx.abs() > dy.abs() {
                if dx > 0.0 { Gesture::SwipeRight } else { Gesture::SwipeLeft }
            } else {
                if dy > 0.0 { Gesture::SwipeDown } else { Gesture::SwipeUp }
            });
        }
    }

    // drag: PointerMove中に距離が閾値超え → 差分を返す
    if matches!(event_type, EventType::PointerMove) && distance > 10.0 {
        state.is_dragging = true;
        return Some(Gesture::Drag { x: state.current_x, y: state.current_y });
    }

    None
}

// ============================================================
// dom (rust item <=> element id)
// ============================================================
//
// id規則:
//   "_" = 親子セグメント区切り  例: main_div_section-1
//   "-N" = 同タグ内の連番       例: span-3, th-2
//   連番なし = その階層に1つだけ 例: thead_tr, legend_h5
//
// dom::Id::encode()  -> "seg1_seg2_seg-N_..."
// dom::Id::decode()  -> Vec<dom::Segment> のパース

pub mod dom {
    use core::{option::Option::{self, Some, None}, result::Result::Ok, marker::Copy, fmt, cmp::PartialEq, clone::Clone};
    use alloc::{vec::Vec, string::String, format};

    #[derive(Debug, Clone, PartialEq)]
    pub enum Tag {
        Head,
        Header,
        H1, H2, H3,
        Ul, Li,
        Button,
        Main,
        Section,
        Span,
        Dl, Dt, Dd,
        Ol,
        P,
        Textarea,
        Drawer, // <dialog id="*drawer*">
        Modal,  // <dialog id="*modal*">
        Form,
        Input,
        Fieldset,
        Table, Thead, Tbody, Tr, Th, Td,
        Select,
        Footer,
        Output, Article,
        Other,
    }

    impl Tag {
        pub fn decode(s: &str) -> Self {
            match s {
                "head"     => Self::Head,
                "header"   => Self::Header,
                "h1"       => Self::H1,
                "h2"       => Self::H2,
                "h3"       => Self::H3,
                "ul"       => Self::Ul,
                "li"       => Self::Li,
                "button"   => Self::Button,
                "main"     => Self::Main,
                "section"  => Self::Section,
                "span"     => Self::Span,
                "dl"       => Self::Dl,
                "dt"       => Self::Dt,
                "dd"       => Self::Dd,
                "ol"       => Self::Ol,
                "p"        => Self::P,
                "textarea" => Self::Textarea,
                "drawer"   => Self::Drawer,
                "modal"    => Self::Modal,
                "form"     => Self::Form,
                "input"    => Self::Input,
                "fieldset" => Self::Fieldset,
                "table"    => Self::Table,
                "thead"    => Self::Thead,
                "tbody"    => Self::Tbody,
                "tr"       => Self::Tr,
                "th"       => Self::Th,
                "td"       => Self::Td,
                "select"   => Self::Select,
                "footer"   => Self::Footer,
                "output"   => Self::Output,
                "article"  => Self::Article,
                _          => Self::Other,
            }
        }

        pub fn encode(&self) -> &'static str {
            match self {
                Self::Head     => "head",
                Self::Header   => "header",
                Self::H1       => "h1",
                Self::H2       => "h2",
                Self::H3       => "h3",
                Self::Ul       => "ul",
                Self::Li       => "li",
                Self::Button   => "button",
                Self::Main     => "main",
                Self::Section  => "section",
                Self::Span     => "span",
                Self::Dl       => "dl",
                Self::Dt       => "dt",
                Self::Dd       => "dd",
                Self::Ol       => "ol",
                Self::P        => "p",
                Self::Textarea => "textarea",
                Self::Drawer   => "drawer",
                Self::Modal    => "modal",
                Self::Form     => "form",
                Self::Input    => "input",
                Self::Fieldset => "fieldset",
                Self::Table    => "table",
                Self::Thead    => "thead",
                Self::Tbody    => "tbody",
                Self::Tr       => "tr",
                Self::Th       => "th",
                Self::Td       => "td",
                Self::Select   => "select",
                Self::Footer   => "footer",
                Self::Output   => "output",
                Self::Article  => "article",
                Self::Other    => "",
            }
        }
    }

    // セグメント1つ: タグ + オプション連番
    #[derive(Debug, Clone, PartialEq)]
    pub struct Segment {
        pub tag: Tag,
        pub n:   Option<u32>,
    }

    impl Segment {
        pub fn new(tag: Tag) -> Self { Self { tag, n: None } }
        pub fn numbered(tag: Tag, n: u32) -> Self { Self { tag, n: Some(n) } }

        pub fn decode(s: &str) -> Self {
            if let Some(pos) = s.rfind('-') {
                let (tag, num) = s.split_at(pos);
                if let Ok(n) = num[1..].parse::<u32>() {
                    return Self::numbered(Tag::decode(tag), n);
                }
            }
            Self::new(Tag::decode(s))
        }

        pub fn encode(&self) -> String {
            match self.n {
                Some(n) => format!("{}-{}", self.tag.encode(), n),
                None    => self.tag.encode().to_string(),
            }
        }
    }

    // id全体: セグメントのリスト
    #[derive(Debug, Clone, PartialEq)]
    pub struct Id(pub Vec<Segment>);

    impl Id {
        pub fn new(segs: &[(Tag, Option<u32>)]) -> Self {
            Self(segs.iter().map(|(tag, n)| Segment { tag: tag.clone(), n: *n }).collect())
        }

        pub fn decode(id: &str) -> Self {
            Self(id.split('_').map(Segment::decode).collect())
        }

        pub fn encode(&self) -> String {
            self.0.iter()
                .map(Segment::encode)
                .collect::<Vec<_>>()
                .join("_")
        }

        pub fn last_tag(&self) -> Option<&Tag> {
            self.0.last().map(|s| &s.tag)
        }
    }
}

// ============================================================
// canvas event
// ============================================================

pub struct CanvasEvent {
    pub event_type: EventType,
    pub id:         dom::Id,
    pub key:        KeyName,
    pub value:      String,
    pub x:          f64,
    pub y:          f64,
    pub time:       f64,
}

impl CanvasEvent {
    pub fn decode(payload: &wasm_bindgen::JsValue) -> Self {
        let event_type = get_js_str(payload, "event_type").as_deref().map(EventType::decode).unwrap_or(EventType::Other);
        let id         = get_js_str(payload, "target_id").as_deref().map(dom::Id::decode).unwrap_or_else(|| dom::Id(vec![]));
        let key        = get_js_str(payload, "key").as_deref().map(KeyName::decode).unwrap_or(KeyName::Other);
        let value      = get_js_str(payload, "value").unwrap_or_default();
        let x          = get_js_f64(payload, "x").unwrap_or(0.0);
        let y          = get_js_f64(payload, "y").unwrap_or(0.0);
        let time       = get_js_f64(payload, "time").unwrap_or(0.0);
        Self { event_type, id, key, value, x, y, time }
    }
}