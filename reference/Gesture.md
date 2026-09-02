# Gesture

Reference data for `js_client::detect_gesture()`

## References

- [hammerjs](https://github.com/hammerjs/hammer.js/)
- [use-gesture](https://github.com/pmndrs/use-gesture)

判定アルゴリズム（distance / velocity / duration の閾値比較）に埋め込まれた値を、 `gesture_fixed.rs` の `Thresholds` / `detect_gesture` に、両ライブラリの値をpx・ms単位に揃えて統合した。

`detect_gesture` は `PointerState` と `Event` 1 個だけから `Gesture` を導出する関数であり、タイマー等の第二の入口は持たない。`app.rs` の `dispatch` が `Event` を FIFO キューで捌く都度確定型アーキテクチャのため、時間経過そのものを表す `Event` は存在しない。`LongPress` もこの制約の中で、既存の `PointerMove` / `PointerUp` の判定に組み込んでいる（詳細は後述）。

## Hammer.js, use-gesture 共通の関数と定数

- Hammer.js: `src/inputjs/get-distance.js` 
- use-gesture: `packages/core/src/utils/maths.ts`, etc.

```
# 疑似コード
distance(p0, p1)  = sqrt((p1.x - p0.x)^2 + (p1.y - p0.y)^2)   # px
velocity(p0, p1)  = distance(p0, p1) / (p1.t - p0.t)          # px/ms
angle(p0, p1)     = atan2(p1.y - p0.y, p1.x - p0.x) * 180/pi  # deg
direction(dx, dy) = abs(dx) > abs(dy)
                       ? (dx > 0 ? RIGHT : LEFT)
                       : (dy > 0 ? DOWN  : UP)

# Constants（unit: px, ms, px/ms）
tap_max_ms          = 250   # Hammer.js: tap.time
tap_slop_px         = 9     # Hammer.js: tap.threshold        (use-gesture: tapsThreshold = 3)
long_press_ms       = 251   # Hammer.js: press.time           (tap.timeと1ms差で排他)
long_press_slop_px  = 9     # Hammer.js: press.threshold
drag_start_px       = 10    # Hammer.js: pan.threshold        (use-gesture: axisThreshold = 0)
swipe_min_px        = 50    # use-gesture: DEFAULT_SWIPE_DISTANCE (Hammer.js: swipe.threshold = 10)
swipe_min_velocity  = 0.5   # use-gesture: DEFAULT_SWIPE_VELOCITY (Hammer.js: swipe.velocity = 0.3)
swipe_max_ms        = 250   # use-gesture: DEFAULT_SWIPE_DURATION (Hammer.jsに対応項目なし)
```

## detect_gesture

```rust
use js_client::{
    PointerState, 
    Thresholds::{MOUSE, TOUCH}, 
    detect_gesture, 
    detect_on_release, 
    detect_on_move,
    gesture_tests,
};

// app::App::init()
use js_client::{pointer_coarse, detect_device, Thresholds::for_device};

// app::App::dispatch(Event::Canvas)
use js_client::{PointerState::update, detect_gesture};
```

### 状態遷移

```
PointerDown → start_x/y, start_time, last_move_x/y/time を記録, is_down = true
PointerMove → current_x/y, last_move_x/y/time を更新
PointerUp/Cancel → current_x/y, cancelled フラグを更新, is_down = false
                   （座標・時刻は消さない。判定はこの直後に行う）
```

`last_move_x/y/time` は直近の `PointerMove` の座標・時刻（無ければ `PointerDown` のそれ）。swipe の速度計算窓（後述）に使う。

### 判定順（`detect_gesture`）

1. **終了イベント** (`PointerUp` / `PointerCancel`) → `detect_on_release`
   1. ドラッグ中だった (`prev_state.is_dragging`) → `DragEnd` / `DragCancel` で確定
   2. `cancelled` → 何も返さない（tap/swipeに昇格させない）
   3. `velocity > swipe_min_velocity && distance > swipe_min_px && dt < swipe_max_ms` → `Swipe{Up,Down,Left,Right}`（dx/dyの絶対値が大きい方向で4方向判定。`velocity` と `distance` の計算窓は異なる。後述）
   4. `long_press_fired` 済み → 何も返さない（tapを重ねない）
   5. `dt > long_press_ms && distance < long_press_slop_px` → `LongPress`（動かないまま保持時間を超えて離したケース。`PointerMove`が一度も来ていないため、ここが最後の判定機会）
   6. `dt < tap_max_ms && distance < tap_slop_px` → `Tap`
2. **移動イベント** (`PointerMove`) → `detect_on_move`
   1. `!long_press_fired && !is_dragging && distance < long_press_slop_px && dt > long_press_ms` → `LongPress`（動かないまま保持時間を超えた後、わずかに動いた最初の`PointerMove`で確定。ラッチして以後は発火しない）
   2. `distance <= drag_start_px` → 何もしない
   3. 既に `is_dragging` → `Drag{x,y}` を継続発火
   4. まだswipeになりうる速度・距離なら確定を保留（`PointerUp`側に譲る）
   5. それ以外 → `is_dragging = true` にして `Drag{x,y}` 発火

`LongPress`はタイマー駆動ではなく、上記2箇所（release/move）に分散して埋め込まれている。指を完全に静止させたまま保持し続けた場合、実際の発火は次に`PointerMove`か`PointerUp`が届いた瞬間まで遅延する。これは「1 Event → 1 Gesture」という関数の形を保つための制約であり、フレームループやタイマーからの明示的な追加呼び出しは行わない。

### velocity の計算窓

`detect_on_release` の swipe 判定で、`distance`（方向判定にも使う）と `velocity` は異なる区間から計算する。

- `distance` / 方向 (`dx`, `dy`) : `start` → `current`（ジェスチャ全体の変位）
- `velocity` : `last_move` → `current`（直近の `PointerMove` からの区間）

`start` からの平均だけで速度を出すと、序盤に大きく速く動いた後に指を止めたまま保持してから離した場合、距離が大きいままなので平均速度が閾値を超え続け、実際には止まっていたのに swipe と誤判定される。直近区間（`last_move` → `current`）で計算すれば、動きが止まっていた分だけ区間の時間 (`move_dt = current_time - last_move_time`) が伸びて速度は自然に下がる。`PointerMove` が一度も来ていない場合は `last_move` は `start` と等しいため、この式は従来の平均計算と一致する（`swipe_without_move_event` のケース）。

これは Hammer.js `COMPUTE_INTERVAL`（velocity の再計算間隔、25ms）や @use-gesture
`BEFORE_LAST_KINEMATICS_DELAY`（最終イベント直前の運動量計算を有効とみなす時間差の閾値、32ms。「pointerup とその直前の pointermove の時間差がこれ以上なら『止まってから離した』と判断して velocity=0 を確定する」）と同じ問題意識に基づく。両ライブラリは複数サンプルの窓や明示的なゼロ化で対処するが、本実装は 1 Event = 1 Gesture の制約下で「速度計算に使う 2 点を `last_move`/`current` に変える」だけで同じ効果を得ている（`last_move` から動いていなければ距離 0 になり、自動的に velocity が下がるため、明示的なゼロ化は不要）。`detect_on_move` 側の deferral 判定（項目 2.4）は `start` からの平均のままで、この窓の変更は release 側のみに適用してある。

### 閾値（`Thresholds`）— px / ms に統一

| フィールド | 由来 | Hammer.js値 | @use-gesture値 | 採用値（MOUSE / TOUCH） |
|---|---|---|---|---|
| `tap_max_ms` | tap.time | 250ms | — | 250 / 300 |
| `tap_slop_px` | tap.threshold | 9px | tapsThreshold: 3px | 9 / 16 |
| `long_press_ms` | press.time | 251ms（tapと1ms差で排他） | dragDelay: 180ms | 251 / 500 |
| `long_press_slop_px` | press.threshold | 9px | — | 9 / 16 |
| `drag_start_px` | pan.threshold | 10px | axisThreshold: 0px | 10 / 16 |
| `swipe_min_px` | swipe.threshold | 10px | swipeDistance: 50px | 50 / 50 |
| `swipe_min_velocity` | swipe.velocity | 0.3px/ms（300px/s） | swipeVelocity: 0.5px/ms（500px/s） | 0.5 / 0.5 |
| `swipe_max_ms` | ― （Hammer.jsに対応する項目なし） | — | swipeDuration: 250ms | 250 / 300 |

- `swipe_min_velocity` と `swipe_min_px` は @use-gesture のより保守的な値（誤発火が少ない）を採用。
- `TOUCH`はタッチの接触面の広さ・座標ブレを考慮し、ブレ許容系（`*_slop_px`, `drag_start_px`）と時間系（`long_press_ms`, `tap_max_ms`, `swipe_max_ms`）を`MOUSE`より広げている。Hammer.js/@use-gestureともにデバイス別の既定値分岐は持たないため、ここは実装側の独自拡張。
- pinch/rotate/pan(2本指)・wheel系はどちらのライブラリにも定数はあるが、`detect_gesture`は単一ポインタのtap/press/swipe/dragのみを扱うため対象外（2本指ジェスチャーの検討は別途進めている。追加する場合は本表と同じ形式で追記する）。

### 除外した値（今回の関数の対象外）

- Hammer.js: pinch/rotate（2ポインタ）、`STATE_*`（内部状態機械のbitmask）、`DIRECTION_*`（bitmask定数、本実装ではenum variantで代替済み）、`COMPUTE_INTERVAL`（velocity再計算間隔、本実装は毎イベント計算のため不要）、`DEDUP_TIMEOUT`/`DEDUP_DISTANCE`（touch/mouse合成イベント除去、PointerEvent統一により不要）
- @use-gesture: pinch/rotate/wheel/keyboard系一式、`BEFORE_LAST_KINEMATICS_DELAY`（velocity計算の内部実装詳細。上記の通り windowを変えることで同じ効果を得ているため定数としては未採用）、`LINE_HEIGHT`/`PAGE_HEIGHT`（wheel専用）
