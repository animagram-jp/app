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
    SetHtml,
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
            Self::SetHtml     => 10,
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
// device context
// ============================================================

pub enum Device {
    Mobile,
    Tablet,
    Desktop,
}

// screen_width: screen.width (px)
// pointer_coarse: window.matchMedia('(pointer: coarse)').matches
pub fn detect_device(screen_width: u32, pointer_coarse: bool) -> Device {
    match (pointer_coarse, screen_width) {
        (true, w) if w < 768  => Device::Mobile,
        (true, _)             => Device::Tablet,
        _                     => Device::Desktop,
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
//
// id規則:
//   "_" = 親子セグメント区切り  例: main_div_section-1
//   "-N" = 同タグ内の連番       例: span-3, th-2
//   連番なし = その階層に1つだけ 例: thead_tr, legend_h5
//
// dom::Id::encode()  -> "seg1_seg2_seg-N_..."
// dom::Id::decode()  -> Vec<dom::Segment> のパース

pub mod dom {
    #[derive(Debug, Clone, PartialEq)]
    pub enum Tag {
        Head,
        Main,
        Drawer,   // <dialog id="drawer">
        Modal,    // <dialog id="modal">
        Form,
        Header,
        Div,
        Fieldset,
        Footer,
        Section,
        Span,
        Ol, Ul, Li,
        Textarea,
        Button,
        Input,
        Select,
        H4, H5,
        Legend,
        P,
        Table, Thead, Tbody, Tr, Th, Td,
        Other,
    }

    impl Tag {
        pub fn decode(s: &str) -> Self {
            match s {
                "head"     => Self::Head,
                "main"     => Self::Main,
                "drawer"   => Self::Drawer,
                "modal"    => Self::Modal,
                "form"     => Self::Form,
                "header"   => Self::Header,
                "div"      => Self::Div,
                "fieldset" => Self::Fieldset,
                "footer"   => Self::Footer,
                "section"  => Self::Section,
                "span"     => Self::Span,
                "ol"       => Self::Ol,
                "ul"       => Self::Ul,
                "li"       => Self::Li,
                "textarea" => Self::Textarea,
                "button"   => Self::Button,
                "input"    => Self::Input,
                "select"   => Self::Select,
                "h4"       => Self::H4,
                "h5"       => Self::H5,
                "legend"   => Self::Legend,
                "p"        => Self::P,
                "table"    => Self::Table,
                "thead"    => Self::Thead,
                "tbody"    => Self::Tbody,
                "tr"       => Self::Tr,
                "th"       => Self::Th,
                "td"       => Self::Td,
                _          => Self::Other,
            }
        }

        pub fn encode(&self) -> &'static str {
            match self {
                Self::Head     => "head",
                Self::Main     => "main",
                Self::Drawer   => "drawer",
                Self::Modal    => "modal",
                Self::Form     => "form",
                Self::Header   => "header",
                Self::Div      => "div",
                Self::Fieldset => "fieldset",
                Self::Footer   => "footer",
                Self::Section  => "section",
                Self::Span     => "span",
                Self::Ol       => "ol",
                Self::Ul       => "ul",
                Self::Li       => "li",
                Self::Textarea => "textarea",
                Self::Button   => "button",
                Self::Input    => "input",
                Self::Select   => "select",
                Self::H4       => "h4",
                Self::H5       => "h5",
                Self::Legend   => "legend",
                Self::P        => "p",
                Self::Table    => "table",
                Self::Thead    => "thead",
                Self::Tbody    => "tbody",
                Self::Tr       => "tr",
                Self::Th       => "th",
                Self::Td       => "td",
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