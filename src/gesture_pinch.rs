//! 2 本指ジェスチャー（pinch/zoom）の設計スケッチ。**検証・コンパイル未確認。**
//!
//! 「zoom が欲しいだけ」という前提のもと、pinch と rotate を別カテゴリの
//! Recognizer/Engine として作らず、2 本の指の相対運動を 1 本の合成入力に
//! 畳み込んでから `gesture_fixed.rs` の 1 本指パイプライン
//! (`detect_on_move` / `detect_on_release`) にそのまま通す設計を検討する。
//!
//! # 畳み込みの考え方
//!
//! 2 本の指の変位ベクトルを `d1`, `d2` としたとき、
//!
//! - `d1 · d2 < 0`（逆向き、または片方が支配的に距離を変えている）
//!   → pinch。`distance(p1, p2)` の始点との比を `scale` として返すだけで、
//!     角度 (`get-rotation.js` / `touchDistanceAngle` の angle 成分) は
//!     畳み込みの外に置き、実装しない。
//! - それ以外（ほぼ平行）
//!   → 合成点 `mid = (p1 + p2) / 2` を 1 本指の現在座標として扱い、
//!     既存の `detect_on_move` / `detect_on_release` にそのまま渡す。
//!     Drag / Swipe* が「2 本指パン」としてそのまま出る。
//!
//! 新規に増える語彙は `Gesture::Pinch { scale: f64 }` の 1 つだけで済む。
//!
//! # 差し込み場所
//!
//! `pointerId` は `PointerEvent` の標準フィールドであり、ブラウザ側の
//! Pointer Events API として当然渡ってくる値である。今の `CanvasEvent`
//! (`js_client.app.rs:302-317`, フィールドは `event_type` / `id: dom::Id`
//! / `key` / `value` / `x` / `y` / `time`) とフレームフォーマット
//! (`event.app.rs` の `decode_event`) がまだそれを運んでいないだけで、
//! API レベルの制約ではない。よって「`pointer_id` を運ぶ」ことを前提に
//! 進めてよい。
//!
//! 変更は 3 箇所。
//!
//! 1. JS 側の `pointerdown`/`pointermove`/`pointerup` リスナーで
//!    `event.pointerId` をフレームへ追加エンコードする。
//! 2. `decode_event` (`event.app.rs`) で読み、`CanvasEvent` に
//!    `pointer_id: u32` を足す。
//! 3. `PointerState` を 2 本指分に増やす（`pointerId` でキー付けする
//!    汎用配列ではなく、`primary` / `secondary` の固定 2 枠で足りる。
//!    zoom 用途では 3 本目以降を扱う理由がない）。
//!
//! 1 本指ロジック (`detect_on_move` / `detect_on_release`) 自体は
//! 無改造のまま、`fold_two_fingers` が作る合成入力を渡すだけで済む。
//! フレームフォーマットは「protocol の一部」(`event.app.rs` 冒頭コメント)
//! なので JS 側・wasm 側で同時に変える必要がある、という点だけ留意する。
//!
//! 以下は `CanvasEvent` に `pointer_id` が生えた後を前提にした
//! 畳み込みロジックのみを示す。

#![allow(dead_code)]

// ============================================================
// 前提: CanvasEvent / decode_event に pointer_id が
// 追加された後の型。gesture_fixed.rs の型をそのまま使う。
// ============================================================

/// 2 本指の一方の追跡状態。`gesture_fixed.rs::PointerState` の x/y/time 部分
/// だけを抜き出したもの。tap/swipe/drag/long_press の判定はこちらではなく
/// 畳み込み後に `gesture_fixed.rs::PointerState` へ渡すため、ここでは
/// 座標と時刻のみを持つ。
#[derive(Debug, Clone, Copy, Default)]
struct TouchPoint {
    x: f64,
    y: f64,
    /// `PointerDown` 時の座標。distance比の基準点。
    start_x: f64,
    start_y: f64,
    start_time: f64,
}

/// 2 本指の状態。3 本目以降は無視する（zoom 用途では不要と判断）。
///
/// `primary` が埋まっていない状態で `secondary` だけ埋まることはない
/// （1 本目が離れたら 2 本目を `primary` へ繰り上げる）。
#[derive(Debug, Clone, Copy, Default)]
struct TwoFingerState {
    primary: Option<TouchPoint>,
    secondary: Option<TouchPoint>,
}

/// 畳み込み結果。`gesture_fixed.rs::detect_gesture` へ渡す「仮想の 1 点」か、
/// pinch として確定した scale のどちらか。
#[derive(Debug, Clone, Copy, PartialEq)]
enum FoldedInput {
    /// 2 本の指がほぼ平行に動いている。1 本指パイプラインへ渡す合成座標。
    AsSinglePoint { x: f64, y: f64 },
    /// 2 本の指が逆向きに動いている。pinch として確定。
    Pinch { scale: f64 },
    /// 1 本指のみ、または判定材料が揃っていない。
    None,
}

/// pinch と判定する際の、変位ベクトルの内積の閾値。
///
/// 内積が正 (順向き) でも小さければ「ほぼ直交」であり、pinch 側に
/// 倒しても実害が小さい。逆に内積が大きな正の値なら明確な平行移動。
/// この値は Hammer.js / @use-gesture に対応する定数が無く、実装側の
/// 独自導入になる。0.0 (符号だけで判定) から始めて実機で調整する想定。
const PINCH_DOT_THRESHOLD: f64 = 0.0;

/// 2 本指の現在フレームから、畳み込み結果を導出する。
///
/// # 引数
///
/// `state` は直前フレームまでの追跡状態（呼び出し側が `TouchPoint` の
/// `x`/`y` を最新値へ更新済みである前提。`gesture_fixed.rs::PointerState`
/// の `update` に相当する処理は、ここでは呼び出し側の責務とする）。
fn fold_two_fingers(state: &TwoFingerState) -> FoldedInput {
    let (Some(p1), Some(p2)) = (state.primary, state.secondary) else {
        return FoldedInput::None;
    };

    // 各指の始点からの変位ベクトル。
    let d1 = (p1.x - p1.start_x, p1.y - p1.start_y);
    let d2 = (p2.x - p2.start_x, p2.y - p2.start_y);

    let dot = d1.0 * d2.0 + d1.1 * d2.1;

    if dot < PINCH_DOT_THRESHOLD {
        // 逆向き: pinch。scale は「開始時の距離」に対する「現在の距離」の比。
        let start_distance = distance(p1.start_x, p1.start_y, p2.start_x, p2.start_y);
        let current_distance = distance(p1.x, p1.y, p2.x, p2.y);
        if start_distance <= 0.0 {
            return FoldedInput::None;
        }
        return FoldedInput::Pinch { scale: current_distance / start_distance };
    }

    // 順向き: 合成点を 1 本指の現在座標として渡す。
    // gesture_fixed.rs::detect_on_move / detect_on_release へそのまま渡せば
    // Drag / Swipe* が「2 本指パン」として出る。
    FoldedInput::AsSinglePoint { x: (p1.x + p2.x) / 2.0, y: (p1.y + p2.y) / 2.0 }
}

/// 共通演算。`gesture_fixed.rs::PointerState::distance` と同じ式。
#[cfg(feature = "std")]
fn distance(x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
    let dx = x1 - x0;
    let dy = y1 - y0;
    (dx * dx + dy * dy).sqrt()
}

// ============================================================
// Gesture 拡張案
// ============================================================

/// `gesture_fixed.rs::Gesture` に足す想定の 1 variant。
///
/// 既存の 9 variant（Tap / LongPress / Swipe* / Drag / DragEnd /
/// DragCancel）はそのまま、これを 1 つ追加するだけで済む
/// （畳み込みで順向きは既存 variant に吸収されるため）。
#[derive(Debug, Clone, Copy, PartialEq)]
enum PinchGesture {
    /// pinch 継続中。`scale > 1.0` で拡大、`< 1.0` で縮小。
    Pinch { scale: f64 },
    /// pinch 終了 (どちらかの指が離れた)。
    PinchEnd,
}
