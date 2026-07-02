// This file includes untranslated text (ja).

# Architecture

Gui application system for editing and reading structured data.

---

## Rule

- [common for projects](https://github.com/animagram-jp/.github/blob/main/Rule.md)

## Commands

```bash
# unit test
cargo test

# wasm-pack compile
wasm-pack build --target web --out-dir examples/app --out-name app
```

- OPFS files are in
   - `C:\Users\<User>\AppData\Roaming\Mozilla\Firefox\Profiles\<Profile>\storage\default\`.
   - `C:\Users\<User>\AppData\Local\Google\Chrome\User Data\Default\Storage\ext\`

## Debug

- for iPhone: [url with eruda](https://animagram-jp.github.io/app/?eruda)

---

## System diagram

```
┌──────┐
│ user │
└──┬───┘
   ▼
┌────────────────────────┐
│ browser                │
│┌──────┐┌──────┐┌──────┐│
││ dom  ││ opfs ││ sw   ││
│└──────┘└──────┘└──────┘│
│┌──────────────────────┐│
││ app (web worker)     ││
│└──────────────────────┘│
│  ▲                  │▲ │
│  │ post/onMessage   ││ │
│  ▼                  ││ │
│┌──────────────────┐ ││ │
││ extension worker │ ││ │
│└──────────────────┘ ││ │
└─ ▲ ──────────────── ││─┘
   │                  ││
   │ native messaging ││websocket
   ▼                  ││
┌───────────────┐     ││
│ native worker │     ││
│ (host api)    │     ││
└──┬────────────┘     ││
   │ http request     ││
   │     ┌──────┬─────┤│
   │     ▼stun  ▼turn │←:http
   │ ┌──────┐┌──────┐ ││
   │ │ stun ││ turn │ ││
   ▼ └──────┘└──────┘ ▼▼
┌────────────────────────┐
│ server                 │
│┌──────────────────────┐│
││ nginx (external port)││
│└──────────────────────┘│
│┌──────────────────────┐│
││ app (rust)           ││
│└──────────────────────┘│
│┌──────────────────────┐│
││ vfs                  ││
│└──────────────────────┘│
└────────────────────────┘
```

---

## html

- index.htmlの1ファイル完結。
- FOUC防止のため、body atrributeにhiddenを書く。
- 初期表示しないelement以外、.hiddenクラスを追加しておく。
- text content: 一切変化しないテキストは書き込むが、言語切り替えが必要なので、原則書かない。
- 動的に増えるelement: ディスプレイと人間の物理制約を考慮した最大数を決め、-1,-2,...をidの末尾に付けてhtmlに静的に書き込む。
- cssと作成作業を分離する。一部の開閉要素を特定タグで表すほかは、htmlの責務を、静的に決定される列指向のグリッド要素による、要件の描画表現に限定する。
- cssスタイリング作業を同時に行わない。css作成時は作成済みのhtmlを編集可能。
- html作成時の責務を2つに限定する。
    - 1. 要件から導出される、ディスプレイ表示するべきデータモデルを網羅し、ハードウェアの物理特性と人類の生物特性から、適切な最大インスタンスフィールド数を決定して、列指向のグリッドレイアウトに静的に配置場所を決定する。描画状況の取得が二度手間になるので、`display: grid`などの自動配置は利用を避ける。
    - 2. 要素に対し、適切なタグを選択することで、ブラウザなどの支援機能を受けやすくする。`hidden`,`.hidden`,`display:`等のレイアウトプロパティを定義・適用する。
      - 視覚効果: `<main>`,`<dialog id="drawer">`,`<dialog id="modal">`,`<form>`,`<toast>` (いずれも、1画面でbody直下に単一要素とする)
      - 集合要素: `<header>`,`<footer>`,`<section>`,`<article>`,`<fieldset>`,`<table>`
      - 段落要素: `display: block`,
      - 同列要素: `display: inline-block`, `display: inline`, `display: table-cell`
      - htmlの制約として、同列要素の中に段落要素を格納する、すなわち集合要素を持つべき時、タグを子に分離する必要がある。この時も、セマンティクスを最もよく表すタグを選択する。
- idはbody以降の親tag・その連番と、同層同tagの連番から機械的に決定される。
  - id規則:
    - "_" = 親子セグメント区切り  例: main_div_section-1
    - "-N" = 同タグ内の連番      例: span-3, th-2
    - 連番なし = その階層に1つだけ 例: thead_tr, legend_h5
- 各element内の記述順は、`<tagname, id, html standard attribute, aria-label, class, class unique attribute>`。
- formatting rule:
    - Do not insert a line break before a closing tag.
    - Insert a line break before the start of every tag.

```yaml
# htmlの基本構造
html:
  head:
  body:
    main:          # 主に閲覧機能
      header:
      section-{N}: # または、その他のsemantic tag。
      footer:
    drawer:    # 画面遷移時のメニュー表示。手動ではなくappが開閉する。<dialog id="drawer">
    modal:     # 編集機能・要アテンション時 <dialog id="modal"> showModal()
    form:      # <form id="form" method="dialog"></form> のみの1行要素
    toast:     # <output>: info~warningまでの重要度をポップアップ通知する。
```

### css

- config.css(変数定義), style.css, idや構造に依存のない外部css。
- style.cssにて[hidden], .hidden {display: none !important;} を定義する。
- tagまたはidのリレーションでセレクタを定義する。セレクタ指定のためにclassを新設してはいけない。

### javascript

- アプリに依らず、同内容の4ファイルで構成する:
  - htmlにmoduleとして呼ばれるメインスレッドinit.js
  - メインと非同期なdedicated web workerスレッドでappを実行するためのworker.js
  - wasm-packで自動生成されるapp.js (app_bg.wasmを実行)
  - pwaのservice workerを起動するためのsw.js
- init.js: workerに適宜eventをpostMessageで渡す。また、excute()でappからの指示を実行する。

```js
// init.js
// Command: { operation: u8, id: string, attribute?: string, value?: string }
function execute({ operation, id, attribute, value }) {
  const el = document.getElementById(id);
  if (!el) return;
  switch (operation) {
    case 1: el.textContent = value ?? ""; break;
    case 2: el.value = value ?? ""; break;
    case 3: el.toggleAttribute(attribute, value === "true"); break;
    case 4: el.classList.add(value); break;
    case 5: el.classList.remove(value); break;
    case 6: el.focus(); break;
    case 7: el.showModal(); break;
    case 8: el.close(); break;
    case 9: applyClass(el, value); break;
    case 10: el.innerHTML = value ?? ""; break;
  }
}
```

### wasm

- app.wasmとしてコンパイルする。

#### js_client.rs

```rust
use js_client::{
    Operation, Command,
    get_js_str, get_js_u32, get_js_f64, get_js_field,
    EventType, KeyName,
    Device, Gesture, PointerState,
    dom, CanvasEvent
};
```

- DOM Living Standard知識の操作対象と操作関数を定義する。
- DOMのステートはブラウザが保持しているので、操作関数は引数に取る。
- 端末・人間の特性値に関わる操作関数(`detect_gesture`)は既存の知見を参照する: [Gesture.md](./Gesture.md)

#### list.rs

```rust
use list::{
    List::{new, get, set, delete},
    VariableList::{new, new_from_bytes, get, get_from_bytes, set, delete},
};
```

- 可変長論理バイト列の宣言と、固定長要素列操作Listと可変長(バイト倍数)要素列操作VariabeList。
- FileStoreのプールメモリの読み取りに対応して、バイト列読み取り関数new_from_bytesとget_from_bytesも公開。

#### store.rs

```rust
use store::FileStore::{new, issue, get, set, save, delete, close, compact};
```

- ローカルストア操作を発行するモジュール。log snapペアファイルによる回復機能をvfs(opfs)上で実行。
- ステートにプールメモリとunsavedインデックスセットを持ち、メモリオンリーの操作関数と、ディスクへの反映関数を分離して公開。
- 1つのインスタンスは、1つの可変長論理バイト列に対する保存単位(想定はデータモデルインスタンス1つ)に対する操作を提供する。

#### timestamp.rs

```rust
use timestamp::{
    Field, YEAR, MONTH, DAY, HOUR, MINUTE, SECOND, DECISECOND, IS_UTC, TIMEZONE, Timezone,
    from_ut, new, display, unpack, pack, 
    add_years, sub_years, add_months, sub_months, add_days, sub_days, 
    add_hours, sub_hours, add_minutes, sub_minutes
};
```

- TZ, decisecondsまでとカレンダー加減算に対応した、u64 timestampモジュール。

#### data_struct.rs

```rust
use data_struct::DataStruct::{new, get_from_bytes, get, set, delete, compact, to_bytes, from_bytes};
```

- データモデル固有のフィールド数(schema_size)固定Listと可変部VariableListによるデータインスタンス操作モジュール。
- フィールド1にid(u32), 2にcreated_at(timestamp), 3にupdated_at(timestamp)を確定し、4~を開放。

#### object.rs

```rust
use object::{
    Dice, dice::{display, roll}, 
    Character, Profile, Characteristic, Skill, 
    ArtAndCraft, Fighting, Firearms, Pilot, Science, Survival,
};
```

- ドメイン固有のデータモデルの全フィールドとロジックを、各自公開されたenumのネスト群で表現したモジュール。
- 関数はitemのドメイン意味(表示)を定義する`label`, 一意なschema_idを発行する`id`, バイト列とdomからの流入(u32,str,f64)を相互変換する`read` / `write`, 値の表示を導出する`display`などを各enum itemに対して定義する。

#### event.rs

```rust
use event::{Dialog, Handler, Toast};
```

- canvasを操作する、ドメイン固有のstate定義とhandler。
- handlerは、DataStructと、フィールド4~schema_sizeまでの操作ロジックを定義するobjectを束ねて操作を行う。
- js_clientのdom::Idとobjectのフィールドを相互にバルクマッピングする関数を定義して、canvasと内部データを互換する。

#### app.rs

```rust
use app::{Event, App::{init, close, process, dispatch}};
```

- initとprocessの公開apiを持つ、Appインスタンス。
- eventsとcommandsの2つのキューを持ち、event::Handler.processへevents消費を移譲ループする。

#### その他

- roll.rs: object.rsの形に整形する前のダイスロールモジュール。lib.rsの関連fnの収容・character.rsとの相互参照のモジュール化対応必要。
- ugrid.rs: Region operating functions with two (base and derived) Cartesian coordinate. It's under development now.
- temporal.rs: カレンダー機能に向けた時間表現モジュール。timestamp.rsに依存。開発途中。
