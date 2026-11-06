// This file includes untranslated text (ja).

Reference data for `gesture::Pointer::track()`.

## References

- [hammerjs](https://github.com/hammerjs/hammer.js/)
- [use-gesture](https://github.com/pmndrs/use-gesture)

判定アルゴリズム（distance / velocity / duration の閾値比較）に埋め込まれた値を、両ライブラリからpx・ms単位に揃えて `Thresholds` に統合した。

`Pointer::track` は `Pointer` の状態と `Event` 1 個だけから `Gesture` を導出する。
実時間情報`Event` (タイマー)には依存せず実装する。

## Hammer.js, use-gesture 共通の関数

- Hammer.js: `src/inputjs/get-distance.js`
- use-gesture: `packages/core/src/utils/maths.ts`, etc.

```
distance(p0, p1)  = sqrt((p1.x - p0.x)^2 + (p1.y - p0.y)^2)   # px
velocity(p0, p1)  = distance(p0, p1) / (p1.t - p0.t)          # px/ms
direction(dx, dy) = abs(dx) > abs(dy)
                       ? (dx > 0 ? RIGHT : LEFT)
                       : (dy > 0 ? DOWN  : UP)
```

## 状態遷移（`Pointer::track`）

`pointer_id` 単位で `TwoPointerState`（`primary` / `secondary` / `mode`）を管理し、
単指判定用の状態（`start`, `current`, `last_move`, フラグ群）は `primary` に対してのみ更新する。

| Event | 条件 | 動作 |
|---|---|---|
| PointerDown | 既知の id | 座標更新のみ（単指状態はリセットしない） |
| | `primary` 無し | 単指状態をリセットして `primary` に登録 |
| | `secondary` 無し | `secondary` に登録、`mode = None`。`is_dragging` なら `DragCancel` を発火して解除 |
| | 3 本目以降 | 無視 |
| PointerMove | 未登録の id | 無視 |
| | `secondary` 有り | 2 本指の畳み込み（後述）。`primary` 単独の判定は凍結 |
| | `primary` 単独 | `current` / `last_move` を更新し `detect_move` |
| PointerUp / Cancel | `primary` 単独 | `current` / `cancelled` を更新し `detect_release`、`primary` を解除 |
| | 2 本指中 | 該当 id を解除、`mode` に応じて終了処理（後述）、残った指の現在位置へ単指状態を `relocate`（起点座標の置き直し。`start_time` と `long_press_fired` は保持）。副ポインタが昇格した場合は `start_time` をその接触時刻に差し替え、`long_press_fired` を false に戻す。残らなければ全消去 |

`relocate` 後は `Pointer.start` と `primary` の `TrackedPointer::start` が一致しなくなる（1 本目の `PointerDown` は `start` で両者を同じ座標・時刻に初期化する）。この不一致を厳密比較で検出している間は `Tap` / `LongPress` / `Swipe` を抑止し、`Drag` 系のみ通す。Pan 確定時の中点への `relocate` でも同じ不一致が生じるため、2 本指パン終了時の `Tap` も同時に抑止される。残った指が 2 本指中に動いていなければ不一致は生じず、保持した `start_time` から通常どおり `Tap` / `LongPress` を判定する。`Gesture` は指の本数を持たないため、動かずに触れて離れた 2 本指タップも 1 本指の `Tap` として返る。

`last_move` は直近の `PointerMove` の座標・時刻（無ければ `PointerDown` のそれ）。swipe の速度計算窓に使う。

## 単指の判定順

1. 終了イベント (`PointerUp` / `PointerCancel`) → `detect_release`
   1. `is_dragging` → `DragEnd` / `DragCancel`
   2. `cancelled` → 何も返さない（tap/swipeに昇格させない）
   3. `velocity > swipe_min_velocity && distance > swipe_min_px && dt < swipe_max_ms` → `Swipe{Up,Down,Left,Right}`（`direction(dx, dy)` で4方向判定。`velocity` と `distance` の計算窓は異なる。後述）
   4. `long_press_fired` 済み → 何も返さない（tapを重ねない）
   5. `dt > long_press_ms && distance < long_press_slop_px` → `LongPress`（`PointerMove` が一度も来ていないケースの最後の判定機会）
   6. `dt < tap_max_ms && distance < tap_slop_px` → `Tap`
2. 移動イベント (`PointerMove`) → `detect_move`
   1. `!long_press_fired && !is_dragging && distance < long_press_slop_px && dt > long_press_ms` → `LongPress`（ラッチして以後は発火しない）
   2. `distance <= drag_start_px` → 何もしない
   3. 既に `is_dragging` → `Drag{x,y}` を継続発火
   4. まだswipeになりうる速度・距離なら確定を保留（`PointerUp`側に譲る）
   5. それ以外 → `is_dragging = true` にして `Drag{x,y}` 発火

`LongPress` はタイマー駆動ではないため、指を完全に静止させたまま保持し続けた場合、発火は次の `PointerMove` か `PointerUp` まで遅延する。

### velocity の計算窓

`detect_release` の swipe 判定では、`distance` / 方向は `start` → `current`、`velocity` は `last_move` → `current` から計算する。

`start` からの平均速度だと、序盤に速く動いた後に止めたまま保持して離した場合に swipe と誤判定される。直近区間で計算すれば止まっていた時間ぶん `move_dt` が伸びて速度が自然に下がる。`PointerMove` が無ければ `last_move == start` のため平均計算と一致する。

Hammer.js `COMPUTE_INTERVAL`（25ms）や use-gesture `BEFORE_LAST_KINEMATICS_DELAY`（32ms。pointerup と直前の pointermove の時間差がこれ以上なら velocity=0 とする）と同じ問題意識だが、本実装は速度計算の 2 点を変えるだけで同じ効果を得ている。`detect_move` 側の保留判定（2.4）は `start` からの平均のまま。

## 2 本指の判定（`TwoPointerState::fold`）

`secondary` が存在する間の `PointerMove` ごとに、両指の `start` → `current` 変位から `mode` を確定し、確定後はラッチする。

1. 両指の変位がともに `two_pointer_commit_in_px` を超えるまで `mode = None`（何も返さない）
2. 変位ベクトルの内積 `< pinch_dot_max` → `Pinch`、それ以外 → `Pan`
3. `Pinch` → `Pinch{scale, center_x, center_y}` を発火。`scale` は `start` 間距離に対する `current` 間距離の比、`center` は両指の中点
4. `Pan` → 両指の中点を単一ポインタに畳み込む。初回は中点へ `relocate` して `panning = true`、以後は `PointerMove` として `detect_move` に流す（`Drag` 系として発火）

いずれかの指が離れたときの終了処理は `mode` に依存する。

| 終了時の `mode` | 動作 |
|---|---|
| None | 何も返さない |
| Pinch | `PinchEnd` |
| Pan | `panning` なら中点座標で `detect_release`（`DragEnd` / `DragCancel`）、`panning = false` |

## 閾値（`Thresholds`）

表中の名称は `_in_` を省いた略記で、実装では `tap_max_in_ms` のように単位の前に `_in_` が入る。`pinch_dot_max` は無次元（px²）で、変位ベクトルの内積がこれ未満なら `Pinch`。

| フィールド | 由来 | Hammer.js値 | use-gesture値 | 採用値（MOUSE / TOUCH） |
|---|---|---|---|---|
| `tap_max_ms` | tap.time | 250ms | - | 250 / 300 |
| `tap_slop_px` | tap.threshold | 9px | tapsThreshold: 3px | 9 / 16 |
| `long_press_ms` | press.time | 251ms（tapと1ms差で排他） | dragDelay: 180ms | 251 / 500 |
| `long_press_slop_px` | press.threshold | 9px | - | 9 / 16 |
| `drag_start_px` | pan.threshold | 10px | axisThreshold: 0px | 10 / 16 |
| `swipe_min_px` | swipe.threshold | 10px | swipeDistance: 50px | 50 / 50 |
| `swipe_min_velocity` | swipe.velocity | 0.3px/ms | swipeVelocity: 0.5px/ms | 0.5 / 0.5 |
| `swipe_max_ms` | - | - | swipeDuration: 250ms | 250 / 300 |
| `two_pointer_commit_px` | - | - | - | 8 / 8 |
| `pinch_dot_max` | - | - | - | 0 / 0 |

- `swipe_min_velocity` と `swipe_min_px` は use-gesture のより保守的な値を採用。
- `TOUCH` はタッチの座標ブレを考慮し、ブレ許容系（`*_slop_px`, `drag_start_px`）と時間系（`long_press_ms`, `tap_max_ms`, `swipe_max_ms`）を `MOUSE` より広げている。両ライブラリともデバイス別の既定値分岐は持たないため、実装側の独自拡張。
- rotate・wheel系は両ライブラリに定数があるが対象外。
