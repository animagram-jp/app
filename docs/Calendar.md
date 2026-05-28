// This file includes untranslated text (ja).

## Requirement

- webカレンダーとは、ユーザーが時系列上にメモを書く機能である
  - ユーザーが指定する時系列の単位は、数日・日・時分(10分刻みまで)・
- 携帯型でもノート型・据え置き型でも共通して、月次に時系列上データを見渡せる画面を提供する
  - メイン画面は5行x7列の35日分に固定し、上下スクロールで1週ずつの表示範囲更新を提供する
- オフラインでも同期以外の動作を提供し、ネットワークは端末間同期・ユーザー間共有を担う
- 画面は リリースノート(Github Pagesなど外部), ログイン, メイン, ドロワー, モーダルを用意する。
  - ユーザーのメールアドレスはfileに直で追加し、公開アカウント登録機能は持たない
- 入力データの最小単位には所有者ユーザーを紐づけ、所有者は更新しない。ユーザー間共有機能におけるview/permissionの判断はfixture及びterminalの両app層で行う
- iCalデータ(RFC5545)のインポート/エクスポート互換。
  - システム設計成立後に、RFC5545とシステム内の各フィールドに、1対1のマッピングを定義する。
- メイン画面...常設ボタン+閲覧, ドロワー...常設ボタンからネストした動作ボタンの展開部。開閉はユーザーが直で指示する必要が無い, モーダル...編集

## Concept

- event: 現実で起きた出来事。システム内では扱わない
- entry: システムがUI表示時に扱う、UIと1対1に対応したランタイムデータ
- resource: 電子計算機がentryを内部で解釈処理するためのストアデータ群
  - schedule: 複数のentryを作成するためのリソース
  - record: 1つのentryを作成するためのリソース。scheduleとの抵触時は、patchとして機能する
- state: プロセスがpattern群から特定のentryを構成するためのコンテクストデータ

## Todo


[html]
- main header: 前月/次月ボタン（‹ ›）、週スクロール（↑↓）、歯車ボタンを追加
- modal header: 歯車ボタンを追加、entry編集↔settings切り替えに使用
- modal fieldset: settings用fieldを追加（entry編集fieldは既存）
  - email/password: hidden→表示切り替えに変更
  - preference: locale, color(theme), youbi, datetime, main_scope
  - tag一覧 × 最大512件: name/type/parent/color/削除ボタン、追加ボタン

settings field仕様:
- locale: select — 0001 EN / 0010 JA
- color(theme): select — 001 light_01 / 101 dark_01
- youbi: select — 001 月〜111 日
- datetime: select — 001 YYYY-M-D h:m / 010 YYYY年M月D日 h時m分
- main_scope: radio — 001 year / 010 month / 011 day / 100 hour
- tag.name: input[text] — 32byte pool参照
- tag.type: radio — 001 color / 010 title / 011 free
- tag.parent: select — tag index（0=root）
- tag.color: input[color] — RGB 24bit

[system]
- FileSystemSyncAccessHandleでkill耐性を得るため、append-only log fileを作成し、handle.write(&entry_bytes);handle.flush();する。再起動時にsnap fileを作成し、logをクリアする
- エンドポイントはfixtureとterminalで対称な/sync/negotiateと/sync/transferを持つ。

```bash
POST /sync/negotiate
  req: { authorization, last_sync, requirements:{ own, extra } }
  res: { session_id, transfer_spec }

POST /sync/transfer
  req: { authorization, session_id, data }
  res: { ack, next_negotiate? }
```