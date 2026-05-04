use wasm_bindgen::JsValue;
use js_sys::Reflect;
use serde::Serialize;

// ============================================================
// send (dom operation)
// ============================================================

pub enum Operation {
    SetText,
    SetValue,
    SetAttr,
    AddClass,
    RemoveClass,
    Focus,
    OpenModal,
    CloseModal,
    JsClass,
}

impl Operation {
    pub fn as_u8(&self) -> u8 {
        match self {
            Self::SetText     => 1,
            Self::SetValue    => 2,
            Self::SetAttr     => 3,
            Self::AddClass    => 4,
            Self::RemoveClass => 5,
            Self::Focus       => 6,
            Self::OpenModal   => 7,
            Self::CloseModal  => 8,
            Self::JsClass     => 9,
        }
    }
}

#[derive(Serialize)]
pub struct DomCmd {
    operation: u8,
    id:        String,
    #[serde(skip_serializing_if = "Option::is_none")]
    attribute: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value:     Option<String>,
}

impl DomCmd {
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
    Other,
}

impl KeyName {
    pub fn decode(s: &str) -> Self {
        match s {
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