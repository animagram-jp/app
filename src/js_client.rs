use wasm_bindgen::JsValue;
use js_sys::Reflect;
use serde_wasm_bindgen::to_value;
use serde::Serialize;

// ============================================================
// send (dom operation)
// ============================================================

const OPERATION_SET_TEXT:     u8 = 1;
const OPERATION_SET_VALUE:    u8 = 2;
const OPERATION_SET_ATTR:     u8 = 3;
const OPERATION_ADD_CLASS:    u8 = 4;
const OPERATION_REMOVE_CLASS: u8 = 5;
const OPERATION_FOCUS:        u8 = 6;
const OPERATION_OPEN_MODAL:   u8 = 7;
const OPERATION_CLOSE_MODAL:  u8 = 8;
const OPERATION_JS_CLASS:     u8 = 9;

#[derive(serde::Serialize)]
pub struct DomCmd {
    operation: u8,
    id:        String,
    #[serde(skip_serializing_if = "Option::is_none")]
    attribute: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value:     Option<String>,
}

// ============================================================
// receive (js value)
// ============================================================

pub enum EventType {
    Submit,
    Click,
    ContextMenu,
    KeyDown,
    Input,
    Change,
    Focus,
    Blur,
    Resize,
    Scroll,
    Drop,
    PointerDown,
    PointerUp,
    PointerMove,
    PointerCancel,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Submit      => "submit",
            Self::Click       => "click",
            Self::ContextMenu => "contextmenu",
            Self::KeyDown     => "keydown",
            Self::Input       => "input",
            Self::Change      => "change",
            Self::Focus       => "focus",
            Self::Blur        => "blur",
            Self::Resize      => "resize",
            Self::Scroll      => "scroll",
            Self::Drop        => "drop"
            Self::PointerDown => "pointerdown",
            Self::PointerUp   => "pointerup",
            Self::PointerMove => "pointermove",
            Self::PointerCancel => "pointercancel",
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
}

impl KeyName {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ArrowUp    => "ArrowUp",
            Self::ArrowDown  => "ArrowDown",
            Self::ArrowLeft  => "ArrowLeft",
            Self::ArrowRight => "ArrowRight",
            Self::Enter      => "Enter",
            Self::Escape     => "Escape",
            Self::Tab        => "Tab",
            Self::Backspace  => "Backspace",
        }
    }
}

/// js由来の文字列をstrとして取得
fn js_get_str(obj: &JsValue, key: &str) -> Option<String> {
    js_sys::Reflect::get(obj, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_string())
}

/// js由来の整数をu32として取得
fn js_get_u32(obj: &JsValue, key: &str) -> u32 {
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

/// js由来の小数をf64として取得
fn js_get_f64(obj: &JsValue, key: &str) -> Option<f64> {
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
fn js_get_field(obj: &JsValue, key: &str) -> Option<JsValue> {
    Reflect::get(obj, &JsValue::from_str(key)).ok()
}