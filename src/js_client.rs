use wasm_bindgen::JsValue;
use js_sys::Reflect;
use serde::Serialize;
use crate::table::Roll;
use crate::character::{Model, schema};

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

pub enum EventType {
    Submit,
    Click,
    ContextMenu,
    KeyDown,
    Input,
    Change,
    FocusIn,
    Blur,
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
            "blur"         => Self::Blur,
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
// gesture: long press, swipe (up,down,left,right), drag
// ============================================================

pub enum Gesture {
    LongPress,
    SwipeUp,
    SwipeDown,
    SwipeLeft,
    SwipeRight,
    Drag,
}

// pointerdown:   is_down = true, 座標・時刻記録, タイマー起動
// pointermove:   座標がブレてたら長押しキャンセル (指がズレた)
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
}

impl PointerState {
    // payloadから必要な値を全て引数で受け取り、新しい状態を返す
    pub fn update(self, event_type: &EventType, x: f64, y: f64, time: f64) -> Self {
        match event_type {
            EventType::PointerDown => Self {
                is_down:    true,
                start_x:    x,
                start_y:    y,
                current_x:  x,
                current_y:  y,
                start_time: time,
            },
            EventType::PointerMove => Self {
                current_x: x,
                current_y: y,
                ..self
            },
            EventType::PointerUp | EventType::PointerCancel => Self::default(),
            _ => self,
        }
    }
}

pub fn detect_gesture(state: &PointerState, current_time: f64) -> Option<Gesture> {
    if !state.is_down { return None; }

    let dx = state.current_x - state.start_x;
    let dy = state.current_y - state.start_y;
    let dt = current_time - state.start_time;
    let distance = (dx * dx + dy * dy).sqrt();

    // long press: 時間長い + 座標ブレ小さい
    if dt > 500.0 && distance < 10.0 {
        return Some(Gesture::LongPress);
    }

    // swipe: 時間短い + 距離大きい
    if dt < 300.0 && distance > 50.0 {
        return Some(if dx.abs() > dy.abs() {
            if dx > 0.0 { Gesture::SwipeRight } else { Gesture::SwipeLeft }
        } else {
            if dy > 0.0 { Gesture::SwipeDown } else { Gesture::SwipeUp }
        });
    }

    // drag: 距離大きい
    if distance > 10.0 {
        return Some(Gesture::Drag);
    }

    None
}

// ============================================================
// dom (rust item <=> element id)
// ============================================================

pub enum Dom {
    ChatForm,
    CharEditForm,
    ChatInput,
    ModalOpen,
    ModalClose,
    DrawerOpen,
    DrawerClose,
    SelectorOverlay,
    RollItem(Roll),
    CharSelectorOverlay,
    CharRollItem(Model),
    SkillSelectorOverlay,
    SkillRollItem(Model),
    DiceInputOverlay,
    DiceUp,
    DiceDown,
    DiceNext,
    CharEditOpen,
    CharEditCancel,
    CharRoll,
    CharEditRoll(Model),
    Other,
}

impl Dom {
    pub fn decode(id: &str) -> Self {
        match id {
            "chat_form"              => Self::ChatForm,
            "char_edit_form"         => Self::CharEditForm,
            "chat_input"             => Self::ChatInput,
            "modal_open"             => Self::ModalOpen,
            "modal_close"            => Self::ModalClose,
            "drawer_open"            => Self::DrawerOpen,
            "drawer_close"           => Self::DrawerClose,
            "selector_overlay"       => Self::SelectorOverlay,
            "char_selector_overlay"  => Self::CharSelectorOverlay,
            "skill_selector_overlay" => Self::SkillSelectorOverlay,
            "dice_input_overlay"     => Self::DiceInputOverlay,
            "dice_up"                => Self::DiceUp,
            "dice_down"              => Self::DiceDown,
            "dice_next"              => Self::DiceNext,
            "char_edit_open"         => Self::CharEditOpen,
            "char_edit_cancel"       => Self::CharEditCancel,
            "char_roll"              => Self::CharRoll,
            _ if id.starts_with("roll_") => {
                let key = id.strip_prefix("roll_").unwrap_or("");
                match Roll::all().iter().find(|r| r.dom_id() == key) {
                    Some(&roll) => Self::RollItem(roll),
                    None        => Self::Other,
                }
            }
            _ if id.starts_with("char_edit_roll_") => {
                let key = id.strip_prefix("char_edit_roll_").unwrap_or("");
                match schema::attribute(schema::Attribute::Characteristic).iter().find(|m| m.dom_id() == key) {
                    Some(&field) => Self::CharEditRoll(field),
                    None         => Self::Other,
                }
            }
            _ if id.starts_with("char_roll_") => {
                let key = id.strip_prefix("char_roll_").unwrap_or("");
                match schema::attribute(schema::Attribute::Characteristic).iter().find(|m| m.dom_id() == key) {
                    Some(&field) => Self::CharRollItem(field),
                    None         => Self::Other,
                }
            }
            _ if id.starts_with("skill_roll_") => {
                let key = id.strip_prefix("skill_roll_").unwrap_or("");
                match schema::attribute(schema::Attribute::Skill).iter().find(|m| m.dom_id() == key) {
                    Some(&field) => Self::SkillRollItem(field),
                    None         => Self::Other,
                }
            }
            _ => Self::Other,
        }
    }
}