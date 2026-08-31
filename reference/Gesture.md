// This file includes untranslated text (ja).

# Gesture

Reference data for `js_client::detect_gesture()`

## Hammer.js 定数一覧

### ジェスチャー認識閾値（人間の動作特性由来）

#### Tap — `src/recognizers/tap.js`

| 定数 (defaults key) | 値 | 単位 |
|---|---|---|
| `interval` | 300 | ms（マルチタップ間の最大間隔） |
| `time` | 250 | ms（指を押し下げていられる最大時間） |
| `threshold` | 9 | px（タップ中の許容移動距離） |
| `posThreshold` | 10 | px（マルチタップの位置ズレ許容量） |
| `taps` | 1 | 回（認識に必要なタップ数） |
| `pointers` | 1 | 本（使用ポインタ数） |

> `time: 250` と `press.time: 251` は意図的に1ms差をつけてあり、tap と press を排他的に区別するための設計。

#### Press — `src/recognizers/press.js`

| 定数 | 値 | 単位 |
|---|---|---|
| `time` | 251 | ms（長押し認識の最短保持時間） |
| `threshold` | 9 | px（許容移動距離） |
| `pointers` | 1 | 本 |

#### Swipe — `src/recognizers/swipe.js`

| 定数 | 値 | 単位 |
|---|---|---|
| `threshold` | 10 | px（スワイプ認識の最小移動距離） |
| `velocity` | 0.3 | px/ms（スワイプ認識の最低速度 = 300 px/s） |
| `pointers` | 1 | 本 |
| `direction` | `DIRECTION_ALL` | — |

#### Pan — `src/recognizers/pan.js`

| 定数 | 値 | 単位 |
|---|---|---|
| `threshold` | 10 | px（パン開始の最小移動距離） |
| `pointers` | 1 | 本 |
| `direction` | `DIRECTION_ALL` | — |

#### Pinch — `src/recognizers/pinch.js`

| 定数 | 値 | 単位 |
|---|---|---|
| `threshold` | 0 | —（scale変化があれば即認識） |
| `pointers` | 2 | 本 |

#### Rotate — `src/recognizers/rotate.js`

| 定数 | 値 | 単位 |
|---|---|---|
| `threshold` | 0 | deg（回転があれば即認識） |
| `pointers` | 2 | 本 |

---

### 入力処理系

#### `src/inputjs/input-consts.js`

| 定数 | 値 | 備考 |
|---|---|---|
| `COMPUTE_INTERVAL` | 25 | ms（velocity等の再計算間隔） |
| `INPUT_START` | 1 | bitmask |
| `INPUT_MOVE` | 2 | bitmask |
| `INPUT_END` | 4 | bitmask |
| `INPUT_CANCEL` | 8 | bitmask |
| `DIRECTION_NONE` | 1 | bitmask |
| `DIRECTION_LEFT` | 2 | bitmask |
| `DIRECTION_RIGHT` | 4 | bitmask |
| `DIRECTION_UP` | 8 | bitmask |
| `DIRECTION_DOWN` | 16 | bitmask |
| `DIRECTION_HORIZONTAL` | `LEFT \| RIGHT` = 6 | bitmask |
| `DIRECTION_VERTICAL` | `UP \| DOWN` = 24 | bitmask |
| `DIRECTION_ALL` | 30 | bitmask |

#### `src/input/touchmouse.js`

| 定数 | 値 | 単位 |
|---|---|---|
| `DEDUP_TIMEOUT` | 2500 | ms（touch後の合成mouseイベント除去ウィンドウ） |
| `DEDUP_DISTANCE` | 25 | px（合成イベント判定の座標許容誤差） |

---

### Recognizer ステート — `src/recognizerjs/recognizer-consts.js`

| 定数 | 値 | 備考 |
|---|---|---|
| `STATE_POSSIBLE` | 1 | bitmask |
| `STATE_BEGAN` | 2 | bitmask |
| `STATE_CHANGED` | 4 | bitmask |
| `STATE_ENDED` | 8 | bitmask |
| `STATE_RECOGNIZED` | `STATE_ENDED` = 8 | エイリアス |
| `STATE_CANCELLED` | 16 | bitmask |
| `STATE_FAILED` | 32 | bitmask |

---

### Manager — `src/manager.js`

| 定数 | 値 | 備考 |
|---|---|---|
| `STOP` | 1 | セッション停止フラグ |
| `FORCED_STOP` | 2 | 強制停止フラグ |

---

### TouchAction — `src/touchactionjs/touchaction-Consts.js`

文字列定数のみ（CSS `touch-action` プロパティ値のラッパー）。数値なし。

---

### px直接計算ロジック（最低層）

以下のファイルは座標・距離・速度・角度をpxやrad単位で直接演算する実装層。定数値は持たず、入力値をそのまま計算する。

- `src/inputjs/compute-delta-xy.js`
- `src/inputjs/compute-input-data.js`
- `src/inputjs/get-angle.js`
- `src/inputjs/get-center.js`
- `src/inputjs/get-delta-xy.js` ※ファイルが存在する場合
- `src/inputjs/get-direction.js`
- `src/inputjs/get-distance.js`
- `src/inputjs/get-rotation.js`
- `src/inputjs/get-scale.js`
- `src/inputjs/get-velocity.js`

## @use-gesture 定数一覧

### ジェスチャー認識閾値（人間の動作特性由来）

#### Drag — `packages/core/src/config/dragConfigResolver.ts`

| 定数 | 値 | 単位 |
|---|---|---|
| `DEFAULT_PREVENT_SCROLL_DELAY` | 250 | ms（スクロール抑制を確定するまでの待機時間） |
| `DEFAULT_DRAG_DELAY` | 180 | ms（ドラッグ開始を遅延認識する時間、長押し判定用） |
| `DEFAULT_SWIPE_VELOCITY` | 0.5 | px/ms（スワイプ認識の最低速度 = 500 px/s） |
| `DEFAULT_SWIPE_DISTANCE` | 50 | px（スワイプ認識の最小移動距離） |
| `DEFAULT_SWIPE_DURATION` | 250 | ms（スワイプとして認識される最大ジェスチャー時間） |
| `DEFAULT_KEYBOARD_DISPLACEMENT` | 10 | px（矢印キー1回押しによる移動量） |
| `tapsThreshold` (default) | 3 | px（タップ判定の移動距離許容量） |
| `DEFAULT_DRAG_AXIS_THRESHOLD` | mouse:0, touch:0, pen:8 | px（軸ロック判定の閾値、デバイス別） |

#### Pinch — `packages/core/src/config/pinchConfigResolver.ts`

| 定数・default値 | 値 | 単位 |
|---|---|---|
| `threshold` (lockDirection時) | [0.1, 3] | [scale倍率, deg]（軸ロック時のpinch/rotate判定閾値） |
| `threshold` (通常) | 0 | —（変化があれば即認識） |

#### Coordinates（pan/scroll/wheel共通） — `packages/core/src/config/coordinatesConfigResolver.ts`

| 定数 | 値 | 単位 |
|---|---|---|
| `DEFAULT_AXIS_THRESHOLD` | 0 | px（軸方向ロック判定閾値のデフォルト） |

#### Rubberband（共通） — `packages/core/src/config/commonConfigResolver.ts`

| 定数 | 値 | 備考 |
|---|---|---|
| `DEFAULT_RUBBERBAND` | 0.15 | 係数（バウンド外での減衰率、`rubberband: true`時） |

---

### エンジン内部

#### Engine — `packages/core/src/engines/Engine.ts`

| 定数 | 値 | 単位 |
|---|---|---|
| `BEFORE_LAST_KINEMATICS_DELAY` | 32 | ms（最終イベント直前の運動量計算を有効とみなす時間差の閾値） |

> ドラッグ終了時にvelocityが常に[0,0]になる問題を防ぐための値。pointerupとその直前のpointermoveの時間差がこれ以上なら「止まってから離した」と判断してvelocity=0を確定する。

#### PinchEngine — `packages/core/src/engines/PinchEngine.ts`

| 定数 | 値 | 備考 |
|---|---|---|
| `SCALE_ANGLE_RATIO_INTENT_DEG` | 30 | deg（pinchとrotateを区別する意図判定の角度閾値） |
| `PINCH_WHEEL_RATIO` | 100 | ホイールイベントをpinch scaleに変換する係数 |

#### TimeoutStore — `packages/core/src/TimeoutStore.ts`

| 定数 | 値 | 単位 |
|---|---|---|
| `ms` (default) | 140 | ms（タイムアウト登録時のデフォルト待機時間） |

---

### ホイール正規化 — `packages/core/src/utils/events.ts`

| 定数 | 値 | 備考 |
|---|---|---|
| `LINE_HEIGHT` | 40 | px（`deltaMode=1`時のline単位→px換算値、Firefox対応） |
| `PAGE_HEIGHT` | 800 | px（`deltaMode=2`時のpage単位→px換算値） |

---

### キーボード操作マップ — `packages/core/src/engines/DragEngine.ts`

文字列→ベクトル変換のマップ定数。数値は `DEFAULT_KEYBOARD_DISPLACEMENT` を受け取って乗算するため、ここに固有の数値はなし。

---

### px直接計算ロジック（最低層）

- `packages/core/src/utils/maths.ts`
- `packages/core/src/utils/events.ts`（`pointerValues`, `touchDistanceAngle`, `distanceAngle`）
- `packages/core/src/engines/CoordinatesEngine.ts`
- `packages/core/src/engines/PinchEngine.ts`（scale/angle計算部）
