// This file includes untranslated text (ja).

# CoC7th

Softwear for playing Call of Cthulhu 7th Edition

1. CoC TRPG 7th Editionをプレイするために必要な機能を集約した、webブラウザソフトウェアです。
2. 現在本repoに統合済みのwebカレンダー開発中に、時間軸を基盤にしたデータモデル編集ソフトウェアって、1つで良くない? と感じ、データモデル設計を汎用化するにあたって、TRPGは良い実践例になるという判断で行っています。
3. この他、インディーズRPGクリエイター向けに、キャラクターデータモデルの設計機能へ展開することも検討しています。

## Requirement

1. CoC TRPG 7th Editionをオンラインで遊ぶ時、1: 背景やキャラクター画像の共有 2: 通話 3 チャットやキャラクターなど、数値とテキストデータの編集・共有閲覧 が必要。このうち1,2は運用コストがかかるので、既存サービスに任せ、先ずは3を網羅する。
2. 1の理由で、プレイヤー目線ではソフトウェアを1つ追加することになるので、利便性を損なわないことに注力する。1 ブラウザタブの増加を抑えるため、ブラウザの拡張機能として動作出来るようにする 2 既存サービスがHTMLを変更したら動かなくなるのは困るので、ユーザーがクリック・タッチでチャットボックスを指定できるようにする 3 既存サービス群からのインポート機能を順次追加する(イクスポートは(既存オンラインサービスが指定している点も踏まえて)jsonだけに留め、一々個別対応しない)。
3. 通信機能について。en圏では、プライバシーや独立性を重視して、OSSのセルフホストが人気である。ja圏では、便利さを重視してオンラインプラットフォームが人気である。デフォルトでP2P(webRTC, S)+フォールバック共通サーバー(STUN+TURN)+オプション任意ドメインとすれば、開発のコストも小さいし、en圏でキャッチーなので、最終的にこれを目指す。

## Function

### 1. 自前html上でのキャラクターシート作成

以降の全ての機能の前提。

- 1. 簡便に入力して保存できることを重視する。
- 2. 既存のキャラクターシートサービスくらいには良い感じの表示になるようにする。
- 3. スマートフォンでも表示(・編集)に支障が無いようにUIを精査する。

### 2. チャットボックスでのコマンド実行・保持

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
  - 開いた時点で一番目の選択肢にfocusを当てる。
  - 上下キー/tab/shift+tabでフォーカスが移動, enter(click, tap)で次へ

#### 2-1. Dice Roll - ダイスロール (nDn + n)

- Calculate {count: i8} *Rand(1..{side: u8}) + {count: i8} *Rand(1..{side: u8}) + {modifier: i8}

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

#### 2-2. Sanity Roll — 正気度喪失判定

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

- 各技能の専門分野(自由記入)の発行は最大4つ。 // 要緩和
- 技能の完全自由欄も最大4つ。 // 要緩和

### キャラクターシート

#### 表示最大数

キャラクターシート1枚の中で、各フィールドの表示上限数。tsumugiサンプルを基準に。

| フィールド | たたき台 | 備考 |
|---|---|---|
| Skill エントリ数 | 60 | tsumugiは26技能。Art/Craft等の専門分化が重なると増える |
| Art/Craft等の専門分野サブスキル数 | 10 | tsumugiは7種 (Acting/Dance/Hanafuda/...) |
| Language (Other) のエントリ数 | 10 | tsumugiは1種 (English) |
| Fighting のエントリ数 | 8 | tsumugiは2種 (Brawl/Sword) |
| Firearms のエントリ数 | 8 | tsumugiは1種 (Handgun) |
| Pilot のエントリ数 | 6 | tsumugiは1種 (Boat) |
| Science のエントリ数 | 8 | tsumugiは1種 (Astronomy) |
| 完全自由技能欄のエントリ数 | 8 | 現Limitationでは4。緩和検討中 |
| Possessions エントリ数 | 30 | tsumugiは13点。武器・防具・その他混在 |
| Possessions 1件あたりのdamageダイスリスト数 | 4 | tsumugiは [[1,10,1],"DB"] で2要素 |
| SignificantPeople の人名リスト文字数 | 2000 | tsumugiは約1700文字。長大なリスト |
| EncountersWithStrangeEntities の文字数 | 3000 | tsumugiは最長フィールド。遭遇ログが蓄積する |
| ArcaneTomesAndSpells 呪文リスト数 | 40 | tsumugiは24種。セッション重ねると増える |
| Backstory 各テキストフィールド文字数 | 1000 | IdeologyAndBeliefsなど |
| PhobiasAndManias エントリ数 | 10 | tsumugiは5件 |

#### 保存最大数

アプリ全体・ユーザー単位での保存上限数。

| フィールド | たたき台 | 備考 |
|---|---|---|
| キャラクターシート総保存数 | 20 | 1ユーザーが複数キャラを管理する想定 |
| 1セッションあたりのキャラクター数 | 10 | PL人数 + NPC想定。KP運用含む |
| Roll::Stack の保持数 | 100 | 1セッション内のロール履歴。古いものから破棄 |
| SkillのDevelopment Check済みフラグ保持数 | 60 | Skill表示最大数と同値 |
| キャラクターのバックアップ(スナップショット)数 | 5 | セッション毎の成長前後比較用 |

---

## Note

This is a note by Andyou written through confirming rulebook.

### 先制の一撃 (奇襲)

- Striking the First Blow - (Surprise)
- 日本語訳版 第2刷 p.102

- 1. KPは対象が攻撃を予期できるか、技能ロールできる。
- 2. 予期できなかった場合、攻撃の技能ロールがファンブルしない限り成功する。
- 3. 奇襲は行動順を意志を持った順に修正する意図がある。つまり、同ラウンドに奇襲を受けた側が行動することは構わないと考えられる。

## Reference

- https://basicroleplaying.org/topic/8902-armor-rules-clarification-coc-7th-ed/