// This file includes untranslated text (ja).

# Architecture

Gui application system for editing and reading structured data.

## Commands

```bash
# unit test
cargo test

# wasm-pack compile
wasm-pack build --target web --out-dir examples/app --out-name app
```

## dice-engine

1. dice-engine (D-Engine) は、CoC TRPG 7th Editionをプレイするために必要なデータ処理機能を集約した、webブラウザソフトウェアです。
2. 開発者本人のやる気が湧くCoC TRPG 7th Editionに特化している点、ダイスロールを出力するだけではない点に注意してください。
3. このソフトウェア開発は、現在停止中のwebカレンダー開発作業中に、時間軸を基盤にしたデータモデル編集ソフトウェアって、1つで良くない? と感じ、データモデル設計を汎用化するにあたって、TRPGは良い実践例になるという判断で行っています。
4. 3のデータモデル編集ソフトウェアとしての止揚と別に、TRPGに限らないRPGクリエイター向けに、データモデルの設計エンジン機能を展開することも検討しています。

### Requirement

1. CoC TRPG 7th Editionをオンラインで遊ぶ時、1: 背景やキャラクター画像の共有 2: 通話 3 チャットやキャラクターなど、数値とテキストデータの編集・共有閲覧 が必要。このうち1,2は運用コストがかかるので、既存サービスに任せ、先ずは3を網羅する。
2. 1の理由で、プレイヤー目線ではソフトウェアを1つ追加することになるので、利便性を損なわないことに注力する。1 ブラウザタブの増加を抑えるため、ブラウザの拡張機能として動作出来るようにする 2 既存サービスがHTMLを変更したら動かなくなるのは困るので、ユーザーがクリック・タッチでチャットボックスを指定できるようにする 3 既存サービス群からのインポート機能を順次追加する(イクスポートは(既存オンラインサービスが指定している点も踏まえて)jsonだけに留め、一々個別対応しない)。
3. 通信機能について。en圏では、プライバシーや独立性を重視して、OSSのセルフホストが人気である。ja圏では、便利さを重視してオンラインプラットフォームが人気である。デフォルトでP2P(webRTC, S)+フォールバック共通サーバー(STUN+TURN)+オプション任意ドメインとすれば、開発のコストも小さいし、en圏でキャッチーなので、最終的にこれを目指す。

### Function

1. キャラクターシート作成・表示

以降の全ての機能の前提。第一に、簡便に入力して保存できることを重視する。第二に、既存のキャラクターシートサービスくらいには良い感じの表示になるようにする。第三に、スマートフォンでも表示(・編集)に支障が無いようにUIを精査する。

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
  - ディスプレイに表示するのは、ルールブック準拠の言葉(label)であることを徹底する。プレイヤーの知らない実装都合の略称を作らない・使わない・表示しない。ラベルは、1つの変数の属性値(UTF-8)であり、言語(ja,en)別・略称等の引数を取って一意に決まる。
  - UI実装上の割り切りとして、入力欄の単位などの後置は排除する。単位は" ()""などでラベルに含めてinput外に前置することで、複雑性を抑える。


## System diagram

```
┌──────┐
│ user │
└──┬───┘
   ▼
┌────────────────────────┐
│ terminal (browser)     │
│┌──────────────────────┐│
││ canvas (html,css,js) ││
│└──────────────────────┘│
│┌──────────────────────┐│
││ app (rust as wasm)   ││
│└──────────────────────┘│
│┌──────────────────────┐│
││ opfs (wal)           ││
│└──────────────────────┘│
│┌──────────────────────┐│
││ pwa (service worker) ││
│└──────────────────────┘│
└────────────────────────┘
        ▲         ▲
 ┌──────┴──────┐  │ network functions:
 │ https proxy │  │ - realtime device-to-device
 └──────┬──────┘  │ - background data sync
        ▼         ▼
┌────────────────────────┐
│ fixture (linux)        │
│┌──────────────────────┐│
││ nginx (http)         ││
│└──────────────────────┘│
│┌──────────────────────┐│
││ signaling (stun,turn)││
│└──────────────────────┘│
│┌──────────────────────┐│
││ app (rust)           ││
│└──────────────────────┘│
│┌──────────────────────┐│
││ filesystem (wal)     ││
│└──────────────────────┘│
└────────────────────────┘
```

### Detail

#### Canvas

##### html

- index.htmlの1ファイル完結。
- FOUC防止のため、body atrributeにhiddenを書く。
- 初期表示しないelement以外、.hiddenクラスを追加しておく。
- text content: 一切変化しないテキストは書き込むが、言語切り替え必要なので、原則書かない。
- 動的に増えるelement: 最大数を決めて、-1,-2,...をidの末尾に付けてhtmlに書き込んでおく。
- divはmainの構成要素{header, div, footer}として定義する。汎用tagとしての利用を禁止する。
- semantic tagを使用する:
  - htmlにあるべき基本構造は定まっている。以下yamlを参照のこと。
  - 基本構造外のタグ決定の第一判断箇所は、「この要素は子の中で唯一か? そうでなければ縦積み(block)か横流し(inline)か?」
  - 子の中で唯一: <header>`,`<footer>`
  - 複数の変数を縦に並べる(block): `<p>`,`<section>`,`<article>`,`<header>`,`<footer>`,`<address>`, etc.
  - 同一行の中に複数変数を並べる(inline): `<span>`,`<time>`,`<a>`, etc.
- 開発者向けのコメントが不要になるように、全ての要素にaria-labelを付ける:
  - h1など1body1つのタグ・並列数の多い要素は省略可。
  - 命名は「その要素が何であるか」を単一の説明で表す。

```yaml
# htmlの基本構造
html:
  head:
  body:
    main:     # 主に閲覧機能
      header:
      div:    # または、特定のsemantic tag。
      footer:
    drawer:   # 画面遷移時のメニュー表示。手動UIではなくappが開閉する。<dialog id="drawer">
    modal:    # 編集機能・要アテンション時 <dialog id="modal"> showModal()
    form:     # <form id="form" method="dialog"></form> のみの1行要素
    toast:    # <output>
```


##### css

- config.css(変数定義), style.css, idや構造に依存のない外部css。
- style.cssにて[hidden], .hidden {display: none !important;} を定義する。
- 各セレクタはtagのパイプまたはaria-labelで指定する。classで指定しない。

##### javascript

- 要件に依らず、同内容の4ファイルで構成する:
  - htmlにmoduleとして呼ばれるinit.js
  - メインと非同期なスレッドでappを実行するためのworker.js
  - wasm-packで自動生成されるapp.js (app_bg.wasmを実行)
  - pwa用のservice workerを起動するsw.js
- init.js: workerに適宜eventをpostMessageで渡す。また、excute()でappからの指示を実行する。
- excute(operation: u8, element_id: str, attribute: str, value: str){}
  - Element.getElementId(element_id).textContent = value;
  - Element.getElementId(element_id).value = value;
  - Element.getElementId(element_id).toggleAttribute(attribute, value);
  - Element.getElementId(element_id).classList.add(value);
  - Element.getElementId(element_id).classList.remove(value);
  - Element.getElementId(element_id).openModal(); # modal専用
  - Element.getElementId(element_id).close();     # modal専用
  - applyClass(element_id, value); # rAFやsetTimeoutなど、非同期処理のみ

### App (browser)

- app.wasmとしてコンパイルする。

#### js_client.rs

- html, cssの内容を反映し、操作itemとfn(デバイス判定・ジェスチャー判定含む)を発行するモジュール。

#### store.rs

- ローカルストア(walとopfs)操作を発行するモジュール。
- ステートにメモリバッファとunsavedインデックスセットを持ち、メモリオンリーの操作関数setと、opfsへの反映関数saveを公開している。

### Roll

 - 開いた時点で一番目の選択肢にfocusを当てる。
 - 上下キー/tab/shift+tabでフォーカスが移動, enter(click, tap)で次へ

#### Dice Roll - ダイスロール (nDn + n)

選択後に表示されるべきインタラクティブUIは、出現順に
1. text[field](Roll::Field::DiceCount), +-ボタン(上下キーも同等に), 初期値1のnumber[1~100]入力欄(focusが当たったら直接入力とする。入力時のkeyboard enterで決定を発火), 「次へ」ボタン(enterも同等に)
2. text[field](Roll::Field::DiceSide), button[up] button[down], input[number(2(初期値),3,4,5,6,8,12,16,20,50,100)], button[next]
3. text[field](Roll::Field::「補正」の英単語), input[number(0(初期値), -100~100), button[submit]]
結果のState::Stack(roll: Roll)保持は不要。

Skill Roll — 技能値に対する基本判定
1. State::Character::Instance()に存在する技能を優先ソートしてセレクタとして表示。 text[field](skills: Instance::Fields(attribute: Schema::Attribute::Skill), button[up] button[down], button[next]
- 列指向で表示。1列にまとまる数で無い場合も多いので、画面幅に応じてflexに表示する
2. text[field](Roll::Field::「補正」の英単語), input[number(0(初期値), -100~100), button[submit]をinline表示
3. submitしたらApp::Roll::display()をしつつ結果をApp::Roll::stack(State::Stack(roll: SkillRoll))する。
    SkillRoll,

Characteristic Roll — 能力値判定 (幸運含む)
1. select[characteristic] を表示。nextボタンは無し
2. text[field](Roll::Field::「補正」の英単語), input[number(0(初期値), -100~100), button[submit]をinline表示
 - str~luck。Sanityは含まない (それは狂気判定)

Sanity Roll — 正気度喪失判定

Bout of Madness (Real Time) — 狂気の発作 (リアルタイム)
intを判定対象としてロール。regularまでの成功で「発狂」が判定結果。failure以下の場合は、「発狂しない」では微妙なので達成度を出して表す。
期間 (ラウンド) (1d10)も同時に実行してBoundOfMadnessResultに含む
regular以上(狂気の発作は)
BoutOfMadnessRealTime,
Bout of Madness (Summary) — 狂気の発作 (サマリー)
RealTimeとの違いは、label文字列と、期間の単位が「時間(hour)」なことだけ
BoutOfMadnessSummary,
Pushed Roll — 失敗後の再挑戦ロール
保持しているskill stack stateの中で、failure以下のものだけ候補化する。この時、新しい順にソートする
既にpush stackに紐づけがあるロールは候補から外すのが正確だが、複雑性が一気に増すので一旦省略。
PushedRoll,
Combined Skill Roll — 2技能を1ロールで同時判定
1. select[Skill]
2. select[Skill] って感じでrulebook通り2つ技能を選択したら実行で良いんだが、プレイヤーを観察していると、skill+characteristicの混合も需要あるので、一応メモ。
3. 出力は、[技能1 技能2] 実値1 実値2 出目 判定1(普通のSkill Rollと同様) 判定2。「部分的成功」みたいな組み合わせロール特有の用語は、rulebookに実は無いので、それは扱わない

Development Check - 上達チェック
- ボーナスダイスの無いregular以上のstackのあるskillを候補にする。
- ロールした結果、技能値を超過しているか、96~100の範囲であれば、上達する。1d10を追加で処理して、判定としては 上達 n という出力になる
- 通常の「失敗」「成功」という概念と違うので、Judge::{Developed,Undeveloped}を使う。labelは「上達」「上達なし」

## Specification (仕様)

### Limitation (制限事項)

- 各技能の専門分野(自由記入)の発行は最大4つ。

---