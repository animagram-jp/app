//! ジェスチャ判定の修正版。
//!
//! 元実装は `js_client.rs` の `PointerState` / `detect_gesture`
//! (`~/w/app/src/js_client.rs`, `rectgrid/examples/src/js_client.rs`)。
//! 両者は `detect_gesture` が完全に同一で、`PointerState::update` の
//! `PointerUp` 分岐だけが異なる。どちらも下記 3 点の欠陥を共有する。
//!
//! # 元実装の欠陥
//!
//! ## 1. `Swipe*` 4 variant が到達不能
//!
//! 呼び出し側 (`app.rs`) は `update` → `detect_gesture` の順で呼ぶ。
//! `update` の `PointerUp` 分岐が `is_down` を `false` に、`start_x/y` と
//! `start_time` を `0.0` にするため、`detect_gesture` 冒頭の
//! `if !state.is_down { .. return None; }` で必ず抜ける。
//! `matches!(event_type, EventType::PointerUp)` を条件に持つ swipe
//! ブロックには制御が到達しない。
//!
//! ## 2. `LongPress` が発火しない
//!
//! `detect_gesture` は `PointerMove` / `PointerUp` でしか呼ばれない。
//! 押したまま指を動かさない場合、次にどちらかのイベントが届くまで
//! `dt > 251.0` を満たす時刻を誰も判定しない。長押しの最も基本的な
//! 使い方が発火しない。タイマー起動を前提としたコメントが元実装に
//! 残っているが、タイマーは実装されていない。
//!
//! ## 3. `Swipe` が `Drag` に食われる
//!
//! 1 を直しても、`Drag` の閾値 `distance > 10.0` が `Swipe` の
//! `distance > 50.0` より先に成立する。素早く 50px 以上動かすと
//! `PointerMove` の時点で `Drag` が確定し、`is_dragging` が立つため
//! `PointerUp` では `DragEnd` になる。
//!
//! # 修正方針
//!
//! - `update` は `PointerUp` / `PointerCancel` で座標と時刻を保持し、
//!   `is_down` フラグだけを倒す。判定に必要な情報を判定前に消さない。
//! - `detect_gesture` は `is_down == false` でも、`PointerUp` なら
//!   終了時ジェスチャ (`DragEnd` / `Swipe*` / `Tap` / `LongPress`) の
//!   判定へ進む。
//! - `Drag` は「速度が swipe 閾値未満」または「既に drag 中」のときだけ
//!   発火させ、素早いフリックを swipe に譲る。
//! - `LongPress` はタイマーを増設せず、既存の `PointerMove` /
//!   `PointerUp` の中で判定する。動かないまま保持時間を超えた場合、
//!   実際の発火は次にどちらかのイベントが届いた瞬間までずれる
//!   （`detect_gesture` は `Event` 1 個から `Gesture` を導出する
//!   関数のままで、時刻だけを外から供給される第二の入口を持たない）。
//!   一度発火したら `long_press_fired` でラッチし、連続発火を防ぐ。
//! - `PointerCancel` は `DragEnd` ではなく `DragCancel` を返し、正常終了と
//!   区別できるようにした。元実装は両者を同一視しており、割り込み時に
//!   ドロップを取り消せない。
//! - 閾値を `Thresholds` に括り出し、`Device` ごとに変えられるようにした。
//!   元実装はタッチとマウスで同じ px 値を使っている。指の接触面は数 mm
//!   あるため、タッチで `9.0px` のブレ許容は小さすぎる。

#![allow(dead_code)]

// ============================================================
// 元実装から持ち込む型 (実際の統合時は js_client.rs のものを使う)
// ============================================================

/// ポインタ入力を運ぶイベント種別。元実装の `EventType` のうち
/// ジェスチャ判定が参照する 4 variant のみ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    /// `pointerdown`。
    PointerDown,
    /// `pointerup`。
    PointerUp,
    /// `pointermove`。
    PointerMove,
    /// `pointercancel`。
    PointerCancel,
}

/// 入力装置。`window.matchMedia('(pointer: coarse)')` で判定する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Device {
    /// マウス・トラックパッド。
    Mouse,
    /// タッチ。
    Touch,
}

/// `pointer_coarse` から入力装置を判定する。
#[must_use]
pub fn detect_device(pointer_coarse: bool) -> Device {
    if pointer_coarse { Device::Touch } else { Device::Mouse }
}

// ============================================================
// 閾値
// ============================================================

/// ジェスチャ判定の閾値。すべて CSS px と ms。
///
/// 元実装は `detect_device` で装置を判別しながら閾値を共通にしていた。
/// 指の接触面はマウスカーソルより広く、押下中の座標のブレも大きいため、
/// タッチでは許容を広げる。
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

// ============================================================
// PointerState
// ============================================================

/// pointer の追跡状態。
///
/// 元実装との差分は 2 点。
///
/// - `PointerUp` / `PointerCancel` で `start_*` と `current_*`、`start_time`
///   を保持する。元実装はここでゼロクリアしており、判定に必要な情報が
///   判定前に消えていた。
/// - `long_press_fired` と `cancelled` を追加した。前者は長押しの連続発火を
///   防ぐラッチ、後者は `PointerCancel` と `PointerUp` の区別に使う。
///
/// `drag_offset` / `drag_px` は `Handler` 側が書き込む
/// (`rectgrid/examples/src/event.rs` の `corner_test` 経路)。`DragEnd` 後の
/// `snap_region_to_unit` / `snap_point_to_unit` が読むため、`PointerUp` で
/// 消してはならない。`~/w/app` 版は `..Self::default()` によりここを
/// 消しており、スナップ位置がずれる。
#[derive(Debug, Default, Clone, Copy)]
pub struct PointerState {
    is_down: bool,
    start_x: f64,
    start_y: f64,
    current_x: f64,
    current_y: f64,
    start_time: f64,
    /// `PointerDown` 時の (pointer_px - 対象の左上 px)。`Handler` が書く。
    pub drag_offset: (f64, f64),
    /// ドラッグ中の対象左上 px (一時値)。`Handler` が書く。
    pub drag_px: (f64, f64),
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
    /// 消すのは `is_down` と `is_dragging` のフラグだけである。判定は
    /// この直後に `detect_gesture` が行うため、そこで必要な値を保持する。
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
            EventType::PointerMove => Self { current_x: x, current_y: y, ..self },
            EventType::PointerUp => {
                Self { is_down: false, current_x: x, current_y: y, cancelled: false, ..self }
            }
            EventType::PointerCancel => {
                Self { is_down: false, current_x: x, current_y: y, cancelled: true, ..self }
            }
        }
    }

    /// ポインタが押下中か。
    #[must_use]
    pub const fn is_down(&self) -> bool {
        self.is_down
    }

    /// ドラッグ中か。
    #[must_use]
    pub const fn is_dragging(&self) -> bool {
        self.is_dragging
    }

    /// 押下開始座標。
    #[must_use]
    pub const fn start(&self) -> (f64, f64) {
        (self.start_x, self.start_y)
    }

    /// 現在座標。
    #[must_use]
    pub const fn current(&self) -> (f64, f64) {
        (self.current_x, self.current_y)
    }

    /// 押下開始時刻。
    #[must_use]
    pub const fn start_time(&self) -> f64 {
        self.start_time
    }

    /// 押下開始からの移動距離 (px)。
    #[must_use]
    fn distance(&self) -> f64 {
        let dx = self.current_x - self.start_x;
        let dy = self.current_y - self.start_y;
        sqrt(dx * dx + dy * dy)
    }
}

// ============================================================
// Gesture
// ============================================================

/// 認識したジェスチャ。
///
/// 元実装に `Tap` と `DragCancel` を追加した。
///
/// `Tap` が無いと、押して離しただけの操作が `None` に落ちて呼び出し側で
/// 区別できない。元実装は `detect_gesture` が `None` を返したとき
/// `PointerUp` を捨てているため、クリック相当の操作を拾う経路が無い。
///
/// `DragCancel` は `PointerCancel` による中断。元実装は `PointerUp` と
/// 同じ `DragEnd` を返しており、割り込みでドロップを取り消せない。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gesture {
    /// 単純なタップ / クリック。
    Tap,
    /// 長押し。押下したまま `long_press_ms` を超えた時点で 1 度だけ発火する。
    /// 動かさずに保持した場合、実際の発火は次の `PointerMove` /
    /// `PointerUp` まで遅延する（[`detect_gesture`] の doc を参照）。
    LongPress,
    /// 上方向のスワイプ。
    SwipeUp,
    /// 下方向のスワイプ。
    SwipeDown,
    /// 左方向のスワイプ。
    SwipeLeft,
    /// 右方向のスワイプ。
    SwipeRight,
    /// ドラッグ中。`x` / `y` は現在のポインタ座標。
    Drag { x: f64, y: f64 },
    /// ドラッグ終了 (`pointerup`)。スナップ処理はここで行う。
    DragEnd,
    /// ドラッグ中断 (`pointercancel`)。ドロップを取り消す。
    DragCancel,
}

// ============================================================
// detect
// ============================================================

/// pointer 状態の遷移からジェスチャを認識する。
///
/// `app.rs` の呼び出し順 (`update` → `detect_gesture`) を維持したまま
/// 動くよう、`is_down == false` でも `PointerUp` / `PointerCancel` なら
/// 終了時判定へ進む。元実装はここで無条件に `return` していたため
/// swipe 判定に到達しなかった。
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
/// `LongPress` はタイマーを持たない。`Event` 1 個から `Gesture` を
/// 導出する関数の外に第二の入口（フレームループ等）を作らないため、
/// 動かないまま保持され続けた場合は次の `PointerMove` / `PointerUp`
/// まで発火が遅延する。
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
        EventType::PointerDown => None,
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
    // 元実装は `prev_state.is_dragging` だけを見て `DragEnd` を返し、
    // キャンセルと正常終了を区別していなかった。
    if prev_state.is_dragging {
        state.is_dragging = false;
        return Some(if state.cancelled { Gesture::DragCancel } else { Gesture::DragEnd });
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
        return Some(if fabs(dx) > fabs(dy) {
            if dx > 0.0 { Gesture::SwipeRight } else { Gesture::SwipeLeft }
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
    // で確定させる。イベント駆動のため、実際の発火は「保持時間を超えた
    // 後、指がわずかでも動いた次のイベント」までずれる。指を完全に
    // 静止させたままなら次の `PointerUp` で `detect_on_release` が拾う。
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
        return Some(Gesture::Drag { x: state.current_x, y: state.current_y });
    }

    // まだドラッグに入っていない場合、swipe になりうる動きは譲る。
    // 元実装は距離だけを見て即 `Drag` を返していたため、素早いフリックが
    // 常に drag に食われ swipe が発火しなかった。
    let dt = current_time - state.start_time;
    if dt > 0.0 && dt < thresholds.swipe_max_ms {
        let velocity = distance / dt;
        if velocity > thresholds.swipe_min_velocity && distance > thresholds.swipe_min_px {
            // まだ確定させない。PointerUp で swipe か drag かを決める。
            return None;
        }
    }

    state.is_dragging = true;
    Some(Gesture::Drag { x: state.current_x, y: state.current_y })
}

// ============================================================
// math (core に無い f64 メソッドの代替)
// ============================================================

// `f64::sqrt` / `f64::abs` は std の inherent method であり core には無い。
// wasm ターゲットの no_std では libm を使う。テスト時 (std あり) は
// inherent method をそのまま使い、libm への依存を持ち込まない。

#[cfg(not(feature = "std"))]
#[inline]
fn sqrt(v: f64) -> f64 {
    libm::sqrt(v)
}

#[cfg(not(feature = "std"))]
#[inline]
fn fabs(v: f64) -> f64 {
    libm::fabs(v)
}

#[cfg(feature = "std")]
#[inline]
fn sqrt(v: f64) -> f64 {
    v.sqrt()
}

#[cfg(feature = "std")]
#[inline]
fn fabs(v: f64) -> f64 {
    v.abs()
}

// ============================================================
// tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

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
        // 160px を 100ms (1.6 px/ms)。閾値 0.5 の 3 倍以上。
        let got = run(
            &[
                (EventType::PointerDown, 100.0, 100.0, 0.0),
                (EventType::PointerMove, 200.0, 100.0, 50.0),
                (EventType::PointerUp, 260.0, 100.0, 100.0),
            ],
            &th,
        );
        assert_eq!(got, vec![Gesture::SwipeRight]);
    }

    #[test]
    fn swipe_left_fires() {
        let th = Thresholds::MOUSE;
        let got = run(
            &[
                (EventType::PointerDown, 260.0, 100.0, 0.0),
                (EventType::PointerMove, 160.0, 100.0, 50.0),
                (EventType::PointerUp, 100.0, 100.0, 100.0),
            ],
            &th,
        );
        assert_eq!(got, vec![Gesture::SwipeLeft]);
    }

    #[test]
    fn swipe_up_fires() {
        let th = Thresholds::MOUSE;
        let got = run(
            &[
                (EventType::PointerDown, 100.0, 260.0, 0.0),
                (EventType::PointerMove, 100.0, 160.0, 50.0),
                (EventType::PointerUp, 100.0, 100.0, 100.0),
            ],
            &th,
        );
        assert_eq!(got, vec![Gesture::SwipeUp]);
    }

    #[test]
    fn swipe_down_fires() {
        let th = Thresholds::MOUSE;
        let got = run(
            &[
                (EventType::PointerDown, 100.0, 100.0, 0.0),
                (EventType::PointerMove, 100.0, 200.0, 50.0),
                (EventType::PointerUp, 100.0, 260.0, 100.0),
            ],
            &th,
        );
        assert_eq!(got, vec![Gesture::SwipeDown]);
    }

    /// `PointerMove` を挟まない純粋なフリックでも swipe になる。
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
        assert_eq!(got, vec![Gesture::SwipeRight]);
    }

    // --- drag ---

    /// ゆっくり動かせば drag になる。速度が swipe 閾値未満。
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
            vec![
                Gesture::Drag { x: 150.0, y: 100.0 },
                Gesture::Drag { x: 200.0, y: 100.0 },
                Gesture::DragEnd,
            ]
        );
    }

    /// `PointerCancel` は `DragCancel` になり、`DragEnd` と区別できる。
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
        assert_eq!(got, vec![Gesture::Drag { x: 150.0, y: 100.0 }, Gesture::DragCancel]);
    }

    /// 閾値未満の移動では drag にならない。
    #[test]
    fn tiny_move_is_not_drag() {
        let th = Thresholds::MOUSE;
        let got = run(
            &[
                (EventType::PointerDown, 100.0, 100.0, 0.0),
                (EventType::PointerMove, 105.0, 100.0, 400.0),
                (EventType::PointerUp, 105.0, 100.0, 500.0),
            ],
            &th,
        );
        // 5px の移動は drag_start_px (10.0) 未満。時間が長いので tap でもない。
        assert_eq!(got, vec![]);
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
        assert_eq!(got, vec![Gesture::Tap]);
    }

    // --- long press: 元実装では発火しなかった経路 ---

    /// 押したまま動かさず離すと、`PointerUp` の時点で長押しが発火する
    /// (`detect_on_release` が拾う)。
    #[test]
    fn long_press_fires_on_release() {
        let th = Thresholds::MOUSE;
        let got = run(
            &[(EventType::PointerDown, 10.0, 10.0, 0.0), (EventType::PointerUp, 10.0, 10.0, 400.0)],
            &th,
        );
        assert_eq!(got, vec![Gesture::LongPress]);
    }

    /// 押したまま保持時間を超えてからわずかに動くと、その `PointerMove`
    /// の時点で長押しが発火する (`detect_on_move` が拾う)。
    #[test]
    fn long_press_fires_on_move_after_hold() {
        let th = Thresholds::MOUSE;
        let got = run(
            &[
                (EventType::PointerDown, 10.0, 10.0, 0.0),
                // 300ms 経過、移動は slop (9.0px) 未満。
                (EventType::PointerMove, 11.0, 10.0, 300.0),
            ],
            &th,
        );
        assert_eq!(got, vec![Gesture::LongPress]);
    }

    /// 長押しは 1 度しか発火しない。発火後に離しても重ねて返さない。
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
        assert_eq!(got, vec![Gesture::LongPress]);
    }

    /// 指がズレたら長押しにならない。
    #[test]
    fn long_press_cancelled_by_movement() {
        let th = Thresholds::MOUSE;
        let got = run(
            &[
                (EventType::PointerDown, 10.0, 10.0, 0.0),
                // slop (9.0px) を超えて動かす。drag_start_px (10.0) 未満
                // なので drag にもならない。
                (EventType::PointerMove, 19.5, 10.0, 100.0),
                (EventType::PointerUp, 19.5, 10.0, 400.0),
            ],
            &th,
        );
        assert_eq!(got, vec![]);
    }

    /// ドラッグ中は長押しにならない。
    #[test]
    fn long_press_suppressed_while_dragging() {
        let th = Thresholds::MOUSE;
        let got = run(
            &[
                (EventType::PointerDown, 10.0, 10.0, 0.0),
                // drag_start_px (10.0) を超えて動かし、drag を確定させる。
                (EventType::PointerMove, 40.0, 10.0, 100.0),
                // 保持時間 (251.0ms) を超えても、drag 中なので long press
                // にはならない。
                (EventType::PointerMove, 40.0, 10.0, 400.0),
            ],
            &th,
        );
        assert_eq!(
            got,
            vec![Gesture::Drag { x: 40.0, y: 10.0 }, Gesture::Drag { x: 40.0, y: 10.0 }]
        );
    }

    // --- drag_offset の保持 ---

    /// `PointerUp` で `drag_offset` / `drag_px` が消えない。
    ///
    /// `Handler` が `DragEnd` 後の `snap_region_to_unit` /
    /// `snap_point_to_unit` でこれらを読む。`~/w/app` 版は
    /// `..Self::default()` により消してしまい、スナップ位置がずれる。
    #[test]
    fn drag_offset_survives_release() {
        let mut state = PointerState::default();
        state = state.update(&EventType::PointerDown, 100.0, 100.0, 0.0);
        // Handler が書き込む想定。
        state.drag_offset = (12.0, 34.0);
        state.drag_px = (56.0, 78.0);

        state = state.update(&EventType::PointerUp, 100.0, 100.0, 50.0);

        assert_eq!(state.drag_offset, (12.0, 34.0));
        assert_eq!(state.drag_px, (56.0, 78.0));
    }

    // --- 装置別閾値 ---

    /// タッチではマウスより長押し判定が遅く、ブレ許容が広い。
    #[test]
    fn touch_thresholds_are_looser() {
        let mouse = Thresholds::for_device(Device::Mouse);
        let touch = Thresholds::for_device(Device::Touch);
        assert!(touch.long_press_ms > mouse.long_press_ms);
        assert!(touch.long_press_slop_px > mouse.long_press_slop_px);
        assert!(touch.drag_start_px > mouse.drag_start_px);
    }

    /// マウスでは長押しになる時間が、タッチではまだ長押しにならない
    /// (かつタップの最長時間も超えているため何も発火しない)。
    #[test]
    fn same_input_differs_by_device() {
        // 300ms: MOUSE (251.0) は超える、TOUCH (500.0) も TOUCH の
        // tap_max_ms (300.0) も超えない/満たさない。
        let events =
            [(EventType::PointerDown, 10.0, 10.0, 0.0), (EventType::PointerUp, 10.0, 10.0, 300.0)];
        assert_eq!(run(&events, &Thresholds::MOUSE), vec![Gesture::LongPress]);
        assert_eq!(run(&events, &Thresholds::TOUCH), vec![]);
    }
}
