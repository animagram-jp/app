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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
// gesture: tap, long press, swipe (up,down,left,right), drag
// See ./docs/Gesture.md
// ============================================================
//
// 元実装 (旧 `detect_gesture`) は 3 点の欠陥を持っていた。
//
// 1. `update` の `PointerUp` 分岐が `..Self::default()` で `is_down` と
//    `start_x/y` / `start_time` を同時にゼロクリアしていたため、直後の
//    `detect_gesture` 冒頭の `if !state.is_down { .. return None; }` で
//    必ず抜け、`Swipe*` 系の分岐に制御が到達しなかった。
// 2. `PointerMove` / `PointerUp` でしか判定しないため、押したまま指を
//    動かさない長押しが発火しなかった (タイマー未実装)。
// 3. `Drag` の閾値 (`distance > 10.0`) が `Swipe` の閾値 (`> 50.0`) より
//    先に成立するため、素早いフリックが `PointerMove` の時点で `Drag`
//    として確定してしまい、swipe に到達しなかった。
//
// 以下はこれらを修正した版である。`update` は `PointerUp` /
// `PointerCancel` でも座標・時刻・`drag_offset` / `drag_px` を保持し、
// `is_down` と `is_dragging` のフラグだけを倒す。`detect_gesture` は
// 終了イベントと移動イベントで判定を分け、`Drag` は「速度が swipe 閾値
// 未満」または「既に drag 中」のときだけ発火させる。長押しはタイマーを
// 増設せず、`PointerMove` / `PointerUp` の中で経過時間を見て判定し、
// 一度発火したら `long_press_fired` でラッチして連続発火を防ぐ。
// `PointerCancel` は `DragEnd` ではなく `DragCancel` を返し、正常終了と
// 区別する。

/// ジェスチャ判定の閾値。すべて CSS px と ms。
///
/// 装置ごとに閾値を分ける。指の接触面はマウスカーソルより広く、押下中の
/// 座標のブレも大きいため、タッチでは許容を広げる。
#[derive(Debug, Clone, Copy)]
pub struct Thresholds {
    /// 長押しと見なす最短時間 (ms)。
    pub long_press_ms: f64,
    /// 長押し中に許容する座標のブレ (px)。これを超えたら長押しを取り消す。
    pub long_press_slop_px: f64,
    /// ドラッグ開始と見なす移動距離 (px)。
    pub drag_start_px: f64,
    /// スワイプと見なす最短距離 (px)。
    pub swipe_min_px: f64,
    /// スワイプと見なす最低速度 (px/ms)。
    pub swipe_min_velocity: f64,
    /// スワイプと見なす最長時間 (ms)。これを超えたらドラッグ扱い。
    pub swipe_max_ms: f64,
    /// タップと見なす最長時間 (ms)。
    pub tap_max_ms: f64,
    /// タップ中に許容する座標のブレ (px)。
    pub tap_slop_px: f64,
}

impl Thresholds {
    /// マウス向けの既定値。元実装の数値をそのまま引き継いでいる。
    pub const MOUSE: Self = Self {
        long_press_ms: 251.0,
        long_press_slop_px: 9.0,
        drag_start_px: 10.0,
        swipe_min_px: 50.0,
        swipe_min_velocity: 0.5,
        swipe_max_ms: 250.0,
        tap_max_ms: 250.0,
        tap_slop_px: 9.0,
    };

    /// タッチ向けの既定値。ブレ許容と開始距離をマウスより広く取る。
    pub const TOUCH: Self = Self {
        long_press_ms: 500.0,
        long_press_slop_px: 16.0,
        drag_start_px: 16.0,
        swipe_min_px: 50.0,
        swipe_min_velocity: 0.5,
        swipe_max_ms: 300.0,
        tap_max_ms: 300.0,
        tap_slop_px: 16.0,
    };

    /// 装置に応じた既定値を返す。
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

/// pointer の追跡状態。中身は app repository の `PointerState` と同じ。
///
/// `PointerUp` / `PointerCancel` でも座標・時刻・`drag_offset` / `drag_px`
/// を保持する。消すのは `is_down` と `is_dragging` のフラグだけである。
/// 判定はこの直後に `detect_gesture` が行うため、そこで必要な値を
/// 判定前に消さない。
#[derive(Debug, Default, Clone, Copy)]
pub struct PointerState {
    is_down: bool,
    start_x: f64,
    start_y: f64,
    current_x: f64,
    current_y: f64,
    start_time: f64,
    /// `PointerDown` 時の (pointer_px - 対象の左上 px)。
    drag_offset: (f64, f64),
    /// ドラッグ中の対象左上 px (一時値)。
    drag_px: (f64, f64),
    is_dragging: bool,
    /// 長押しを発火済みか。連続発火を防ぐラッチ。
    long_press_fired: bool,
    /// 直前の終了が `PointerCancel` だったか。
    cancelled: bool,
}

impl PointerState {
    /// pointer 状態を 1 イベント分進める。
    ///
    /// 元実装と異なり、`PointerUp` / `PointerCancel` でも座標・時刻を残す。
    #[must_use]
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
                long_press_fired: false,
                cancelled: false,
            },
            EventType::PointerMove => Self {
                current_x: x,
                current_y: y,
                ..self
            },
            EventType::PointerUp => Self {
                is_down: false,
                current_x: x,
                current_y: y,
                cancelled: false,
                ..self
            },
            EventType::PointerCancel => Self {
                is_down: false,
                current_x: x,
                current_y: y,
                cancelled: true,
                ..self
            },
            _ => self,
        }
    }

    /// 押下開始からの移動距離 (px)。
    fn distance(&self) -> f64 {
        let dx = self.current_x - self.start_x;
        let dy = self.current_y - self.start_y;
        libm::sqrt(dx * dx + dy * dy)
    }
}

/// 認識したジェスチャ。variant は app repository の `Gesture` に `Tap` と
/// `DragCancel` を追加したもの。
///
/// `Tap` が無いと、押して離しただけの操作が `None` に落ちて呼び出し側で
/// 区別できない。`DragCancel` は `PointerCancel` による中断で、`DragEnd`
/// と同一視すると割り込み時にドロップを取り消せない。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gesture {
    /// 単純なタップ / クリック。
    Tap,
    /// 長押し。押下したまま `long_press_ms` を超えた時点で 1 度だけ発火する。
    /// 動かさずに保持した場合、実際の発火は次の `PointerMove` /
    /// `PointerUp` まで遅延する ([`detect_gesture`] の doc を参照)。
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
    /// ドラッグ終了 (`pointerup`)。スナップ処理はここで行う。
    DragEnd,
    /// ドラッグ中断 (`pointercancel`)。ドロップを取り消す。
    DragCancel,
}

/// pointer 状態の遷移からジェスチャを認識する。
///
/// `is_down == false` でも、`PointerUp` / `PointerCancel` なら終了時
/// ジェスチャ (`DragEnd` / `Swipe*` / `Tap` / `LongPress`) の判定へ進む。
///
/// # 判定順
///
/// 1. 終了イベント (`PointerUp` / `PointerCancel`)
///    - ドラッグ中なら `DragEnd` / `DragCancel`
///    - 速い + 遠い + 短い なら `Swipe*`
///    - 長押し発火済みなら何も返さない (発火済みのため)
///    - 保持時間超過 + ブレ小 なら `LongPress`（動かないまま離した場合）
///    - 短い + ブレ小 なら `Tap`
/// 2. 移動イベント (`PointerMove`)
///    - 保持時間超過 + ブレ小 かつ未発火なら `LongPress`
///      （動かないまま保持時間を超え、その後わずかに動いた場合）
///    - 既にドラッグ中、または swipe 条件を満たさない移動なら `Drag`
///
/// `LongPress` はタイマーを持たない。動かないまま保持され続けた場合は
/// 次の `PointerMove` / `PointerUp` まで発火が遅延する。
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

/// 終了イベントの判定。
fn detect_on_release(
    state: &mut PointerState,
    prev_state: &PointerState,
    current_time: f64,
    thresholds: &Thresholds,
) -> Option<Gesture> {
    // ドラッグしていたなら、終了種別を返して確定させる。
    if prev_state.is_dragging {
        state.is_dragging = false;
        return Some(if state.cancelled {
            Gesture::DragCancel
        } else {
            Gesture::DragEnd
        });
    }

    // キャンセルはここで打ち切る。タップにもスワイプにもしない。
    if state.cancelled {
        return None;
    }

    let dt = current_time - state.start_time;
    if dt <= 0.0 {
        return None;
    }
    let distance = state.distance();

    // swipe: 速い + 遠い + 短い。
    let velocity = distance / dt;
    if velocity > thresholds.swipe_min_velocity
        && distance > thresholds.swipe_min_px
        && dt < thresholds.swipe_max_ms
    {
        let dx = state.current_x - state.start_x;
        let dy = state.current_y - state.start_y;
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

    // 長押しは `detect_on_move` で既に発火済み。ここで tap を重ねて返さない。
    if state.long_press_fired {
        return None;
    }

    // long press: 指を動かさないまま保持時間を超えて離した場合、
    // `PointerMove` が一度も来ていないため `detect_on_move` 側では
    // 拾えていない。ここが最後の判定機会になる。
    if dt > thresholds.long_press_ms && distance < thresholds.long_press_slop_px {
        return Some(Gesture::LongPress);
    }

    // tap: 短い + ブレ小。
    if dt < thresholds.tap_max_ms && distance < thresholds.tap_slop_px {
        return Some(Gesture::Tap);
    }

    None
}

/// 移動イベントの判定。
fn detect_on_move(
    state: &mut PointerState,
    current_time: f64,
    thresholds: &Thresholds,
) -> Option<Gesture> {
    if !state.is_down {
        return None;
    }

    let distance = state.distance();

    // long press: 動いていない状態で保持時間を超えたら、この `PointerMove`
    // で確定させる。指を完全に静止させたままなら次の `PointerUp` で
    // `detect_on_release` が拾う。
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

    // 既にドラッグ中なら継続する。
    if state.is_dragging {
        return Some(Gesture::Drag {
            x: state.current_x,
            y: state.current_y,
        });
    }

    // まだドラッグに入っていない場合、swipe になりうる動きは譲る。
    let dt = current_time - state.start_time;
    if dt > 0.0 && dt < thresholds.swipe_max_ms {
        let velocity = distance / dt;
        if velocity > thresholds.swipe_min_velocity && distance > thresholds.swipe_min_px {
            // まだ確定させない。PointerUp で swipe か drag かを決める。
            return None;
        }
    }

    state.is_dragging = true;
    Some(Gesture::Drag {
        x: state.current_x,
        y: state.current_y,
    })
}

#[cfg(test)]
mod gesture_tests {
    use super::*;
    use alloc::vec::Vec;

    /// `app.rs` と同じ順序 (update → detect_gesture) でイベント列を流す。
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

    // --- swipe: 元実装では到達不能だった経路 ---

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
        assert_eq!(
            got,
            [Gesture::Drag { x: 150.0, y: 100.0 }, Gesture::DragCancel]
        );
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

    // --- long press: 元実装では発火しなかった経路 ---

    #[test]
    fn long_press_fires_on_release() {
        let th = Thresholds::MOUSE;
        let got = run(
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
        assert_eq!(
            got,
            [
                Gesture::Drag { x: 40.0, y: 10.0 },
                Gesture::Drag { x: 40.0, y: 10.0 }
            ]
        );
    }

    // --- 装置別閾値 ---

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
        let events = [
            (EventType::PointerDown, 10.0, 10.0, 0.0),
            (EventType::PointerUp, 10.0, 10.0, 300.0),
        ];
        assert_eq!(run(&events, &Thresholds::MOUSE), [Gesture::LongPress]);
        assert_eq!(run(&events, &Thresholds::TOUCH), []);
    }
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
