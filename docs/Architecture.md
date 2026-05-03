// This file includes untranslated text (ja).

# Architecture

## dice-engine 

1. dice-engine (D-Engine) は、CoC TRPG 7th Editionをプレイするために必要なデータ処理機能を集約した、webブラウザソフトウェアです。
2. 開発者本人のやる気が湧くCoC TRPG 7th Editionに特化している点、ダイスロールを出力するだけではない点が、「ダイスエンジン」という命名から湧く固定概念とズレがあるので注意してください。
3. このソフトウェア開発は、現在停止中のwebカレンダー開発作業中に、時間軸を基盤にしたデータモデル編集ソフトウェアって、1つで良くない? と感じ、データモデル設計を汎用化するにあたって、TRPGは良い実践例になるという判断で行っています。
4. 3のデータモデル編集ソフトウェアとしての止揚と別に、TRPGに限らないRPGクリエイター向けに、データモデルの設計エンジン機能を展開することも検討しています。

### Requirement

1. CoC TRPG 7th Editionをオンラインで遊ぶ時、1: 背景やキャラクター画像の共有 2: 通話 3 チャットやキャラクターなど、数値とテキストデータの編集・共有閲覧 が必要。このうち1,2は運用コストがかかるので、既存サービスに任せ、3を網羅する。
2. 1の理由で、プレイヤー目線ではソフトウェアを1つ追加することになるので、利便性を損なわないことに注力する。1 ブラウザタブの増加を抑えるため、ブラウザの拡張機能として動作出来るようにする 2 既存サービスがHTMLを変更したら動かなくなるのは困るので、ユーザーがクリック・タッチでチャットボックスを指定できるようにする 3 既存サービス群からのインポート機能を順次追加する(イクスポートは(既存オンラインサービスが指定している点も踏まえて)jsonだけに留め、一々個別対応しない)。
3. 通信機能について。en圏では、プライバシーや独立性を重視して、OSSのセルフホストが人気である。ja圏では、便利さを重視してオンラインプラットフォームが人気である。デフォP2P+フォールバック共通サーバー+オプション任意ドメインとすれば、開発のコストも小さいし、en圏でキャッチーなので、最終的にこれを目指す。

### Function

1. キャラクターシート作成・表示

以降の全ての機能の前提。第一に、簡便に入力して保存できることを重視する。第二に、既存のキャラクターシートサービスくらいには良い感じの表示になるようにする。第三に、スマートフォンでも表示(・編集)に支障が無いようにUIを精査する。現在スマートフォンだとテキストエリアフォーカス時にモーダルがなぜか開いて動かなくなるので、機会をみつけて直す。

2. チャットボックスでのコマンド実行・保持

予約語(デフォルトは"/")入力をトリガーとして、ダイスロール実行やデータ操作・集計を行う。
既存のサービスが、表示話者の入力DOMとテキストエリアを分けているので、複雑性が許せば話者のDOMも使う。テキストエリア単独でも、改行を使って簡便に表現は可能。
  - ロール
    - 予約語入力検知: 以下、「ダイスロール (nDn +n)」～「上達チェック」のセレクトボタンUIを重なり表示する。
      - テキストエリアの上辺を基準に表示物位置を決定する。mobileで高さが足りない場合は見切れないことを優先して上辺からの座標を正の値にする。
      - セレクタ外(esc)クリック(タッチ): 表示中の重なりUI非表示化、App::State::DisplayとInputを初期化する。テキストエリアの"/"は消去しない。
      - Roll::Resultはデータとして [ロール種アイテム, 判定対象アイテム, 小計値(単一orリスト), 判定結果アイテム]で構成する。
      - 出力テキストとして、 Roll::Result::display()-> "[{ロール種ラベル} {判定対象ラベル} = {目標値}(= からここまで、必要なロール種のみ。リストの場合は{}で囲う。)] {「出目」ラベル}:{出目の数値(重複の無い最終結果のみ)} {判定}: {判定結果のラベル}"で統一。
      - 後で集計するロール種のRoll::Resultのみ、App::State::Stackにスタックする。
      - 実装上の割り切りとして、入力欄の単位などの後置は排除する。単位は" ()""などでラベルに含めてinput外に前置することで、複雑性を抑える。
      - 同様に、インタラクティブUIの1->2->3で1つ前に戻る手段は用意しない。単純なのでescクリアで十分。
      - 必ずすべてのシーンで、appが「初期focus対象」を想定してそこにautofocusを設定しておく。
      - tabやshift+tabで操作可能なdomだけを適切にfocusできるようにする。
      - App::Rollがこれらのロール実行モジュールを担当する。よって、今table.rsにあるこれはapp.rsに移す。
    - ダイスロール (nDn + n)

### Script

以下、実装にあたっての具体的な手法規則

1. ディスプレイに表示するのは、ルールブック準拠の言葉(label)であることを徹底する。プレイヤーの知らない実装都合の略称を作らない・使わない・表示しない。ラベルは、1つの変数の属性値(UTF-8)であり、言語(ja,en)別・略称等の引数を取って一意に決まる。
2. UI実装上の割り切りとして、入力欄の単位などの後置は排除する。単位は" ()""などでラベルに含めてinput外に前置することで、複雑性を抑える。

### Module

システムのモジュール構成

#### Diagram

```
┌──────┐
│ user │
└──┬───┘
   ▼
┌────────────────────────┐
│ terminal (browser)     │
│┌──────────────────────┐│
││ canvas (html+css+js) ││
│└──────────────────────┘│
│┌──────────────────────┐│
││ app (Rust as Wasm)   ││
│└──────────────────────┘│
│┌──────────────────────┐│
││ opfs (local disk)    ││
│└──────────────────────┘│
│┌──────────────────────┐│
││ pwa (service worker) ││
│└──────────────────────┘│
└────────────────────────┘
        ▲         ▲
 ┌──────┴─────┐   │
 │ dns proxy  │   │
 └──────┬─────┘   │
        ▼         ▼
┌────────────────────────┐
│ fixture (cloudflare)   │
│┌─────────────────┐     │
││ WebRTC (STUN)   │     │
│└─────────────────┘     │
│┌─────────────────┐     │
││ websocket       │     │
│└─────────────────┘     │
│┌─────────────────┐     │
││ fs (local disk) │     │
│└─────────────────┘     │
└────────────────────────┘
```

#### Detail

##### Canvas

- html:
  - ファイル名はindex.html。/へのアクセス時に自動転送してくれるサービスが大半なので採用。特殊要件以外では単ファイル完結。
  - hidden/.hidden: FOUC防止のため、bodyにhiddenを書く。常時表示するelement以外、hiddenクラスを指定しておく。
  - text:   セッション中絶対に変化の無いテキストは書き込んでおくが、現代社会はja/en切り替えがページタイトルレベルで必要なので、該当はほぼ無い。また、それ以外はtextを書き込まない。
  - 動的に増えるelement: 最大数を決めて、1,2,...をidの末尾に付けてhtmlに書き込んでおく。
    - hidden: 初回時一斉にremoveAttribute("hidden")が起こるので、最初からcss .hiddenを付けるべきかも
  - 各elementはheader/div(またはsemantic tag)/footerで構成する。divはこの意味以外で使用禁止。
- css:
  - ファイル名はstyle.css。スタイリング用のアセットなので。外部参照ファイルは、挙動を依存しない範囲で適宜追加してよい。
  - hidden: .hidden {display: none !important;} を定義しておく。html hiddenが支配的なので、この時点で.hidden適用は不要。
  - セレクタはtagと列挙,idのみで指定する。classで指定しない。
- js:
  - htmlにmoduleとして呼ばれるinit.js, workerメモリを確保するためのworker.js, wasm-packで自動生成されるapp.js, pwa用のsw.js。
  - init.js: workerに適宜eventをpostMessageで渡す。また、excute()でAppからの指示を実行する。
  - excute(operation: u8, element_id: str, attribute: str, value: str|u64|i64|boolean){}
  - appが必要とする指示種は、以下の通り。
    - Element.getElementId(element_id).textContent = value;
    - Element.getElementId(element_id).value = value;
    - Element.getElementId(element_id).toggleAttribute(attribute, value);
    - Element.getElementId(element_id).classList.add(value);
    - Element.getElementId(element_id).classList.remove(value);
    - Element.getElementId(element_id).openModal(); # 08,09はmodal専用
    - Element.getElementId(element_id).close();
    - applyClass(element_id, value); # valueはclass name["show", "hide"] # rAFやsetTimeoutなど

##### Terminal app

- ファイル名: app.wasm
- app初期化時: 初期画面で必要なDOMにremoveAttribute("hidden")指示を出す。
- 以降
  - セッションライフタイム中に以降絶対に不要: addAttribute("hidden")
  - 表示(非表示)したい: classList.add/remove("hidden")

1. html

```yaml
html:
  head:
  body:
    main:
    drawer: # <dialog id="drawer"> set/removeAttribute("open")
    modal: # <dialog id="modal"> showModal()
    form:  # <form id="form" method="dialog"></form> のみの1行要素
    toast: # <output>
```

```html
<html>
  <head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <link rel="stylesheet" href="style.css">
    <script type="module" src="init.js"></script>
  </head>
  <body hidden>
    <main>
      <header></header>
      <div></div>
      <footer></footer>
    </main>
    <dialog id="modal"></dialog>
    <output id="output" role="status" aria-live="polite" aria-atomic="false">
      <article id="output_article1"><span id="output_article1_span"></span><p id="output_article1_p"></p></article>
      <article id="output_article2"><span id="output_article2_span"></span><p id="output_article2_p"></p></article>
    </output>
    <form id="form" method="dialog"></form>
  </body>
</html>
```

```css
.hidden {
  display: none !important;
}
output {
  position: fixed;
  bottom: 1.5rem;
  right: 1.5rem;
  display: flex;
  flex-direction: column-reverse; /* 新着が下、上方向にqueue */
  gap: .5rem;
  border: none; /* outputのブラウザデフォルトを上書き */
}
output article {
  display: flex;
  align-items: baseline;
  gap: .5rem;
  padding: .625rem .875rem;
  border-radius: .375rem;
  min-width: 14rem;
  max-width: 22rem;
  background: #1c1c1c;
  color: #f5f5f5;
  font-size: .875rem;
  line-height: 1.45;
  cursor: pointer;
  user-select: none;
  opacity: 0; /* enter前: 右にずれて透明 */
  translate: 1.5rem 0;
  transition: opacity .2s ease, translate .2s ease;
  pointer-events: none;
}
output article.show {
  opacity: 1;
  translate: 0 0;
  pointer-events: auto;
}
output article.hide {
  opacity: 0;
  translate: 1.5rem 0;
  transition-duration: .15s;
  pointer-events: none;
}
output article span { /* icon */
  font-weight: 700;
  font-size: .8125rem;
  flex-shrink: 0;
  padding: 0 .125rem;
  opacity: .7;
}
article.info    { background: var(--color-neutral-solid-gray-500); }
article.success { background: var(--color-semantic-success-1); }
article.warning { background: var(--color-semantic-warning-yellow-2); }
```

```js
// init.js
const worker = new Worker("./worker.js", { type: "module" })
worker.addEventListener("message", (e) => {
  const { type, payload } = e.data;
  if (type === "execute") { payload.forEach(execute); return; }
})
worker.addEventListener("error", (e) => {
  execute(4, "output_article1", "warning");
  execute(1, "output_article1_span", "!");
  execute(1, "output_article1_p", e.message);
  execute(8, "output_article1", "show");
  // worker.terminate(); worker = new Worker("worker.js");  // 再起動する場合
});
const execute = (operation, element_id, attribute = '', value) => {
  switch(operation) {
    const element = document.getElementById(element_id);
    if (!element) return;
    case 1: element.textContent = value; break
    case 2: element.value = value; break
    case 3: element.toggleAttribute(attribute, value); break
    case 4: element.classList.add(value); break
    case 5: element.classList.remove(value); break
    case 6: element.openModal(); break
    case 7: element.close(); break
    case 8: applyClass(element, value); break
  }
}
function applyClass = (element, value) => {
  switch(value) {
    case "show": 
      element.classList.remove("hide");
      requestAnimationFrame(() =>requestAnimationFrame(() => element.classList.add("show"))); break
    case "hide":
      element.classList.replace("show", "hide");
  }
}
function bind() {
  document.getElementById("form")?.addEventListener("submit", (e) => {
    e.preventDefault();
    worker.postMessage({ 
      type: "event",
      event_type: "submit", 
      target_id: "form", 
      value: Object.fromEntries(new FormData(e.target)),
    })
  })
  document.getElementById("input")?.addEventListener("input", (e) => {
    worker.postMessage({ 
      type: "event",
      event_type: "input", 
      target_id: e.target.id, 
      value: e.target.value,
    })
  })
  document.addEventListener("keydown", (e) => {
    if (["ArrowUp", "ArrowDown", "Enter", "Escape"].includes(e.key)) {
      e.preventDefault();
      worker.postMessage({ 
        type: "event",
        event_type: "keydown", 
        target_id: e.target.id, 
        value: e.key,
    })}
  })
  document.getElementById('char_edit_open')?.addEventListener('click', (e) => {
    e.stopPropagation();
    dispatch({ event_type: e.type, target_id: 'char_edit_open' });
  });
  document.addEventListener("click", (e) => {
    const element = e.target.closest('[id]');
    if (!element || ["button_open_modal"].includes(element.id)) return;
    worker.postMessage({ 
        type: "event",
        event_type: "click", 
        target_id: element.id, 
    })
  });
  document.addEventListener("focusin", (e) => {
    const id = e.target.id;
    if (id.startsWith('roll_') || id.startsWith('char_roll_') || id.startsWith('skill_roll_')) {
      worker.postMessage({ 
          type: "event",
          event_type: "focusin", 
          target_id: element.id, 
      })
    }
  });
}
worker.postMessage({ type: "init" });
worker.addEventListener("message", (e) => {
  if (e.data.type === "ready") bind();
}, { once: true });

## worker.js
```

2. 