// This file includes untranslated text (ja).

# Canvas

html, cssとテンプレートjsで構成される描画要素の設計指針。

## html

htmlの作成規則

cssと作成作業を分離する。一部の開閉要素を特定タグで表すほかは、htmlの責務を、
静的に決定される列指向のグリッド要素による、要件の描画表現に限定する。

### ルール

- cssスタイリング作業を同時に行わない。css作成時は作成済みのhtmlを編集可能。
- html作成時の責務を2つに限定する。
    - 1. 要件から導出される、ディスプレイ表示するべきデータモデルを網羅し、ハードウェアの物理特性と人類の生物特性から、適切な最大インスタンスフィールド数を決定して、列指向のグリッドレイアウトに静的に配置場所を決定する。描画状況の取得が二度手間になるので、`display: grid`などの自動配置は利用を避ける。
    - 2. 要素に対し、適切なタグを選択することで、ブラウザなどの支援機能を受けやすくする。`hidden`,`.hidden`,`display:`等のレイアウトプロパティを定義・適用する。
- idはbody以降の親tagと、同層同tagの連番から機械的に決定する。
- 各element内の記述順は、tag名, id, html standard attribute, aria-label, class, class unique attribute。
- formatting rule:
    - Do not insert a line break before a closing tag.
    - Insert a line break before the start of every tag.

### タグの決定

- 視覚効果: `<main>`,`<dialog id="drawer">`,`<dialog id="modal">`,`<form>`,`<toast>` (いずれも、1画面でbody直下に単一要素とする)
- 集合要素: `<header>`,`<footer>`,`<section>`,`<article>`,`<fieldset>`,`<table>`
- 段落要素: `display: block`,
- 同列要素: `display: inline-block`, `display: inline`, `display: table-cell`

htmlの制約として、同列要素の中に段落要素を格納する、すなわち集合要素を持つべき時、タグを子に分離する必要がある。この時も、セマンティクスを最もよく表すタグを選択する。

## ブラウザアプリにおける基本構成

1つのドメインを扱うSPAは、単一のindex.htmlに表現できる。

```yaml
# htmlの基本構造
html:
  head:
  body:
    main:      # 主に閲覧機能
      header:
      section: # または、その他のsemantic tag。
      footer:
    drawer:    # 画面遷移時のメニュー表示。手動ではなくappが開閉する。<dialog id="drawer">
    modal:     # 編集機能・要アテンション時 <dialog id="modal"> showModal()
    form:      # <form id="form" method="dialog"></form> のみの1行要素
    toast:     # <output>: info~warningまでの重要度をポップアップ通知する。
```