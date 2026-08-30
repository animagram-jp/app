// app repository の `src/js_client.rs` に対応する。
//
// このファイルの項目は 2 群に分かれる。
//
// 1. 共有アリーナ経由に伴って新設 / 変更するもの。
//    `Command` / `encode_command` / operation 番号 / `Name` / `name` /
//    `EventType::decode_u8` / `KeyName::decode_u8` /
//    `dom::Tag::encode_u8` / `dom::Tag::decode_u8`。
//
// 2. app repository に既にあり、経路の検討に必要な signature だけを
//    置いたもの。中身は省略してある。取り込み時にはこれらを削除し、
//    既存の定義をそのまま使う。
//    `Device` / `detect_device` / `PointerState` / `Gesture` /
//    `detect_gesture` / `EventType` と `KeyName` の variant /
//    `dom::Tag` / `dom::Segment` / `dom::Id`。
//
// なお `CanvasEvent::decode` は `crate::event::decode_event` に統合した。
// `dom::Id` の直列化は `crate::arena` の `Encoder::id` / `Decoder::id` が担う。

use alloc::{string::String, vec::Vec};
use core::{
    clone::Clone,
    cmp::{Eq, PartialEq},
    default::Default,
    fmt::Debug,
    marker::Copy,
    matches,
    option::Option::{self, None, Some},
    primitive::{bool, f32, f64, i32, u8, u16, u32},
};

use crate::arena::Encoder;

// ============================================================
// send operation
// ============================================================
//
// operation 番号は JavaScript 側 (init.js の execute) の switch 分岐と対応。
// 値を追加/変更する際は両方を揃えて更新する。
//
// app repository の `Serialize for Command` が用いる 1〜16 をそのまま
// 引き継ぎ、17 以降を追加している。

/// 要素の `textContent` を設定する。
pub const OPERATION_SET_TEXT: u8 = 1;
/// 要素の `value` を設定する。
pub const OPERATION_SET_VALUE: u8 = 2;
/// 属性を設定する。
pub const OPERATION_SET_ATTRIBUTE: u8 = 3;
/// 属性を削除する。
pub const OPERATION_REMOVE_ATTRIBUTE: u8 = 4;
/// class を追加する。
pub const OPERATION_ADD_CLASS: u8 = 5;
/// class を削除する。
pub const OPERATION_REMOVE_CLASS: u8 = 6;
/// `style.width` を設定する。
pub const OPERATION_SET_WIDTH: u8 = 7;
/// `style.height` を設定する。
pub const OPERATION_SET_HEIGHT: u8 = 8;
/// `style.zIndex` を設定する。
pub const OPERATION_SET_Z_INDEX: u8 = 9;
/// `style.background` を設定する。
pub const OPERATION_SET_BACKGROUND: u8 = 10;
/// `style.translate` を設定する。
pub const OPERATION_SET_TRANSLATE: u8 = 11;
/// `style.cursor` を設定する。
pub const OPERATION_SET_CURSOR: u8 = 12;
/// `dialog` 要素を `showModal` で開く。
pub const OPERATION_SHOW_MODAL: u8 = 13;
/// `dialog` 要素を `close` で閉じる。
pub const OPERATION_CLOSE_MODAL: u8 = 14;
/// 要素に `focus` する。
pub const OPERATION_FOCUS: u8 = 15;
/// JavaScript 側の名前付き関数を呼ぶ。
pub const OPERATION_JS_FN: u8 = 16;
// 17 と 18 は `FileStoreGet` / `FileStoreSet` が使っていた。
// OPFS は `FileSystemSyncAccessHandle` という同期ハンドルを返し、
// worker の init で取得すれば `run_loop` の中から直接呼べるため、
// 往復にする必要が無い。番号は詰めず空けてある (`init.js` と揃える)。
/// トリプルバッファへ新しいフレームを公開した。
pub const OPERATION_FRAME_READY: u8 = 19;
/// Wasm 側で回復不能な異常が起きたことを報告する。
///
/// JavaScript 側はこれを受けて worker を作り直す。`FileStoreError` が
/// `Handler` に届く回復可能なエラーであるのに対し、こちらは
/// `run_loop` を止めて再起動へ向かう片道の通知である。
pub const OPERATION_ERROR: u8 = 20;

/// `Command::Error` が運ぶ異常の種別。
///
/// JavaScript 側 `init.js` の `ERROR_CODES` 配列と順序を揃える。
/// 詳細は `message` が運ぶ。
///
/// 番号は再起動の要否で 2 群に分かれる。`FATAL_FROM` 以上が
/// 「Wasm 側が停止しており、作り直さないと以降無反応になる」ものである。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ErrorCode(pub u8);

/// これ以上の番号は再起動を要する。
pub const FATAL_FROM: u8 = 128;

/// イベントフレームのデコードに失敗した。1 フレーム捨てれば済む。
pub const ERROR_DECODE: ErrorCode = ErrorCode(1);
/// コマンドリングが満杯でフレームを捨てた。画面が実際の状態からずれる。
pub const ERROR_COMMAND_OVERFLOW: ErrorCode = ErrorCode(2);

/// panic hook が捕らえた panic。thread は停止している。
pub const ERROR_PANIC: ErrorCode = ErrorCode(128);
/// `FileStore` の書き込みが繰り返し失敗した。ハンドルが失効している。
///
/// 再取得には `FileStore::new` の `await` が要り、`run_loop` の中では
/// 待てない。worker を作り直し、新しい `App::init` に開き直させる。
pub const ERROR_STORE_LOST: ErrorCode = ErrorCode(129);

impl ErrorCode {
    /// 再起動を要するなら true。
    ///
    /// ```
    /// # use app::js_client::{ERROR_DECODE, ERROR_PANIC};
    /// assert!(!ERROR_DECODE.is_fatal());
    /// assert!(ERROR_PANIC.is_fatal());
    /// ```
    pub fn is_fatal(self) -> bool {
        self.0 >= FATAL_FROM
    }
}

/// Wasm から JavaScript へ送る指示。
///
/// app repository の `Command` に対応する。相違点は 3 つである。
///
/// 1. `id` を `String` ではなく `dom::Id` で持つ。直列化の際に
///    `Id::encode` を呼ばず、セグメント列をそのままバイト列にする。
/// 2. 属性名、class 名、cursor 値、JavaScript 関数名を `String` ではなく
///    `Name` で持つ。JavaScript 側の静的文字列配列の添字である。
/// 3. `FrameReady` / `Error` を持つ。アリーナ由来の 2 つを
///    同じ enum に統合したためである。
pub enum Command {
    /// `el.textContent = d.string() ?? ""; break;`
    SetText { id: dom::Id, value: String },
    /// `el.value = d.string() ?? ""; break;`
    SetValue { id: dom::Id, value: String },
    /// `el.setAttribute(NAMES[d.u16()], d.string() ?? ""); break;`
    SetAttribute {
        id: dom::Id,
        attribute: Name,
        value: String,
    },
    /// `el.removeAttribute(NAMES[d.u16()]); break;`
    RemoveAttribute { id: dom::Id, attribute: Name },
    /// `el.classList.add(NAMES[d.u16()]); break;`
    AddClass { id: dom::Id, value: Name },
    /// `el.classList.remove(NAMES[d.u16()]); break;`
    RemoveClass { id: dom::Id, value: Name },
    /// `el.style.width = d.u32() + "px"; break;`
    SetWidth { id: dom::Id, px: u32 },
    /// `el.style.height = d.u32() + "px"; break;`
    SetHeight { id: dom::Id, px: u32 },
    /// `el.style.zIndex = d.i32(); break;`
    SetZIndex { id: dom::Id, z: i32 },
    /// `el.style.background = d.string(); break;`
    SetBackground { id: dom::Id, value: String },
    /// `el.style.translate = `${d.f32()}px ${d.f32()}px`; break;`
    SetTranslate { id: dom::Id, x: f32, y: f32 },
    /// `el.style.cursor = NAMES[d.u16()] ?? ""; break;`
    SetCursor { id: dom::Id, value: Name },
    /// `el.showModal(); break;`
    ShowModal { id: dom::Id },
    /// `el.close(); break;`
    CloseModal { id: dom::Id },
    /// `el.focus(); break;`
    Focus { id: dom::Id },
    /// `jsFn[NAMES[d.u16()]]?.(el); break;`
    JsFn { id: dom::Id, name: Name },
    /// トリプルバッファへ新しいフレームを公開した。
    FrameReady,
    /// 回復不能な異常を報告する。JavaScript 側は worker を作り直す。
    Error { code: ErrorCode, message: String },
}

/// コマンド 1 件をバイト列へ追記する。
///
/// フレーム構造は `[operation:u8][payload...]` である。要素を持つ
/// operation は `id` を長さ前置のセグメント列として続ける。
///
/// app repository では `Serialize for Command` が同じ役割を担う。
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
        Command::SetAttribute {
            ref id,
            attribute,
            ref value,
        } => {
            encoder.u8(OPERATION_SET_ATTRIBUTE);
            encoder.id(id);
            encoder.u16(attribute.0);
            encoder.str(value);
        }
        Command::RemoveAttribute { ref id, attribute } => {
            encoder.u8(OPERATION_REMOVE_ATTRIBUTE);
            encoder.id(id);
            encoder.u16(attribute.0);
        }
        Command::AddClass { ref id, value } => {
            encoder.u8(OPERATION_ADD_CLASS);
            encoder.id(id);
            encoder.u16(value.0);
        }
        Command::RemoveClass { ref id, value } => {
            encoder.u8(OPERATION_REMOVE_CLASS);
            encoder.id(id);
            encoder.u16(value.0);
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
            encoder.u16(value.0);
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
            encoder.u16(name.0);
        }
        Command::FrameReady => {
            encoder.u8(OPERATION_FRAME_READY);
        }
        Command::Error { code, ref message } => {
            encoder.u8(OPERATION_ERROR);
            encoder.u8(code.0);
            encoder.str(message);
        }
    }
}

// ============================================================
// receive (canvas event)
// ============================================================

/// DOM 由来のイベントの内容。
///
/// app repository の `CanvasEvent` と同じ構成である。`decode` の入力が
/// `JsValue` から `&[u8]` に変わるだけで、フィールドは変えていない。
/// デコードそのものは `crate::event::decode_event` が担う。
pub struct CanvasEvent {
    /// イベント種別。
    pub event_type: EventType,
    /// 発生元の要素。
    pub id: dom::Id,
    /// 押されたキー。
    pub key: KeyName,
    /// 要素の `value`。
    pub value: String,
    /// `clientX` の値。
    pub x: f64,
    /// `clientY` の値。
    pub y: f64,
    /// `timeStamp` の値。
    pub time: f64,
}

// ============================================================
// name (static string index)
// ============================================================

/// 静的文字列の識別子。JavaScript 側の文字列配列の添字である。
///
/// app repository は属性名や class 名を `String` で持ち、そのまま
/// JavaScript へ渡す。ここでは実行時に生成されない文字列を添字に
/// 置き換え、フレーム長を 2 byte に抑える。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Name(pub u16);

/// 事前に登録した静的文字列の識別子。
///
/// 属性名、class 名、cursor 値、JavaScript 関数名、永続化キーを含む。
/// JavaScript 側 `init.js` の `NAMES` 配列と順序を揃える。
pub mod name {
    use super::Name;

    /// `hidden` 属性。
    pub const HIDDEN: Name = Name(0);
    /// `disabled` 属性。
    pub const DISABLED: Name = Name(1);
    /// `active` class。
    pub const ACTIVE: Name = Name(2);
    /// `grab` cursor。
    pub const CURSOR_GRAB: Name = Name(3);
    /// `default` cursor。
    pub const CURSOR_DEFAULT: Name = Name(4);
    /// `init.js` の `jsFn.show`。
    pub const FN_SHOW: Name = Name(5);
    /// `init.js` の `jsFn.hide`。
    pub const FN_HIDE: Name = Name(6);
}

// ============================================================
// 既存項目 (signature のみ)
// ============================================================
//
// 以下は app repository の `src/js_client.rs` に既にある項目である。
// 経路の検討に必要な signature のみを置き、中身は省略する。
// 取り込み時にはこれらを削除し、既存の定義をそのまま使う。
//
// 中身が変わるのは 3 つだけである。
//
// - `EventType::decode_u8` / `KeyName::decode_u8` / `Tag::encode_u8` /
//   `Tag::decode_u8` を追加する。文字列ではなく番号で受け渡すため。
// - `CanvasEvent::decode` は `crate::event::decode_event` に統合する。
// - `dom::Id` の直列化は `Encoder::id` / `Decoder::id` が担う。

/// 入力装置の種別。中身は app repository の `Device` と同じ。
pub enum Device {
    /// touch 入力。
    Touch,
    /// mouse 入力。
    Mouse,
}

/// `pointer_coarse` から入力装置を判定する。中身は app repository と同じ。
pub fn detect_device(pointer_coarse: bool) -> Device {
    if pointer_coarse {
        Device::Touch
    } else {
        Device::Mouse
    }
}

/// イベント種別。variant は app repository の `EventType` と同じ。
///
/// `decode` (文字列から) はそのまま残し、`decode_u8` を追加する。
/// JavaScript 側が文字列ではなく番号を送るためである。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EventType {
    /// `submit`。
    Submit,
    /// `click`。
    Click,
    /// `contextmenu`。
    ContextMenu,
    /// `keydown`。
    KeyDown,
    /// `input`。
    Input,
    /// `change`。
    Change,
    /// `focusin`。
    FocusIn,
    /// `focusout`。
    FocusOut,
    /// `resize`。
    Resize,
    /// `scroll`。
    Scroll,
    /// `drop`。
    Drop,
    /// `pointerdown`。
    PointerDown,
    /// `pointerup`。
    PointerUp,
    /// `pointermove`。
    PointerMove,
    /// `pointercancel`。
    PointerCancel,
    /// 上記以外。
    Other,
}

impl EventType {
    /// 番号からイベント種別を得る。未知の番号は `Other` とする。
    ///
    /// 番号は app repository の `EventType::decode` の分岐順に対応する。
    /// `init.js` の `EVENT_TYPES` 配列と順序を揃える。
    ///
    /// ```
    /// # use app::js_client::EventType;
    /// assert_eq!(EventType::decode_u8(12), EventType::PointerDown);
    /// assert_eq!(EventType::decode_u8(200), EventType::Other);
    /// ```
    pub fn decode_u8(value: u8) -> Self {
        match value {
            1 => Self::Submit,
            2 => Self::Click,
            3 => Self::ContextMenu,
            4 => Self::KeyDown,
            5 => Self::Input,
            6 => Self::Change,
            7 => Self::FocusIn,
            8 => Self::FocusOut,
            9 => Self::Resize,
            10 => Self::Scroll,
            11 => Self::Drop,
            12 => Self::PointerDown,
            13 => Self::PointerUp,
            14 => Self::PointerMove,
            15 => Self::PointerCancel,
            _ => Self::Other,
        }
    }

    // pub fn decode(event_type: &str) -> Self { .. }
    // 中身は app repository と同じ。
}

/// キー名。variant は app repository の `KeyName` と同じ。
///
/// `decode` (文字列から) はそのまま残し、`decode_u8` を追加する。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyName {
    /// `ArrowUp`。
    ArrowUp,
    /// `ArrowDown`。
    ArrowDown,
    /// `ArrowLeft`。
    ArrowLeft,
    /// `ArrowRight`。
    ArrowRight,
    /// `Enter`。
    Enter,
    /// `Escape`。
    Escape,
    /// `Tab`。
    Tab,
    /// `Backspace`。
    Backspace,
    /// 上記以外。
    Other,
}

impl KeyName {
    /// 番号からキー名を得る。未知の番号は `Other` とする。
    ///
    /// 番号は app repository の `KeyName::decode` の分岐順に対応する。
    ///
    /// ```
    /// # use app::js_client::KeyName;
    /// assert_eq!(KeyName::decode_u8(5), KeyName::Enter);
    /// assert_eq!(KeyName::decode_u8(200), KeyName::Other);
    /// ```
    pub fn decode_u8(value: u8) -> Self {
        match value {
            1 => Self::ArrowUp,
            2 => Self::ArrowDown,
            3 => Self::ArrowLeft,
            4 => Self::ArrowRight,
            5 => Self::Enter,
            6 => Self::Escape,
            7 => Self::Tab,
            8 => Self::Backspace,
            _ => Self::Other,
        }
    }

    // pub fn decode(key_name: &str) -> Self { .. }
    // 中身は app repository と同じ。
}

// ============================================================
// gesture: long press, swipe (up,down,left,right), drag
// See ./docs/Gesture.md
// ============================================================

// pointerdown:   is_down = true, 座標・時刻記録, タイマー起動
// pointermove:   座標がブレていたら長押しキャンセル (指がズレた)
// pointerup:     経過時間で click か 長押し か判定
// pointercancel: 全部リセット (割り込まれた時)

/// pointer の追跡状態。中身は app repository の `PointerState` と同じ。
#[derive(Default, Clone, Copy)]
pub struct PointerState {
    is_down: bool,           // default: false
    start_x: f64,            // default: 0.0
    start_y: f64,            // default: 0.0
    current_x: f64,          // default: 0.0
    current_y: f64,          // default: 0.0
    start_time: f64,         // default: 0.0
    drag_offset: (f64, f64), // (pointer_px - target base px) when PointerDown
    drag_px: (f64, f64),     // target base px when is_dragging == true
    is_dragging: bool,
}

impl PointerState {
    /// pointer 状態を 1 イベント分進める。中身は app repository と同じ。
    pub fn update(self, event_type: &EventType, x: f64, y: f64, time: f64) -> Self {
        match event_type {
            EventType::PointerDown => Self {
                is_down: true,
                start_x: x,
                start_y: y,
                current_x: x,
                current_y: y,
                start_time: time,
                drag_offset: (0.0, 0.0),
                drag_px: (0.0, 0.0),
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

/// 認識したジェスチャ。variant は app repository の `Gesture` と同じ。
pub enum Gesture {
    /// 長押し。
    LongPress,
    /// 上方向のスワイプ。
    SwipeUp,
    /// 下方向のスワイプ。
    SwipeDown,
    /// 左方向のスワイプ。
    SwipeLeft,
    /// 右方向のスワイプ。
    SwipeRight,
    /// ドラッグ中。
    Drag { x: f64, y: f64 },
    /// ドラッグ終了。
    DragEnd,
}

/// pointer 状態の遷移からジェスチャを認識する。中身は app repository と同じ。
///
/// `(dx * dx + dy * dy).sqrt()` は `f64::sqrt` が `std` の inherent method で
/// core に無いため、`libm::sqrt` に替えてある。値は同じである。
pub fn detect_gesture(
    state: &mut PointerState,
    prev_state: &PointerState,
    event_type: &EventType,
    current_time: f64,
) -> Option<Gesture> {
    if !state.is_down {
        if prev_state.is_dragging {
            return Some(Gesture::DragEnd);
        }
        return None;
    }

    let dx = state.current_x - state.start_x;
    let dy = state.current_y - state.start_y;
    let dt = current_time - state.start_time;
    let distance = libm::sqrt(dx * dx + dy * dy);

    // long press: long time + short distance
    if dt > 251.0 && distance < 9.0 {
        return Some(Gesture::LongPress);
    }

    // swipe: when PointerUp + velocity > 0.5 px/ms + duration < 250ms
    if matches!(event_type, EventType::PointerUp) && dt > 0.0 {
        let velocity = distance / dt;
        if velocity > 0.5 && distance > 50.0 && dt < 250.0 {
            return Some(if libm::fabs(dx) > libm::fabs(dy) {
                if dx > 0.0 {
                    Gesture::SwipeRight
                } else {
                    Gesture::SwipeLeft
                }
            } else if dy > 0.0 {
                Gesture::SwipeDown
            } else {
                Gesture::SwipeUp
            });
        }
    }

    // drag: when PointerMove + long distance → return offset
    if matches!(event_type, EventType::PointerMove) && distance > 10.0 {
        state.is_dragging = true;
        return Some(Gesture::Drag {
            x: state.current_x,
            y: state.current_y,
        });
    }

    None
}

// ============================================================
// dom (rust item <=> element id)
// ============================================================

/// Rust item と element id の対応。中身は app repository の `dom` と同じ。
///
/// `Tag` に `encode_u8` / `decode_u8` を追加する点のみ異なる。
/// `Id::encode` / `Id::decode` (文字列) はそのまま残すが、共有アリーナ経由の
/// 直列化には `Encoder::id` / `Decoder::id` を用いる。
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

    /// element の tag。variant は app repository の `Tag` と同じ。
    #[derive(Debug, Clone, PartialEq)]
    pub enum Tag {
        /// `body`。
        Body,
        /// `main`。
        Main,
        /// `dialog id="*modal*"`。
        Modal,
        /// `header`。
        Header,
        /// `section`。
        Section,
        /// `button`。
        Button,
        /// 上記以外。
        Other,
        // 残りの variant は app repository と同じ。
    }

    impl Tag {
        /// tag を番号に直す。`init.js` の `TAGS` 配列の添字である。
        ///
        /// ```
        /// # use app::js_client::dom::Tag;
        /// assert_eq!(Tag::Body.encode_u8(), 1);
        /// assert_eq!(Tag::Other.encode_u8(), 0);
        /// ```
        pub fn encode_u8(&self) -> u8 {
            match self {
                Self::Body => 1,
                Self::Main => 2,
                Self::Modal => 3,
                Self::Header => 4,
                Self::Section => 5,
                Self::Button => 6,
                Self::Other => 0,
            }
        }

        /// 番号から tag を得る。未知の番号は `Other` とする。
        ///
        /// ```
        /// # use app::js_client::dom::Tag;
        /// assert_eq!(Tag::decode_u8(1), Tag::Body);
        /// assert_eq!(Tag::decode_u8(200), Tag::Other);
        /// ```
        pub fn decode_u8(value: u8) -> Self {
            match value {
                1 => Self::Body,
                2 => Self::Main,
                3 => Self::Modal,
                4 => Self::Header,
                5 => Self::Section,
                6 => Self::Button,
                _ => Self::Other,
            }
        }

        // pub fn decode(s: &str) -> Self { .. }
        // pub fn encode(&self) -> &'static str { .. }
        // 中身は app repository と同じ。
    }

    /// id の 1 セグメント。中身は app repository の `Segment` と同じ。
    #[derive(Debug, Clone, PartialEq)]
    pub struct Segment {
        /// セグメントの tag。
        pub tag: Tag,
        /// 同一 tag 内の連番。1 つだけなら None。
        pub n: Option<u32>,
    }

    impl Segment {
        // pub fn new(tag: Tag) -> Self { .. }
        // pub fn numbered(tag: Tag, n: u32) -> Self { .. }
        // pub fn decode(s: &str) -> Self { .. }
        // pub fn encode(&self) -> String { .. }
        // 中身は app repository と同じ。
    }

    /// element id。中身は app repository の `Id` と同じ。
    #[derive(Debug, Clone, PartialEq)]
    pub struct Id(pub Vec<Segment>);

    impl Id {
        /// tag と連番の列から id を作る。中身は app repository と同じ。
        pub fn new(segs: &[(Tag, Option<u32>)]) -> Self {
            Self(
                segs.iter()
                    .map(|(tag, n)| Segment {
                        tag: tag.clone(),
                        n: *n,
                    })
                    .collect(),
            )
        }

        // pub fn decode(id: &str) -> Self { .. }
        // pub fn encode(&self) -> String { .. }
        // pub fn last_tag(&self) -> Option<&Tag> { .. }
        // 中身は app repository と同じ。
    }
}
